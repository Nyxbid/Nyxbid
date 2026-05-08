//! Borsh layouts for every Anchor `#[event]` emitted by the Nyxbid program.
//!
//! Anchor encodes events as `[8-byte discriminator] || borsh(fields)`,
//! base64-encodes that payload, and writes it as a `Program data: <b64>`
//! line in the transaction's program logs.
//!
//! Each struct implements [`crate::AnchorEvent`] so the indexer can
//! decode a base64 payload with `try_decode` after stripping the
//! `Program data: ` prefix.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_pubkey::Pubkey;

use crate::{discriminator, AnchorEvent};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct IntentCreated {
    pub intent: Pubkey,
    pub taker: Pubkey,
    pub side: u8,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: i64,
}

impl AnchorEvent for IntentCreated {
    const DISCRIMINATOR: [u8; 8] = discriminator::event::INTENT_CREATED;
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct QuoteSubmitted {
    pub intent: Pubkey,
    pub quote: Pubkey,
    pub maker: Pubkey,
}

impl AnchorEvent for QuoteSubmitted {
    const DISCRIMINATOR: [u8; 8] = discriminator::event::QUOTE_SUBMITTED;
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct QuoteRevealed {
    pub intent: Pubkey,
    pub quote: Pubkey,
    pub maker: Pubkey,
    pub revealed_price: u64,
    pub revealed_size: u64,
    /// True if this reveal made the quote the current best bid.
    pub is_best: bool,
}

impl AnchorEvent for QuoteRevealed {
    const DISCRIMINATOR: [u8; 8] = discriminator::event::QUOTE_REVEALED;
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct AuctionResolved {
    pub intent: Pubkey,
    pub winning_quote: Pubkey,
    pub clearing_price: u64,
    pub filled_size: u64,
}

impl AnchorEvent for AuctionResolved {
    const DISCRIMINATOR: [u8; 8] = discriminator::event::AUCTION_RESOLVED;
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Settled {
    pub intent: Pubkey,
    pub receipt: Pubkey,
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub filled_price: u64,
    pub filled_size: u64,
}

impl AnchorEvent for Settled {
    const DISCRIMINATOR: [u8; 8] = discriminator::event::SETTLED;
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Cancelled {
    pub intent: Pubkey,
    pub reason: u8,
}

impl AnchorEvent for Cancelled {
    const DISCRIMINATOR: [u8; 8] = discriminator::event::CANCELLED;
}
