use chrono::{Duration, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use nyxbid_types::{Intent, IntentStatus, Side};

#[derive(Debug, Deserialize)]
pub struct CreateIntentRequest {
    pub taker: String,
    pub side: Side,
    pub base_mint: String,
    pub quote_mint: String,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_seconds: Option<i64>,
    pub resolve_seconds: Option<i64>,
}

pub fn build_intent(req: CreateIntentRequest) -> Intent {
    let now = Utc::now();
    let reveal = now + Duration::seconds(req.reveal_seconds.unwrap_or(30));
    let resolve = reveal + Duration::seconds(req.resolve_seconds.unwrap_or(15));

    let mut h = Sha256::new();
    h.update(req.taker.as_bytes());
    h.update(req.base_mint.as_bytes());
    h.update(req.quote_mint.as_bytes());
    h.update(req.size.to_le_bytes());
    let commitment_root = hex::encode(h.finalize());

    Intent {
        id: format!("int_{}", Uuid::new_v4().simple()),
        taker: req.taker,
        side: req.side,
        base_mint: req.base_mint,
        quote_mint: req.quote_mint,
        size: req.size,
        limit_price: req.limit_price,
        reveal_deadline: reveal,
        resolve_deadline: resolve,
        commitment_root,
        status: IntentStatus::Open,
        winning_quote: None,
        created_at: now,
    }
}
