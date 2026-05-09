//! Program-log indexer.
//!
//! Subscribes to `logsSubscribe` (Solana WebSocket RPC) filtered to the
//! Nyxbid program ID, decodes every Anchor `Program data: <base64>`
//! line, and pushes the typed event onto a `broadcast::Sender<ChainEvent>`
//! that the rest of the server (UI websocket, indexed state, etc.)
//! consumes.
//!
//! Why WebSocket over polling? Every devnet/mainnet RPC provider
//! exposes WS, push-based delivery hits the UI within one slot, and
//! the architecture stays close to push semantics so the Phase 4
//! Yellowstone-gRPC swap is mostly an interface change.
//!
//! The task auto-reconnects on disconnect with a small backoff so the
//! server keeps up after RPC restarts.

use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures_util::StreamExt;
use nyxbid_program as np;
use serde::{Deserialize, Serialize};
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_commitment_config::CommitmentConfig;
use tokio::sync::broadcast;

use crate::solana::SolanaClient;

/// Wraps a [`ChainEvent`] with the transaction context (signature +
/// slot) so downstream consumers can link an event back to the on-chain
/// transaction without re-fetching it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainEnvelope {
    /// Transaction signature, base58.
    pub signature: String,
    /// Slot the transaction landed in.
    pub slot: u64,
    /// Decoded event payload.
    pub event: ChainEvent,
}

/// Typed wire payload broadcast to every chain-event subscriber.
///
/// `kind` is the event name in `lower_snake_case`, the rest of the
/// fields are the event's borsh-decoded payload (Pubkeys serialise as
/// base58 thanks to `solana-pubkey`'s serde feature).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChainEvent {
    IntentCreated(np::events::IntentCreated),
    QuoteSubmitted(np::events::QuoteSubmitted),
    QuoteRevealed(np::events::QuoteRevealed),
    AuctionResolved(np::events::AuctionResolved),
    Settled(np::events::Settled),
    Cancelled(np::events::Cancelled),
}

impl ChainEvent {
    /// Try to decode a base64-decoded `Program data:` payload into one
    /// of the program's events. Returns `None` for unknown discriminators
    /// (other Anchor programs may share log channels in test validators).
    pub fn try_decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 8 {
            return None;
        }
        let disc = &bytes[..8];
        use np::AnchorEvent;
        if disc == np::discriminator::event::INTENT_CREATED {
            np::events::IntentCreated::try_decode(bytes)
                .ok()
                .map(ChainEvent::IntentCreated)
        } else if disc == np::discriminator::event::QUOTE_SUBMITTED {
            np::events::QuoteSubmitted::try_decode(bytes)
                .ok()
                .map(ChainEvent::QuoteSubmitted)
        } else if disc == np::discriminator::event::QUOTE_REVEALED {
            np::events::QuoteRevealed::try_decode(bytes)
                .ok()
                .map(ChainEvent::QuoteRevealed)
        } else if disc == np::discriminator::event::AUCTION_RESOLVED {
            np::events::AuctionResolved::try_decode(bytes)
                .ok()
                .map(ChainEvent::AuctionResolved)
        } else if disc == np::discriminator::event::SETTLED {
            np::events::Settled::try_decode(bytes)
                .ok()
                .map(ChainEvent::Settled)
        } else if disc == np::discriminator::event::CANCELLED {
            np::events::Cancelled::try_decode(bytes)
                .ok()
                .map(ChainEvent::Cancelled)
        } else {
            None
        }
    }
}

/// Lightweight metrics so operators can watch the indexer's health
/// without a full observability stack. Hooked up to /health later.
#[derive(Debug, Default)]
pub struct IndexerMetrics {
    pub events_decoded: AtomicU64,
    pub events_skipped: AtomicU64,
    pub failed_txs: AtomicU64,
    pub reconnects: AtomicU64,
}

impl IndexerMetrics {
    pub fn snapshot(&self) -> IndexerMetricsSnapshot {
        IndexerMetricsSnapshot {
            events_decoded: self.events_decoded.load(Ordering::Relaxed),
            events_skipped: self.events_skipped.load(Ordering::Relaxed),
            failed_txs: self.failed_txs.load(Ordering::Relaxed),
            reconnects: self.reconnects.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct IndexerMetricsSnapshot {
    pub events_decoded: u64,
    pub events_skipped: u64,
    pub failed_txs: u64,
    pub reconnects: u64,
}

/// Spawn the indexer on the current Tokio runtime.
///
/// Returns a handle to the metrics snapshot and the join handle. The
/// task runs forever (until the runtime drops), reconnecting on any
/// transport-level error with a 3s backoff.
pub fn spawn(
    sol: SolanaClient,
    chain_tx: broadcast::Sender<ChainEnvelope>,
) -> (Arc<IndexerMetrics>, tokio::task::JoinHandle<()>) {
    let metrics = Arc::new(IndexerMetrics::default());
    let m = Arc::clone(&metrics);
    let handle = tokio::spawn(async move {
        loop {
            match run_subscription(&sol, &chain_tx, &m).await {
                Ok(()) => tracing::warn!("log subscription ended cleanly; reconnecting"),
                Err(e) => {
                    tracing::error!(error = %e, "log subscription failed; reconnecting in 3s");
                }
            }
            m.reconnects.fetch_add(1, Ordering::Relaxed);
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    });
    (metrics, handle)
}

/// One subscription lifetime: connect, stream, return on disconnect/err.
async fn run_subscription(
    sol: &SolanaClient,
    chain_tx: &broadcast::Sender<ChainEnvelope>,
    metrics: &IndexerMetrics,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(ws_url = %sol.ws_url, program = %sol.program_id, "indexer connecting");
    let pubsub = PubsubClient::new(&sol.ws_url).await?;
    let filter = RpcTransactionLogsFilter::Mentions(vec![sol.program_id.to_string()]);
    let cfg = RpcTransactionLogsConfig {
        commitment: Some(CommitmentConfig::confirmed()),
    };
    let (mut stream, _unsub) = pubsub.logs_subscribe(filter, cfg).await?;
    tracing::info!("indexer connected; streaming program logs");

    while let Some(resp) = stream.next().await {
        let slot = resp.context.slot;
        let v = resp.value;
        if v.err.is_some() {
            metrics.failed_txs.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        for line in &v.logs {
            let Some(payload_b64) = line.strip_prefix("Program data: ") else {
                continue;
            };
            let bytes = match B64.decode(payload_b64.trim()) {
                Ok(b) => b,
                Err(e) => {
                    metrics.events_skipped.fetch_add(1, Ordering::Relaxed);
                    tracing::trace!(error = %e, "bad base64 in Program data");
                    continue;
                }
            };
            match ChainEvent::try_decode(&bytes) {
                Some(ev) => {
                    metrics.events_decoded.fetch_add(1, Ordering::Relaxed);
                    tracing::debug!(?ev, signature = %v.signature, slot, "chain event");
                    let envelope = ChainEnvelope {
                        signature: v.signature.clone(),
                        slot,
                        event: ev,
                    };
                    // .send() is a no-op if there are zero subscribers,
                    // and that's fine — events are inherently fire-and-forget.
                    let _ = chain_tx.send(envelope);
                }
                None => {
                    metrics.events_skipped.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
    Ok(())
}
