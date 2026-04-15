use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    Open,
    Resolved,
    Settled,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub id: String,
    pub taker: String,
    pub side: Side,
    pub base_mint: String,
    pub quote_mint: String,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: DateTime<Utc>,
    pub resolve_deadline: DateTime<Utc>,
    pub commitment_root: String,
    pub status: IntentStatus,
    pub winning_quote: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub id: String,
    pub intent_id: String,
    pub maker: String,
    pub commitment: String,
    pub revealed_price: Option<u64>,
    pub revealed_size: Option<u64>,
    pub revealed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub id: String,
    pub intent_id: String,
    pub taker: String,
    pub maker: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub size: u64,
    pub price: u64,
    pub tx_signature: Option<String>,
    pub settled_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub symbol: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub min_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub open_intents: u64,
    pub resolved_intents: u64,
    pub total_fills: u64,
    pub notional_24h: u64,
    pub avg_makers_per_intent: f64,
}
