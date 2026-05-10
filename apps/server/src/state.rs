use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use nyxbid_types::Market;

use crate::a2a::{PushNotificationStore, TaskStore};
use crate::indexer::{ChainEnvelope, IndexerMetrics};
use crate::solana::SolanaClient;
use crate::store::SharedStore;

/// Server-side application state. Reads come from the chain-indexed
/// [`SharedStore`]; writes happen only inside the indexer + state-apply
/// task. The `markets` list is the only piece of off-chain catalog data
/// the server still keeps in memory (until it moves to a config file or
/// DB in Phase 5/6).
pub struct AppState {
    /// Markets the UI offers in the create-intent form.
    pub markets: Vec<Market>,
    /// On-chain RPC handle. `None` when `SOLANA_RPC_URL` is unset; in
    /// that mode the server can still serve `/health` and the seeded
    /// market list but rejects every tx-prep route with a 503.
    pub solana: Option<SolanaClient>,
    /// Chain-indexed cache that powers `/api/intents`, `/api/fills`,
    /// `/api/dashboard`, and `/api/intents/{id}/quotes`.
    pub store: SharedStore,
    /// Decoded on-chain events fed by the log indexer. Subscribed to by
    /// the websocket route (Commit 8) and the state-apply task that
    /// keeps `store` warm.
    pub chain_tx: broadcast::Sender<ChainEnvelope>,
    /// Indexer health counters, surfaced via `/health`.
    pub indexer_metrics: Option<Arc<IndexerMetrics>>,
    /// In-memory A2A task registry. Backs `tasks/get` and
    /// `tasks/cancel` JSON-RPC methods. Cheap to clone (`Arc`).
    pub tasks: TaskStore,
    /// Per-task webhook configurations. Backs the
    /// `tasks/pushNotificationConfig/*` JSON-RPC methods and the
    /// task-state-change firing in [`crate::a2a::rpc`].
    pub push_notifications: PushNotificationStore,
}

/// Devnet USDC faucet mint. Fallback when no `SolanaClient` is
/// configured (offline mode); the active client's `usdc_mint` wins
/// whenever it's available so this constant only matters for the
/// `cargo run` -> /api/markets path with no `.env` set.
const DEVNET_USDC_MINT: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

/// Wrapped SOL native mint — same address on devnet and mainnet,
/// so this can stay hardcoded.
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

impl AppState {
    pub fn new(
        solana: Option<SolanaClient>,
        store: SharedStore,
        chain_tx: broadcast::Sender<ChainEnvelope>,
        indexer_metrics: Option<Arc<IndexerMetrics>>,
    ) -> Self {
        // Derive the SOL/USDC market from whatever USDC mint the
        // active SolanaClient is configured with. That way devnet
        // and mainnet always agree on `quote_mint` without anyone
        // having to remember to update a hardcoded list — the same
        // env var (`NYXBID_USDC_MINT`) drives the program's
        // owner-check, the wallet flow, and the catalog the UI
        // shows.
        let quote_mint = solana
            .as_ref()
            .map(|s| s.usdc_mint.to_string())
            .unwrap_or_else(|| DEVNET_USDC_MINT.to_string());

        Self {
            markets: vec![Market {
                symbol: "SOL/USDC".to_string(),
                base_mint: WSOL_MINT.to_string(),
                quote_mint,
                min_size: 100_000_000,
            }],
            solana,
            store,
            chain_tx,
            indexer_metrics,
            tasks: TaskStore::new(),
            push_notifications: PushNotificationStore::new(),
        }
    }
}

pub type SharedState = Arc<RwLock<AppState>>;
