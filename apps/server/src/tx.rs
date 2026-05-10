//! Unsigned transaction builders for the Nyxbid program.
//!
//! Each builder assembles a legacy `solana_transaction::Transaction`
//! with the correct accounts, discriminator, and Borsh-encoded params,
//! attaches the latest blockhash, and returns the bincode-then-base64
//! representation that `@solana/web3.js`, Phantom, Solflare, and
//! `solana-cli` all accept directly.
//!
//! The server **never signs**. The caller (browser wallet or maker bot)
//! signs, then either pushes back to `/api/tx/send` or broadcasts on its
//! own. The builder also returns the deterministic PDAs it derived so
//! the client can render an optimistic UI before the tx lands.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use nyxbid_program as np;
use serde::{Deserialize, Serialize};
use solana_instruction::{AccountMeta, Instruction};
use solana_message::Message;
use solana_pubkey::Pubkey;
use solana_transaction::Transaction;

use crate::solana::{SolanaClient, SolanaError};

/// Errors raised by the tx-prep layer. Distinct from [`SolanaError`] so
/// the route layer can map "user gave us garbage" to 4xx and "RPC blew
/// up" to 5xx.
#[derive(Debug, thiserror::Error)]
pub enum TxBuildError {
    #[error("invalid pubkey for {field}: {error}")]
    BadPubkey { field: &'static str, error: String },
    #[error("invalid hex for {field}: {error}")]
    BadHex { field: &'static str, error: String },
    #[error("{field} must be exactly {expected} bytes, got {got}")]
    WrongLength {
        field: &'static str,
        expected: usize,
        got: usize,
    },
    #[error("invalid side; expected \"buy\" or \"sell\"")]
    BadSide,
    #[error("size and limit_price must be non-zero")]
    ZeroValue,
    #[error("deadlines must be reveal < resolve < settle")]
    BadDeadlines,
    #[error("borsh: {0}")]
    Borsh(#[from] std::io::Error),
    #[error("bincode: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("solana: {0}")]
    Solana(#[from] SolanaError),
}

/// Wire-level response shared by every tx-prep endpoint.
#[derive(Clone, Debug, Serialize)]
pub struct PreparedTx {
    /// Bincode-serialised, base64-encoded legacy `Transaction` ready
    /// for any wallet's `signTransaction(Transaction.from(base64))`.
    pub tx_base64: String,
    /// Just the `Message` (what wallets actually sign). Useful for
    /// hardware wallets that only accept message bytes.
    pub message_base64: String,
    /// Recent blockhash baked into the message, base58-encoded.
    pub blockhash: String,
    /// Last block height for which this blockhash is valid. Clients
    /// should re-prepare the tx if the user takes longer than this to
    /// sign.
    pub last_valid_block_height: u64,
    /// Fee payer (always equals the taker / maker / relayer who is
    /// expected to sign).
    pub fee_payer: String,
    /// PDAs the instruction will create or touch. Predictable so the UI
    /// can subscribe to events for these accounts before the tx lands.
    pub accounts: PreparedAccounts,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct PreparedAccounts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escrow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taker_vault: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maker_vault: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reputation: Option<String>,
}

/// `POST /api/tx/create_intent` body.
///
/// All pubkeys are base58. Hex fields use lowercase, no `0x` prefix. The
/// `nonce` is supplied by the client so the client can pre-compute the
/// `Intent` PDA (see `nyxbid_program::pda::intent`).
#[derive(Clone, Debug, Deserialize)]
pub struct CreateIntentRequest {
    pub taker: String,
    pub side: SideRequest,
    pub base_mint: String,
    pub quote_mint: String,
    pub size: u64,
    pub limit_price: u64,
    pub reveal_deadline: i64,
    pub resolve_deadline: i64,
    pub settle_deadline: i64,
    /// 32-byte hex. Optional; defaults to all-zero. Reserved for future
    /// merkle-of-eligible-makers; the chain currently stores it as-is.
    #[serde(default)]
    pub commitment_root_hex: Option<String>,
    /// 16-byte hex. Required: doubles as the `Intent` PDA seed so the
    /// client can address its own intents.
    pub nonce_hex: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SideRequest {
    Buy,
    Sell,
}

impl SideRequest {
    fn as_byte(self) -> u8 {
        match self {
            SideRequest::Buy => np::params::side::BUY,
            SideRequest::Sell => np::params::side::SELL,
        }
    }
}

/// Build an unsigned `create_intent` transaction.
///
/// On success, the returned `PreparedTx` already contains the recent
/// blockhash and the fee_payer is set to `taker`. The wallet only needs
/// to sign and broadcast.
pub async fn build_create_intent(
    sol: &SolanaClient,
    req: CreateIntentRequest,
) -> Result<PreparedTx, TxBuildError> {
    // -- parse + validate inputs -------------------------------------
    let taker = parse_pk("taker", &req.taker)?;
    let base_mint = parse_pk("base_mint", &req.base_mint)?;
    let quote_mint = parse_pk("quote_mint", &req.quote_mint)?;

    if req.size == 0 || req.limit_price == 0 {
        return Err(TxBuildError::ZeroValue);
    }
    if !(req.reveal_deadline < req.resolve_deadline
        && req.resolve_deadline < req.settle_deadline)
    {
        return Err(TxBuildError::BadDeadlines);
    }

    let nonce = parse_fixed_hex::<16>("nonce_hex", &req.nonce_hex)?;
    let commitment_root = match req.commitment_root_hex.as_deref() {
        Some(s) => parse_fixed_hex::<32>("commitment_root_hex", s)?,
        None => [0u8; 32],
    };

    // -- derive PDAs / mint choice -----------------------------------
    let (intent_pda, _) = np::pda::intent(&taker, &nonce);
    let (escrow_pda, _) = np::pda::escrow(&intent_pda);
    let (taker_vault_pda, _) = np::pda::taker_vault(&intent_pda);

    // Mirror chain logic: buys lock quote_mint up to size*limit_price,
    // sells lock size of base_mint.
    let lock_mint = match req.side {
        SideRequest::Buy => quote_mint,
        SideRequest::Sell => base_mint,
    };
    let (taker_source_ata, _) = np::pda::associated_token(&taker, &lock_mint);

    // -- assemble Anchor instruction ---------------------------------
    let params = np::params::CreateIntentParams {
        side: req.side.as_byte(),
        size: req.size,
        limit_price: req.limit_price,
        reveal_deadline: req.reveal_deadline,
        resolve_deadline: req.resolve_deadline,
        settle_deadline: req.settle_deadline,
        commitment_root,
        nonce,
    };
    let data = np::params::encode_ix_data(np::discriminator::ix::CREATE_INTENT, &params)?;

    // Account order MUST match `CreateIntent` in
    // chain/programs/nyxbid/src/instructions/create_intent.rs.
    let metas = vec![
        AccountMeta::new(taker, true),                       // taker (signer, mut)
        AccountMeta::new_readonly(base_mint, false),         // base_mint
        AccountMeta::new_readonly(quote_mint, false),        // quote_mint
        AccountMeta::new(taker_source_ata, false),           // taker_source (mut)
        AccountMeta::new(intent_pda, false),                 // intent (init)
        AccountMeta::new(escrow_pda, false),                 // escrow (init)
        AccountMeta::new(taker_vault_pda, false),            // taker_vault (init)
        AccountMeta::new_readonly(lock_mint, false),         // taker_lock_mint
        AccountMeta::new_readonly(np::id::TOKEN, false),     // token_program
        AccountMeta::new_readonly(np::id::SYSTEM, false),    // system_program
        AccountMeta::new_readonly(np::id::SYSVAR_RENT, false), // rent
    ];

    let create_intent_ix = Instruction {
        program_id: np::id::PROGRAM,
        accounts: metas,
        data,
    };

    // Prepend an idempotent ATA-create for `taker_source`. Without
    // this, a wallet that has never held `lock_mint` hits the chain
    // with a non-existent account and the program rejects with Anchor
    // 3012 (`AccountNotInitialized`). `create_idempotent` no-ops if
    // the account already exists, so it's free for repeat traders.
    let ata_ix = create_idempotent_ata_ix(&taker, &taker_source_ata, &taker, &lock_mint);

    // If the locked mint is native SOL (WSOL), nobody actually holds
    // WSOL until they wrap it. Rather than ask the user to wrap by
    // hand we top up the freshly-created ATA with native lamports and
    // call `SyncNative`, all inside the same transaction. Result: the
    // user just signs once and Nyxbid takes care of the SPL plumbing.
    let wrap_ixs = if lock_mint == np::id::NATIVE_MINT {
        let lock_amount = match req.side {
            SideRequest::Buy => quote_notional(req.size, req.limit_price)
                .ok_or(TxBuildError::ZeroValue)?,
            SideRequest::Sell => req.size,
        };
        if lock_amount == 0 {
            return Err(TxBuildError::ZeroValue);
        }
        vec![
            system_transfer_ix(&taker, &taker_source_ata, lock_amount),
            sync_native_ix(&taker_source_ata),
        ]
    } else {
        Vec::new()
    };

    let mut ixs = Vec::with_capacity(2 + wrap_ixs.len());
    ixs.push(ata_ix);
    ixs.extend(wrap_ixs);
    ixs.push(create_intent_ix);

    finalize_tx(
        sol,
        &taker,
        ixs,
        PreparedAccounts {
            intent: Some(intent_pda.to_string()),
            escrow: Some(escrow_pda.to_string()),
            taker_vault: Some(taker_vault_pda.to_string()),
            ..Default::default()
        },
    )
    .await
}

/// Mirror of `nyxbid_program`'s on-chain `quote_notional`: fixed-point
/// `size * price / PRICE_SCALE` in u128 to avoid overflow.
fn quote_notional(size: u64, price: u64) -> Option<u64> {
    let n = (size as u128).checked_mul(price as u128)?;
    let n = n.checked_div(np::state::PRICE_SCALE as u128)?;
    u64::try_from(n).ok()
}

/// System program `Transfer` instruction (variant 2). We use this to
/// fund the user's WSOL token account with native lamports before
/// `SyncNative` reconciles the balance into a real token amount.
fn system_transfer_ix(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
    let mut data = Vec::with_capacity(12);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&lamports.to_le_bytes());
    Instruction {
        program_id: np::id::SYSTEM,
        accounts: vec![
            AccountMeta::new(*from, true),
            AccountMeta::new(*to, false),
        ],
        data,
    }
}

/// SPL Token `SyncNative` instruction (variant 17). Updates a WSOL
/// token account's balance to match the native lamports held by it.
fn sync_native_ix(account: &Pubkey) -> Instruction {
    Instruction {
        program_id: np::id::TOKEN,
        accounts: vec![AccountMeta::new(*account, false)],
        data: vec![17u8],
    }
}

/// SPL Associated Token Account program — `CreateIdempotent`
/// instruction (variant tag `1`). Creates the ATA if missing,
/// no-ops if it already exists. Always cheap to include in front of
/// any instruction that touches an ATA the user may not have funded
/// yet. Account order is fixed by the SPL ATA program; do not
/// reorder.
fn create_idempotent_ata_ix(
    payer: &Pubkey,
    ata: &Pubkey,
    wallet: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: np::id::ASSOCIATED_TOKEN,
        accounts: vec![
            AccountMeta::new(*payer, true),                   // funding (signer, mut)
            AccountMeta::new(*ata, false),                    // ata (mut)
            AccountMeta::new_readonly(*wallet, false),        // wallet
            AccountMeta::new_readonly(*mint, false),          // mint
            AccountMeta::new_readonly(np::id::SYSTEM, false), // system_program
            AccountMeta::new_readonly(np::id::TOKEN, false),  // token_program
        ],
        data: vec![1u8],
    }
}

// ---- submit_quote --------------------------------------------------

/// `POST /api/tx/submit_quote` body.
#[derive(Clone, Debug, Deserialize)]
pub struct SubmitQuoteRequest {
    pub maker: String,
    /// `Intent` PDA, base58.
    pub intent: String,
    /// 32-byte hex sha256(price_le || size_le || nonce32).
    pub commitment_hex: String,
    /// 16-byte hex used as the `Quote` PDA seed. Client picks this; it
    /// only needs to be unique per (intent, maker).
    pub nonce_hex: String,
}

pub async fn build_submit_quote(
    sol: &SolanaClient,
    req: SubmitQuoteRequest,
) -> Result<PreparedTx, TxBuildError> {
    let maker = parse_pk("maker", &req.maker)?;
    let intent = parse_pk("intent", &req.intent)?;
    let commitment = parse_fixed_hex::<32>("commitment_hex", &req.commitment_hex)?;
    let nonce = parse_fixed_hex::<16>("nonce_hex", &req.nonce_hex)?;

    let (quote_pda, _) = np::pda::quote(&intent, &maker, &nonce);
    let (reputation_pda, _) = np::pda::reputation(&maker);

    let params = np::params::SubmitQuoteParams { commitment, nonce };
    let data = np::params::encode_ix_data(np::discriminator::ix::SUBMIT_QUOTE, &params)?;

    // Account order MUST match `SubmitQuote` in
    // chain/programs/nyxbid/src/instructions/submit_quote.rs.
    let metas = vec![
        AccountMeta::new(maker, true),                    // maker (signer, mut)
        AccountMeta::new(intent, false),                  // intent (mut)
        AccountMeta::new(quote_pda, false),               // quote (init)
        AccountMeta::new(reputation_pda, false),          // reputation (init_if_needed)
        AccountMeta::new_readonly(np::id::SYSTEM, false), // system_program
    ];

    let prepared = finalize_tx(
        sol,
        &maker,
        vec![Instruction {
            program_id: np::id::PROGRAM,
            accounts: metas,
            data,
        }],
        PreparedAccounts {
            quote: Some(quote_pda.to_string()),
            reputation: Some(reputation_pda.to_string()),
            ..Default::default()
        },
    )
    .await?;
    Ok(prepared)
}

// ---- reveal_quote --------------------------------------------------

/// `POST /api/tx/reveal_quote` body.
#[derive(Clone, Debug, Deserialize)]
pub struct RevealQuoteRequest {
    pub maker: String,
    /// `Intent` PDA, base58.
    pub intent: String,
    /// `Quote` PDA, base58 (returned by /api/tx/submit_quote).
    pub quote: String,
    pub revealed_price: u64,
    pub revealed_size: u64,
    /// 32-byte hex secret used in the original sha256 commitment.
    pub commit_nonce_hex: String,
}

pub async fn build_reveal_quote(
    sol: &SolanaClient,
    req: RevealQuoteRequest,
) -> Result<PreparedTx, TxBuildError> {
    if req.revealed_price == 0 || req.revealed_size == 0 {
        return Err(TxBuildError::ZeroValue);
    }
    let maker = parse_pk("maker", &req.maker)?;
    let intent = parse_pk("intent", &req.intent)?;
    let quote = parse_pk("quote", &req.quote)?;
    let nonce = parse_fixed_hex::<32>("commit_nonce_hex", &req.commit_nonce_hex)?;

    let params = np::params::RevealQuoteParams {
        revealed_price: req.revealed_price,
        revealed_size: req.revealed_size,
        nonce,
    };
    let data = np::params::encode_ix_data(np::discriminator::ix::REVEAL_QUOTE, &params)?;

    // Account order MUST match `RevealQuote` in
    // chain/programs/nyxbid/src/instructions/reveal_quote.rs.
    let metas = vec![
        AccountMeta::new(maker, true),  // maker (signer, mut)
        AccountMeta::new(intent, false), // intent (mut)
        AccountMeta::new(quote, false), // quote (mut)
    ];

    finalize_tx(
        sol,
        &maker,
        vec![Instruction {
            program_id: np::id::PROGRAM,
            accounts: metas,
            data,
        }],
        PreparedAccounts {
            quote: Some(quote.to_string()),
            ..Default::default()
        },
    )
    .await
}

// ---- fund_maker_escrow --------------------------------------------

/// `POST /api/tx/fund_maker_escrow` body.
///
/// `amount` is the units of the maker-locked mint to deposit. The
/// builder fetches the `Intent` account on chain to decide whether the
/// maker locks `base_mint` (for buy intents) or `quote_mint` (for sells),
/// so the client never has to encode that branch.
#[derive(Clone, Debug, Deserialize)]
pub struct FundMakerEscrowRequest {
    pub maker: String,
    pub intent: String,
    pub quote: String,
    pub amount: u64,
}

pub async fn build_fund_maker_escrow(
    sol: &SolanaClient,
    req: FundMakerEscrowRequest,
) -> Result<PreparedTx, TxBuildError> {
    if req.amount == 0 {
        return Err(TxBuildError::ZeroValue);
    }
    let maker = parse_pk("maker", &req.maker)?;
    let intent_pk = parse_pk("intent", &req.intent)?;
    let quote_pk = parse_pk("quote", &req.quote)?;

    // Need the on-chain `Intent` to figure out the maker-locked mint.
    let intent_acc: np::state::Intent = sol
        .get_anchor_account(&intent_pk)
        .await?
        .ok_or_else(|| TxBuildError::BadPubkey {
            field: "intent",
            error: "intent account not found".to_string(),
        })?;
    let maker_lock_mint = match intent_acc.side {
        x if x == np::params::side::BUY => intent_acc.base_mint, // maker delivers base
        x if x == np::params::side::SELL => intent_acc.quote_mint, // maker delivers quote
        _ => return Err(TxBuildError::BadSide),
    };

    let (escrow_pda, _) = np::pda::escrow(&intent_pk);
    let (maker_vault_pda, _) = np::pda::maker_vault(&intent_pk);
    let (maker_source_ata, _) = np::pda::associated_token(&maker, &maker_lock_mint);
    let (reputation_pda, _) = np::pda::reputation(&maker);

    let params = np::params::FundMakerEscrowParams { amount: req.amount };
    let data =
        np::params::encode_ix_data(np::discriminator::ix::FUND_MAKER_ESCROW, &params)?;

    // Account order MUST match `FundMakerEscrow` in
    // chain/programs/nyxbid/src/instructions/fund_maker_escrow.rs.
    let metas = vec![
        AccountMeta::new(maker, true),                          // maker (signer, mut)
        AccountMeta::new(intent_pk, false),                     // intent (mut)
        AccountMeta::new(quote_pk, false),                      // quote (mut)
        AccountMeta::new(escrow_pda, false),                    // escrow (mut)
        AccountMeta::new_readonly(maker_lock_mint, false),      // maker_lock_mint
        AccountMeta::new(maker_source_ata, false),              // maker_source (mut)
        AccountMeta::new(maker_vault_pda, false),               // maker_vault (init)
        AccountMeta::new(reputation_pda, false),                // reputation (mut)
        AccountMeta::new_readonly(np::id::TOKEN, false),        // token_program
        AccountMeta::new_readonly(np::id::SYSTEM, false),       // system_program
        AccountMeta::new_readonly(np::id::SYSVAR_RENT, false),  // rent
    ];

    finalize_tx(
        sol,
        &maker,
        vec![Instruction {
            program_id: np::id::PROGRAM,
            accounts: metas,
            data,
        }],
        PreparedAccounts {
            escrow: Some(escrow_pda.to_string()),
            maker_vault: Some(maker_vault_pda.to_string()),
            quote: Some(quote_pk.to_string()),
            reputation: Some(reputation_pda.to_string()),
            ..Default::default()
        },
    )
    .await
}

// ---- settle --------------------------------------------------------

/// `POST /api/tx/settle` body. Anyone can pay rent for the receipt and
/// drive the settlement, so `payer` is decoupled from `taker`.
#[derive(Clone, Debug, Deserialize)]
pub struct SettleRequest {
    pub payer: String,
    pub intent: String,
}

pub async fn build_settle(
    sol: &SolanaClient,
    req: SettleRequest,
) -> Result<PreparedTx, TxBuildError> {
    let payer = parse_pk("payer", &req.payer)?;
    let intent_pk = parse_pk("intent", &req.intent)?;

    // We need: Intent (taker, side, mints, winning_quote, winning_price,
    // size, limit_price), Escrow (taker_amount + cached vault bumps),
    // Quote (maker pubkey).
    let intent_acc = require_account::<np::state::Intent>(sol, &intent_pk, "intent").await?;
    let (escrow_pda, _) = np::pda::escrow(&intent_pk);
    let escrow_acc =
        require_account::<np::state::Escrow>(sol, &escrow_pda, "escrow").await?;
    let winning_quote_pk = intent_acc.winning_quote;
    let quote_acc =
        require_account::<np::state::Quote>(sol, &winning_quote_pk, "winning_quote").await?;
    let maker = quote_acc.maker;

    // ATA-resolved destinations. The chain only checks owner + mint, so
    // ATAs are the right default.
    let maker_destination_ata = np::pda::associated_token(&maker, &escrow_acc.taker_mint).0;
    let taker_destination_ata =
        np::pda::associated_token(&intent_acc.taker, &escrow_acc.maker_mint).0;

    // Buy-side price-improvement refund: taker_paid is recomputed from
    // the executed price; any overpay (in escrow.taker_mint) is sent to
    // the taker's ATA on that mint. Sell-side has no refund.
    let needs_refund = if intent_acc.side == np::params::side::BUY {
        let filled_price = quote_acc.revealed_price;
        let filled_size = quote_acc.revealed_size;
        let taker_paid = mul_div(filled_size, filled_price, np::state::PRICE_SCALE)
            .ok_or(TxBuildError::ZeroValue)?; // overflow surfaces as a generic err
        taker_paid < escrow_acc.taker_amount
    } else {
        false
    };
    let taker_refund_destination_ata = if needs_refund {
        np::pda::associated_token(&intent_acc.taker, &escrow_acc.taker_mint).0
    } else {
        // Anchor optional sentinel: passing the program ID at the slot
        // resolves to None server-side.
        np::id::PROGRAM
    };

    let (receipt_pda, _) = np::pda::receipt(&intent_pk);
    let (reputation_pda, _) = np::pda::reputation(&maker);
    let taker_vault_pda = np::pda::taker_vault(&intent_pk).0;
    let maker_vault_pda = np::pda::maker_vault(&intent_pk).0;

    let data = np::params::encode_empty_ix_data(np::discriminator::ix::SETTLE);

    // Account order MUST match `Settle` in
    // chain/programs/nyxbid/src/instructions/settle.rs.
    let metas = vec![
        AccountMeta::new(payer, true),                              // payer (signer, mut)
        AccountMeta::new(intent_pk, false),                         // intent (mut)
        AccountMeta::new_readonly(winning_quote_pk, false),         // winning_quote
        AccountMeta::new(escrow_pda, false),                        // escrow (mut, closes)
        AccountMeta::new(taker_vault_pda, false),                   // taker_vault (mut)
        AccountMeta::new(maker_vault_pda, false),                   // maker_vault (mut)
        AccountMeta::new(maker_destination_ata, false),             // maker_destination
        AccountMeta::new(taker_destination_ata, false),             // taker_destination
        AccountMeta::new(taker_refund_destination_ata, false),      // optional refund
        AccountMeta::new(intent_acc.taker, false),                  // taker_rent_beneficiary
        AccountMeta::new(maker, false),                             // maker_rent_beneficiary
        AccountMeta::new(receipt_pda, false),                       // receipt (init)
        AccountMeta::new(reputation_pda, false),                    // reputation (mut)
        AccountMeta::new_readonly(intent_acc.base_mint, false),     // base_mint
        AccountMeta::new_readonly(intent_acc.quote_mint, false),    // quote_mint
        AccountMeta::new_readonly(np::id::TOKEN, false),            // token_program
        AccountMeta::new_readonly(np::id::SYSTEM, false),           // system_program
    ];

    finalize_tx(
        sol,
        &payer,
        vec![Instruction {
            program_id: np::id::PROGRAM,
            accounts: metas,
            data,
        }],
        PreparedAccounts {
            receipt: Some(receipt_pda.to_string()),
            escrow: Some(escrow_pda.to_string()),
            taker_vault: Some(taker_vault_pda.to_string()),
            maker_vault: Some(maker_vault_pda.to_string()),
            quote: Some(winning_quote_pk.to_string()),
            reputation: Some(reputation_pda.to_string()),
            ..Default::default()
        },
    )
    .await
}

// ---- cancel --------------------------------------------------------

/// `POST /api/tx/cancel` body.
#[derive(Clone, Debug, Deserialize)]
pub struct CancelRequest {
    pub taker: String,
    pub intent: String,
}

pub async fn build_cancel(
    sol: &SolanaClient,
    req: CancelRequest,
) -> Result<PreparedTx, TxBuildError> {
    let taker = parse_pk("taker", &req.taker)?;
    let intent_pk = parse_pk("intent", &req.intent)?;

    let escrow_acc_pda = np::pda::escrow(&intent_pk).0;
    let escrow = require_account::<np::state::Escrow>(sol, &escrow_acc_pda, "escrow").await?;
    let taker_vault_pda = np::pda::taker_vault(&intent_pk).0;
    let taker_destination_ata = np::pda::associated_token(&taker, &escrow.taker_mint).0;

    let data = np::params::encode_empty_ix_data(np::discriminator::ix::CANCEL);

    // Account order MUST match `Cancel` in
    // chain/programs/nyxbid/src/instructions/cancel.rs.
    let metas = vec![
        AccountMeta::new(taker, true),                       // taker (signer, mut)
        AccountMeta::new(intent_pk, false),                  // intent (mut)
        AccountMeta::new(escrow_acc_pda, false),             // escrow (mut, closes)
        AccountMeta::new(taker_vault_pda, false),            // taker_vault (mut)
        AccountMeta::new(taker_destination_ata, false),      // taker_destination
        AccountMeta::new_readonly(np::id::TOKEN, false),     // token_program
    ];

    finalize_tx(
        sol,
        &taker,
        vec![Instruction {
            program_id: np::id::PROGRAM,
            accounts: metas,
            data,
        }],
        PreparedAccounts {
            escrow: Some(escrow_acc_pda.to_string()),
            taker_vault: Some(taker_vault_pda.to_string()),
            ..Default::default()
        },
    )
    .await
}

// ---- expire_with_maker --------------------------------------------

/// `POST /api/tx/expire_with_maker` body. Permissionless after the
/// settle deadline if the winning maker funded but never settled.
#[derive(Clone, Debug, Deserialize)]
pub struct ExpireWithMakerRequest {
    pub payer: String,
    pub intent: String,
}

pub async fn build_expire_with_maker(
    sol: &SolanaClient,
    req: ExpireWithMakerRequest,
) -> Result<PreparedTx, TxBuildError> {
    let payer = parse_pk("payer", &req.payer)?;
    let intent_pk = parse_pk("intent", &req.intent)?;

    let intent_acc =
        require_account::<np::state::Intent>(sol, &intent_pk, "intent").await?;
    let escrow_pda = np::pda::escrow(&intent_pk).0;
    let escrow_acc =
        require_account::<np::state::Escrow>(sol, &escrow_pda, "escrow").await?;
    let maker = escrow_acc.maker;

    let taker_vault_pda = np::pda::taker_vault(&intent_pk).0;
    let maker_vault_pda = np::pda::maker_vault(&intent_pk).0;
    let taker_destination_ata =
        np::pda::associated_token(&intent_acc.taker, &escrow_acc.taker_mint).0;
    let maker_destination_ata =
        np::pda::associated_token(&maker, &escrow_acc.maker_mint).0;
    let reputation_pda = np::pda::reputation(&maker).0;

    let data = np::params::encode_empty_ix_data(np::discriminator::ix::EXPIRE_WITH_MAKER);

    // Account order MUST match `ExpireWithMaker` in
    // chain/programs/nyxbid/src/instructions/expire_with_maker.rs.
    let metas = vec![
        AccountMeta::new(payer, true),                          // payer (signer, mut)
        AccountMeta::new(intent_pk, false),                     // intent (mut)
        AccountMeta::new(escrow_pda, false),                    // escrow (mut, closes)
        AccountMeta::new(taker_vault_pda, false),               // taker_vault (mut)
        AccountMeta::new(taker_destination_ata, false),         // taker_destination
        AccountMeta::new(intent_acc.taker, false),              // taker_rent_beneficiary
        AccountMeta::new(maker_vault_pda, false),               // maker_vault (mut)
        AccountMeta::new(maker_destination_ata, false),         // maker_destination
        AccountMeta::new(maker, false),                         // maker_rent_beneficiary
        AccountMeta::new(reputation_pda, false),                // reputation (mut)
        AccountMeta::new_readonly(np::id::TOKEN, false),        // token_program
    ];

    finalize_tx(
        sol,
        &payer,
        vec![Instruction {
            program_id: np::id::PROGRAM,
            accounts: metas,
            data,
        }],
        PreparedAccounts {
            escrow: Some(escrow_pda.to_string()),
            taker_vault: Some(taker_vault_pda.to_string()),
            maker_vault: Some(maker_vault_pda.to_string()),
            reputation: Some(reputation_pda.to_string()),
            ..Default::default()
        },
    )
    .await
}

// ---- expire_no_maker -----------------------------------------------

/// `POST /api/tx/expire_no_maker` body. Permissionless after the
/// settle deadline when no maker ever funded the escrow. Refunds the
/// taker leg; if a winner was selected by reveal but never funded, also
/// bumps that maker's `failed_reveals` counter.
#[derive(Clone, Debug, Deserialize)]
pub struct ExpireNoMakerRequest {
    pub payer: String,
    pub intent: String,
}

pub async fn build_expire_no_maker(
    sol: &SolanaClient,
    req: ExpireNoMakerRequest,
) -> Result<PreparedTx, TxBuildError> {
    let payer = parse_pk("payer", &req.payer)?;
    let intent_pk = parse_pk("intent", &req.intent)?;

    let intent_acc =
        require_account::<np::state::Intent>(sol, &intent_pk, "intent").await?;
    let escrow_pda = np::pda::escrow(&intent_pk).0;
    let escrow_acc =
        require_account::<np::state::Escrow>(sol, &escrow_pda, "escrow").await?;
    let taker_vault_pda = np::pda::taker_vault(&intent_pk).0;
    let taker_destination_ata =
        np::pda::associated_token(&intent_acc.taker, &escrow_acc.taker_mint).0;

    // Optional accounts: present only when a winner was selected during
    // reveal but never funded. We pass program_id as the Anchor "None"
    // sentinel for the not-present case.
    let has_winner = intent_acc.winning_quote != Pubkey::default();
    let (winning_quote_meta_key, winning_reputation_meta_key) = if has_winner {
        let q = require_account::<np::state::Quote>(
            sol,
            &intent_acc.winning_quote,
            "winning_quote",
        )
        .await?;
        let rep = np::pda::reputation(&q.maker).0;
        (intent_acc.winning_quote, rep)
    } else {
        (np::id::PROGRAM, np::id::PROGRAM)
    };

    let data = np::params::encode_empty_ix_data(np::discriminator::ix::EXPIRE_NO_MAKER);

    // Account order MUST match `ExpireNoMaker` in
    // chain/programs/nyxbid/src/instructions/expire_no_maker.rs.
    let metas = vec![
        AccountMeta::new(payer, true),                          // payer (signer, mut)
        AccountMeta::new(intent_pk, false),                     // intent (mut)
        AccountMeta::new(escrow_pda, false),                    // escrow (mut, closes)
        AccountMeta::new(taker_vault_pda, false),               // taker_vault (mut)
        AccountMeta::new(taker_destination_ata, false),         // taker_destination
        AccountMeta::new(intent_acc.taker, false),              // taker_rent_beneficiary
        AccountMeta::new_readonly(winning_quote_meta_key, false), // winning_quote (Option)
        AccountMeta::new(winning_reputation_meta_key, false),   // winning_maker_reputation (Option)
        AccountMeta::new_readonly(np::id::TOKEN, false),        // token_program
    ];

    finalize_tx(
        sol,
        &payer,
        vec![Instruction {
            program_id: np::id::PROGRAM,
            accounts: metas,
            data,
        }],
        PreparedAccounts {
            escrow: Some(escrow_pda.to_string()),
            taker_vault: Some(taker_vault_pda.to_string()),
            ..Default::default()
        },
    )
    .await
}

// ---- helpers -------------------------------------------------------

/// Fetch + decode an Anchor account, surfacing a 400-style error if the
/// account does not exist on chain.
async fn require_account<T: np::AnchorAccount>(
    sol: &SolanaClient,
    pk: &Pubkey,
    field: &'static str,
) -> Result<T, TxBuildError> {
    sol.get_anchor_account::<T>(pk)
        .await?
        .ok_or_else(|| TxBuildError::BadPubkey {
            field,
            error: format!("account {pk} not found on chain"),
        })
}

/// Saturating-safe `(a * b) / scale` in u128.
fn mul_div(a: u64, b: u64, scale: u64) -> Option<u64> {
    let n = (a as u128).checked_mul(b as u128)?;
    let n = n.checked_div(scale as u128)?;
    u64::try_from(n).ok()
}

/// Pull the latest blockhash, build the unsigned `Transaction`, and
/// serialise it for the wire. Shared by every builder above.
///
/// Accepts a Vec so builders can prepend setup instructions (e.g. ATA
/// `create_idempotent`) before the program call. Single-instruction
/// builders pass `vec![ix]`.
async fn finalize_tx(
    sol: &SolanaClient,
    fee_payer: &Pubkey,
    ixs: Vec<Instruction>,
    accounts: PreparedAccounts,
) -> Result<PreparedTx, TxBuildError> {
    let (blockhash, last_valid_block_height) = sol.latest_blockhash().await?;
    let message = Message::new_with_blockhash(&ixs, Some(fee_payer), &blockhash);
    let tx = Transaction::new_unsigned(message.clone());
    let tx_bytes = bincode::serialize(&tx)?;
    let msg_bytes = bincode::serialize(&message)?;
    Ok(PreparedTx {
        tx_base64: B64.encode(tx_bytes),
        message_base64: B64.encode(msg_bytes),
        blockhash: blockhash.to_string(),
        last_valid_block_height,
        fee_payer: fee_payer.to_string(),
        accounts,
    })
}

fn parse_pk(field: &'static str, s: &str) -> Result<Pubkey, TxBuildError> {
    use std::str::FromStr;
    Pubkey::from_str(s).map_err(|e| TxBuildError::BadPubkey {
        field,
        error: e.to_string(),
    })
}

fn parse_fixed_hex<const N: usize>(
    field: &'static str,
    s: &str,
) -> Result<[u8; N], TxBuildError> {
    let raw = hex::decode(s).map_err(|e| TxBuildError::BadHex {
        field,
        error: e.to_string(),
    })?;
    if raw.len() != N {
        return Err(TxBuildError::WrongLength {
            field,
            expected: N,
            got: raw.len(),
        });
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&raw);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first 8 bytes of the data field MUST be the Anchor
    /// discriminator. Borsh-encoded params must round-trip exactly.
    #[test]
    fn create_intent_data_layout_is_anchor_compatible() {
        let params = np::params::CreateIntentParams {
            side: np::params::side::BUY,
            size: 1_000_000_000,
            limit_price: 250_000_000,
            reveal_deadline: 1_700_000_000,
            resolve_deadline: 1_700_000_030,
            settle_deadline: 1_700_000_120,
            commitment_root: [7u8; 32],
            nonce: [3u8; 16],
        };
        let data =
            np::params::encode_ix_data(np::discriminator::ix::CREATE_INTENT, &params).unwrap();

        assert_eq!(&data[..8], &np::discriminator::ix::CREATE_INTENT);
        // u8 + u64 + u64 + i64 + i64 + i64 + [u8;32] + [u8;16]
        let expected_param_bytes = 1 + 8 + 8 + 8 + 8 + 8 + 32 + 16;
        assert_eq!(data.len(), 8 + expected_param_bytes);

        // Round-trip: borsh decode the body.
        let decoded =
            <np::params::CreateIntentParams as borsh::BorshDeserialize>::try_from_slice(
                &data[8..],
            )
            .unwrap();
        assert_eq!(decoded.size, params.size);
        assert_eq!(decoded.limit_price, params.limit_price);
        assert_eq!(decoded.commitment_root, params.commitment_root);
        assert_eq!(decoded.nonce, params.nonce);
    }

    #[test]
    fn submit_quote_data_layout() {
        let p = np::params::SubmitQuoteParams {
            commitment: [9u8; 32],
            nonce: [4u8; 16],
        };
        let data =
            np::params::encode_ix_data(np::discriminator::ix::SUBMIT_QUOTE, &p).unwrap();
        assert_eq!(&data[..8], &np::discriminator::ix::SUBMIT_QUOTE);
        // 32 + 16 = 48 param bytes
        assert_eq!(data.len(), 8 + 48);
        let decoded =
            <np::params::SubmitQuoteParams as borsh::BorshDeserialize>::try_from_slice(
                &data[8..],
            )
            .unwrap();
        assert_eq!(decoded.commitment, p.commitment);
        assert_eq!(decoded.nonce, p.nonce);
    }

    #[test]
    fn reveal_quote_data_layout() {
        let p = np::params::RevealQuoteParams {
            revealed_price: 123_456_789,
            revealed_size: 987_654_321,
            nonce: [5u8; 32],
        };
        let data =
            np::params::encode_ix_data(np::discriminator::ix::REVEAL_QUOTE, &p).unwrap();
        assert_eq!(&data[..8], &np::discriminator::ix::REVEAL_QUOTE);
        // u64 + u64 + [u8;32] = 48 param bytes
        assert_eq!(data.len(), 8 + 48);
    }

    #[test]
    fn fund_maker_escrow_data_layout() {
        let p = np::params::FundMakerEscrowParams {
            amount: 9_999_999,
        };
        let data =
            np::params::encode_ix_data(np::discriminator::ix::FUND_MAKER_ESCROW, &p).unwrap();
        assert_eq!(&data[..8], &np::discriminator::ix::FUND_MAKER_ESCROW);
        // u64 = 8 param bytes
        assert_eq!(data.len(), 8 + 8);
    }
}
