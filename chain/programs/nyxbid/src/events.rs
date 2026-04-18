use anchor_lang::prelude::*;

#[event]
pub struct IntentCreated {
    pub intent: Pubkey,
    pub taker: Pubkey,
    pub side: u8,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: i64,
}

#[event]
pub struct QuoteSubmitted {
    pub intent: Pubkey,
    pub quote: Pubkey,
    pub maker: Pubkey,
}

#[event]
pub struct AuctionResolved {
    pub intent: Pubkey,
    pub winning_quote: Pubkey,
    pub clearing_price: u64,
    pub filled_size: u64,
}

#[event]
pub struct Settled {
    pub intent: Pubkey,
    pub receipt: Pubkey,
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub filled_price: u64,
    pub filled_size: u64,
}

#[event]
pub struct Cancelled {
    pub intent: Pubkey,
    pub reason: u8,
}
