//! Program-derived address helpers.
//!
//! All PDAs use the seeds defined in [`crate::seeds`] and the program ID
//! from [`crate::id::PROGRAM`]. Returning the bump alongside the address
//! lets callers cache it (the on-chain code already does for vault and
//! escrow PDAs).

use solana_pubkey::Pubkey;

use crate::{id, seeds};

/// `Intent` PDA. Seeds: `["intent", taker, nonce(16)]`.
pub fn intent(taker: &Pubkey, nonce: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::INTENT, taker.as_ref(), nonce],
        &id::PROGRAM,
    )
}

/// `Escrow` PDA. Seeds: `["escrow", intent]`.
pub fn escrow(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::ESCROW, intent.as_ref()], &id::PROGRAM)
}

/// `taker_vault` token account PDA. Seeds: `["taker_vault", intent]`.
pub fn taker_vault(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::TAKER_VAULT, intent.as_ref()],
        &id::PROGRAM,
    )
}

/// `maker_vault` token account PDA. Seeds: `["maker_vault", intent]`.
pub fn maker_vault(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::MAKER_VAULT, intent.as_ref()],
        &id::PROGRAM,
    )
}

/// `Quote` PDA. Seeds: `["quote", intent, maker, nonce(16)]`.
pub fn quote(intent: &Pubkey, maker: &Pubkey, nonce: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::QUOTE, intent.as_ref(), maker.as_ref(), nonce],
        &id::PROGRAM,
    )
}

/// `Receipt` PDA (permanent fill record). Seeds: `["receipt", intent]`.
pub fn receipt(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::RECEIPT, intent.as_ref()], &id::PROGRAM)
}

/// `Reputation` PDA, keyed by maker. Seeds: `["reputation", maker]`.
pub fn reputation(maker: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::REPUTATION, maker.as_ref()],
        &id::PROGRAM,
    )
}

/// SPL Associated Token Account derivation: `[owner, token_program, mint]`
/// against the associated-token program ID. Mirrors `spl-associated-token-account`
/// without taking the dependency.
pub fn associated_token(owner: &Pubkey, mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[owner.as_ref(), id::TOKEN.as_ref(), mint.as_ref()],
        &id::ASSOCIATED_TOKEN,
    )
}
