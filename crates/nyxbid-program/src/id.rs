//! Program IDs the Nyxbid program (and our tx builder) reference.
//!
//! `PROGRAM` must stay in sync with `declare_id!` in
//! `chain/programs/nyxbid/src/lib.rs` and `programs.*` in
//! `chain/Anchor.toml`.

use solana_pubkey::Pubkey;

/// The Nyxbid Anchor program.
pub const PROGRAM: Pubkey =
    Pubkey::from_str_const("nyxkGtm8x7GMdTWKyy5TKa72pgsebrECrchPDuRSrEQ");

/// SPL Token program (classic, not Token-2022).
pub const TOKEN: Pubkey =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// SPL Associated Token Account program.
pub const ASSOCIATED_TOKEN: Pubkey =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// System program.
pub const SYSTEM: Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");

/// Sysvar: rent. `create_intent` requires this account.
pub const SYSVAR_RENT: Pubkey =
    Pubkey::from_str_const("SysvarRent111111111111111111111111111111111");

/// Native SOL wrapped as an SPL token (Wrapped SOL / WSOL). The token
/// program treats this mint specially: lamports inside the token
/// account are the WSOL balance and `SyncNative` reconciles them.
pub const NATIVE_MINT: Pubkey =
    Pubkey::from_str_const("So11111111111111111111111111111111111111112");
