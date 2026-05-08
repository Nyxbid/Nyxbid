//! Borsh-serialisable instruction parameter structs.
//!
//! Field order and types must exactly match the on-chain
//! `#[derive(AnchorSerialize, AnchorDeserialize)]` structs in
//! `chain/programs/nyxbid/src/instructions/*.rs`.
//!
//! Anchor's wire format is `[8-byte discriminator] || borsh(params)`,
//! where `params` is the single `params: T` argument to each handler.
//! Use [`encode_ix_data`] to assemble the full instruction `data` field.

use borsh::{BorshDeserialize, BorshSerialize};

/// `Side` discriminator used in `CreateIntentParams::side`.
pub mod side {
    pub const BUY: u8 = 0;
    pub const SELL: u8 = 1;
}

/// Parameters for `create_intent`.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct CreateIntentParams {
    pub side: u8,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: i64,
    pub resolve_deadline: i64,
    pub settle_deadline: i64,
    pub commitment_root: [u8; 32],
    /// 16 random bytes; used as a PDA seed so the same taker can have
    /// many simultaneous open intents.
    pub nonce: [u8; 16],
}

/// Parameters for `submit_quote`.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct SubmitQuoteParams {
    pub commitment: [u8; 32],
    /// 16 random bytes; used as a PDA seed so the same maker can post
    /// multiple quotes for distinct nonces (typically just one per intent).
    pub nonce: [u8; 16],
}

/// Parameters for `reveal_quote`.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct RevealQuoteParams {
    pub revealed_price: u64,
    pub revealed_size: u64,
    /// 32-byte secret nonce used in the original sha256 commitment.
    pub nonce: [u8; 32],
}

/// Parameters for `fund_maker_escrow`.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct FundMakerEscrowParams {
    /// Amount of `maker_lock_mint` to lock; must equal the notional
    /// implied by `intent.winning_price` and `intent.size`.
    pub amount: u64,
}

/// Concatenate an 8-byte instruction discriminator with the borsh
/// encoding of its params. Returns the full `Instruction::data` payload.
pub fn encode_ix_data<T: BorshSerialize>(
    discriminator: [u8; 8],
    params: &T,
) -> Result<Vec<u8>, std::io::Error> {
    let mut buf = Vec::with_capacity(8 + 64);
    buf.extend_from_slice(&discriminator);
    params.serialize(&mut buf)?;
    Ok(buf)
}

/// Convenience for the no-argument instructions (`settle`, `cancel`,
/// `expire_with_maker`, `expire_no_maker`).
pub fn encode_empty_ix_data(discriminator: [u8; 8]) -> Vec<u8> {
    discriminator.to_vec()
}
