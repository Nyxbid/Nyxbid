//! Borsh layouts for every Anchor account in the Nyxbid program.
//!
//! Field order, types, and sizes mirror
//! `chain/programs/nyxbid/src/state.rs`. Each struct implements
//! [`crate::AnchorAccount`] so callers can decode raw account data
//! (including the 8-byte discriminator prefix) with `try_decode`.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;

use crate::{discriminator, AnchorAccount};

/// Fixed-point scale used for `limit_price` and `revealed_price` (1e6).
pub const PRICE_SCALE: u64 = 1_000_000;

/// `Intent` account.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Intent {
    pub taker: Pubkey,
    pub side: u8,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: i64,
    pub resolve_deadline: i64,
    pub settle_deadline: i64,
    pub commitment_root: [u8; 32],
    pub status: u8,
    pub winning_quote: Pubkey,
    pub winning_price: u64,
    pub bump: u8,
    pub escrow_bump: u8,
}

impl AnchorAccount for Intent {
    const DISCRIMINATOR: [u8; 8] = discriminator::account::INTENT;
}

/// `Quote` account.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Quote {
    pub intent: Pubkey,
    pub maker: Pubkey,
    pub commitment: [u8; 32],
    pub revealed_price: u64,
    pub revealed_size: u64,
    pub nonce: [u8; 32],
    pub revealed: bool,
    pub maker_funded: bool,
    pub bump: u8,
}

impl AnchorAccount for Quote {
    const DISCRIMINATOR: [u8; 8] = discriminator::account::QUOTE;
}

/// `Escrow` account.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Escrow {
    pub intent: Pubkey,
    pub taker_amount: u64,
    pub taker_mint: Pubkey,
    pub maker: Pubkey,
    pub maker_amount: u64,
    pub maker_mint: Pubkey,
    pub settled: bool,
    pub bump: u8,
    pub taker_vault_bump: u8,
    pub maker_vault_bump: u8,
}

impl AnchorAccount for Escrow {
    const DISCRIMINATOR: [u8; 8] = discriminator::account::ESCROW;
}

/// Permanent on-chain fill record. Created on `settle` and never closed.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
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

impl AnchorAccount for Receipt {
    const DISCRIMINATOR: [u8; 8] = discriminator::account::RECEIPT;
}

/// Per-maker reputation counters.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Reputation {
    pub maker: Pubkey,
    pub quotes_submitted: u64,
    pub quotes_won: u64,
    pub settled_count: u64,
    pub failed_reveals: u64,
    pub bump: u8,
}

impl AnchorAccount for Reputation {
    const DISCRIMINATOR: [u8; 8] = discriminator::account::REPUTATION;
}

/// IntentStatus byte value, mirroring the on-chain `IntentStatus` enum.
pub mod intent_status {
    pub const OPEN: u8 = 0;
    pub const RESOLVED: u8 = 1;
    pub const SETTLED: u8 = 2;
    pub const CANCELLED: u8 = 3;
    pub const EXPIRED: u8 = 4;

    pub fn name(byte: u8) -> &'static str {
        match byte {
            OPEN => "open",
            RESOLVED => "resolved",
            SETTLED => "settled",
            CANCELLED => "cancelled",
            EXPIRED => "expired",
            _ => "unknown",
        }
    }
}
