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

    let ix = Instruction {
        program_id: np::id::PROGRAM,
        accounts: metas,
        data,
    };

    // -- pull blockhash and assemble the unsigned tx -----------------
    let blockhash = sol.latest_blockhash().await?;
    // We don't have getLatestBlockhashWithLastValidBlockHeight in the
    // wrapper yet; surface 0 for now and expose a real value when the
    // tx-tracker route lands in commit 8.
    let last_valid_block_height = 0;

    let message = Message::new_with_blockhash(&[ix], Some(&taker), &blockhash);
    let tx = Transaction::new_unsigned(message.clone());

    let tx_bytes = bincode::serialize(&tx)?;
    let msg_bytes = bincode::serialize(&message)?;

    Ok(PreparedTx {
        tx_base64: B64.encode(tx_bytes),
        message_base64: B64.encode(msg_bytes),
        blockhash: blockhash.to_string(),
        last_valid_block_height,
        fee_payer: taker.to_string(),
        accounts: PreparedAccounts {
            intent: Some(intent_pda.to_string()),
            escrow: Some(escrow_pda.to_string()),
            taker_vault: Some(taker_vault_pda.to_string()),
            ..Default::default()
        },
    })
}

// ---- helpers -------------------------------------------------------

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
}
