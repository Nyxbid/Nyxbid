//! Shared helpers for LiteSVM-based program tests.
//!
//! Tests load the compiled `nyxbid.so` into a LiteSVM instance,
//! provision SPL token mints/accounts, and drive the lifecycle through
//! real serialized transactions \u2014 the same way a client would.
//!
//! NOTE: Phase 1 ships only the scaffolding here. End-to-end lifecycle
//! coverage lives in the TypeScript anchor tests under `chain/tests/`.
//! A follow-up branch will port the full happy-path + failure-path suite
//! to LiteSVM once the spl-token / solana-program-pack version split
//! between anchor-spl 1.0 and spl-token 8 is reconciled.

#![allow(dead_code)]

use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

pub use nyxbid::state::{
    ESCROW_SEED, INTENT_SEED, MAKER_VAULT_SEED, QUOTE_SEED, RECEIPT_SEED, REPUTATION_SEED,
    TAKER_VAULT_SEED,
};

pub fn program_id() -> Pubkey {
    nyxbid::ID
}

/// Boot a LiteSVM, install the compiled nyxbid program plus SPL Token,
/// and air-drop SOL to a list of payers.
pub fn boot_svm(payers: &[&Keypair]) -> LiteSVM {
    let mut svm = LiteSVM::new();

    let so = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/nyxbid.so"
    ))
    .expect("nyxbid.so not found - run `anchor build` first");
    svm.add_program(program_id(), &so).unwrap();

    let spl_token_so = include_bytes!("./fixtures/spl_token.so");
    svm.add_program(spl_token_id(), spl_token_so).unwrap();

    let spl_ata_so = include_bytes!("./fixtures/spl_associated_token_account.so");
    svm.add_program(spl_ata_id(), spl_ata_so).unwrap();

    for kp in payers {
        svm.airdrop(&kp.pubkey(), 1_000_000_000_000).unwrap();
    }

    svm
}

/// Hard-coded SPL Token program ID. Avoids pulling spl-token directly
/// while the dep matrix is sorted out.
pub fn spl_token_id() -> Pubkey {
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        .parse()
        .unwrap()
}

pub fn spl_ata_id() -> Pubkey {
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        .parse()
        .unwrap()
}

// --- PDA helpers -------------------------------------------------------

pub fn intent_pda(taker: &Pubkey, nonce: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[INTENT_SEED, taker.as_ref(), nonce.as_slice()],
        &program_id(),
    )
}

pub fn escrow_pda(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ESCROW_SEED, intent.as_ref()], &program_id())
}

pub fn taker_vault_pda(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TAKER_VAULT_SEED, intent.as_ref()], &program_id())
}

pub fn maker_vault_pda(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MAKER_VAULT_SEED, intent.as_ref()], &program_id())
}

pub fn quote_pda(intent: &Pubkey, maker: &Pubkey, nonce: &[u8; 16]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[QUOTE_SEED, intent.as_ref(), maker.as_ref(), nonce.as_slice()],
        &program_id(),
    )
}

pub fn receipt_pda(intent: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[RECEIPT_SEED, intent.as_ref()], &program_id())
}

pub fn reputation_pda(maker: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REPUTATION_SEED, maker.as_ref()], &program_id())
}
