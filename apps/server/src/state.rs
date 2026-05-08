use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use nyxbid_types::{Fill, Intent, Market, Quote};

use crate::indexer::{ChainEvent, IndexerMetrics};
use crate::solana::SolanaClient;

/// Legacy DTO-shaped events used by the existing SSE route. Kept alive
/// for backwards-compat while Phase 2 swaps to chain-driven state in
/// the next commit; once the UI is fully on `chain_tx`, this enum
/// (and the seeded `Vec`s below) goes away.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    IntentCreated(Intent),
    QuoteSubmitted(Quote),
    AuctionResolved { intent_id: String },
    Filled(Fill),
}

pub struct AppState {
    pub intents: Vec<Intent>,
    pub quotes: Vec<Quote>,
    pub fills: Vec<Fill>,
    pub markets: Vec<Market>,
    pub solana: Option<SolanaClient>,
    /// Legacy in-memory broadcast (DTO shapes). Going away in commit 7.
    pub tx: broadcast::Sender<StreamEvent>,
    /// Decoded on-chain events, fed by the log indexer. Live as soon as
    /// `SOLANA_RPC_URL` is configured; the seeded REST routes ignore
    /// this for now.
    pub chain_tx: broadcast::Sender<ChainEvent>,
    /// Indexer health counters; surfaced via /health and useful for
    /// "is the server seeing chain activity at all?" debugging.
    pub indexer_metrics: Option<Arc<IndexerMetrics>>,
}

impl AppState {
    pub fn seed(
        solana: Option<SolanaClient>,
        tx: broadcast::Sender<StreamEvent>,
        chain_tx: broadcast::Sender<ChainEvent>,
        indexer_metrics: Option<Arc<IndexerMetrics>>,
    ) -> Self {
        Self {
            intents: vec![],
            quotes: vec![],
            fills: vec![],
            markets: vec![Market {
                symbol: "SOL/USDC".to_string(),
                base_mint: "So11111111111111111111111111111111111111112".to_string(),
                quote_mint: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                min_size: 100_000_000,
            }],
            solana,
            tx,
            chain_tx,
            indexer_metrics,
        }
    }
}

pub type SharedState = Arc<RwLock<AppState>>;
