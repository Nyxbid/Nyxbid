use anchor_lang::prelude::*;

pub const INTENT_SEED: &[u8] = b"intent";
pub const QUOTE_SEED: &[u8] = b"quote";
pub const ESCROW_SEED: &[u8] = b"escrow";
pub const RECEIPT_SEED: &[u8] = b"receipt";

#[account]
pub struct Intent {
    pub taker: Pubkey,
    pub side: u8,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: i64,
    pub resolve_deadline: i64,
    pub commitment_root: [u8; 32],
    pub status: u8,
    pub winning_quote: Pubkey,
    pub bump: u8,
}

impl Intent {
    pub const LEN: usize = 8 + 32 + 1 + 32 + 32 + 8 + 8 + 8 + 8 + 32 + 1 + 32 + 1;
}

#[account]
pub struct Quote {
    pub intent: Pubkey,
    pub maker: Pubkey,
    pub commitment: [u8; 32],
    pub revealed_price: u64,
    pub revealed_size: u64,
    pub nonce: [u8; 32],
    pub revealed: bool,
    pub bump: u8,
}

impl Quote {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 8 + 8 + 32 + 1 + 1;
}

#[account]
pub struct Escrow {
    pub intent: Pubkey,
    pub taker_deposit: u64,
    pub maker: Pubkey,
    pub maker_deposit: u64,
    pub settled: bool,
    pub bump: u8,
}

impl Escrow {
    pub const LEN: usize = 8 + 32 + 8 + 32 + 8 + 1 + 1;
}

#[account]
pub struct Receipt {
    pub intent: Pubkey,
    pub taker: Pubkey,
    pub maker: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub filled_size: u64,
    pub filled_price: u64,
    pub settled_at: i64,
    pub bump: u8,
}

impl Receipt {
    pub const LEN: usize = 8 + 32 + 32 + 32 + 32 + 32 + 8 + 8 + 8 + 1;
}

#[repr(u8)]
pub enum IntentStatus {
    Open = 0,
    Resolved = 1,
    Settled = 2,
    Cancelled = 3,
    Expired = 4,
}

#[repr(u8)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}
