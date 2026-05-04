//! Smoke test: confirm the compiled program loads under LiteSVM and that
//! PDA derivation is deterministic. Lifecycle coverage lives in the
//! TypeScript anchor test suite (chain/tests/) for now.

mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn boots_svm_and_loads_program() {
    let payer = Keypair::new();
    let svm = boot_svm(&[&payer]);

    // Program account exists.
    let acct = svm.get_account(&program_id()).expect("program loaded");
    assert!(acct.executable);

    // SPL Token + ATA also loaded.
    assert!(svm.get_account(&spl_token_id()).is_some());
    assert!(svm.get_account(&spl_ata_id()).is_some());

    // Airdrop landed.
    assert!(svm.get_account(&payer.pubkey()).unwrap().lamports >= 1_000_000_000_000);
}

#[test]
fn pda_derivation_is_stable() {
    let taker = Keypair::new();
    let maker = Keypair::new();
    let nonce16 = [9u8; 16];

    let (intent_a, _) = intent_pda(&taker.pubkey(), &nonce16);
    let (intent_b, _) = intent_pda(&taker.pubkey(), &nonce16);
    assert_eq!(intent_a, intent_b);

    let (escrow, _) = escrow_pda(&intent_a);
    let (taker_vault, _) = taker_vault_pda(&intent_a);
    let (maker_vault, _) = maker_vault_pda(&intent_a);
    let (quote, _) = quote_pda(&intent_a, &maker.pubkey(), &nonce16);
    let (receipt, _) = receipt_pda(&intent_a);
    let (reputation, _) = reputation_pda(&maker.pubkey());

    // All derived PDAs are distinct.
    let pdas = [intent_a, escrow, taker_vault, maker_vault, quote, receipt, reputation];
    for (i, a) in pdas.iter().enumerate() {
        for b in pdas.iter().skip(i + 1) {
            assert_ne!(a, b, "two PDAs collide: {a} == {b}");
        }
    }
}
