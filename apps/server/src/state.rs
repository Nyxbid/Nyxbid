use std::sync::Arc;

use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use nyxbid_types::{Fill, Intent, Market, Quote};

use crate::solana::SolanaClient;

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
    pub tx: broadcast::Sender<StreamEvent>,
}

impl AppState {
    pub fn seed(solana: Option<SolanaClient>, tx: broadcast::Sender<StreamEvent>) -> Self {
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
        }
    }
}

pub type SharedState = Arc<RwLock<AppState>>;
