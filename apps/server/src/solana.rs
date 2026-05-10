//! Thin async wrapper around `solana_client::nonblocking::RpcClient`.
//!
//! Phase 2 entry point for "the server actually talks to the chain".
//! Exposes only the calls the rest of the server uses:
//!
//! - latest blockhash (for tx assembly),
//! - account fetch + decode,
//! - get_program_accounts (for cold-start backfill),
//! - send signed transaction (relay; we never sign),
//! - signature status / confirmation polling.
//!
//! The program ID and RPC URL are read from `NYXBID_PROGRAM_ID` and
//! `SOLANA_RPC_URL`. If the env var omits the program ID, we fall back
//! to [`nyxbid_program::id::PROGRAM`] and warn. If both env vars are
//! missing, [`SolanaClient::from_env`] returns `None` so the server can
//! still boot in offline/local-mock mode.

use std::{env, str::FromStr, sync::Arc, time::Duration};

use nyxbid_program as np;
use solana_account::Account;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcSendTransactionConfig, RpcTransactionConfig},
    rpc_response::RpcSimulateTransactionResult,
};
use solana_commitment_config::{CommitmentConfig, CommitmentLevel};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::Transaction;
use solana_transaction_status_client_types::UiTransactionEncoding;

/// Errors surfaced by the wrapper. Each variant maps cleanly onto an
/// HTTP 5xx in the route layer.
#[derive(Debug, thiserror::Error)]
pub enum SolanaError {
    #[error("solana rpc error: {0}")]
    Rpc(#[from] solana_client::client_error::ClientError),
    #[error("invalid pubkey: {0}")]
    BadPubkey(String),
    #[error("invalid signature: {0}")]
    BadSignature(String),
    #[error("decode: {0}")]
    Decode(#[from] np::DecodeError),
    /// Returned by future helpers that surface a confirmed-but-failed tx
    /// to the caller (e.g. SDK consumers awaiting finality). Kept on the
    /// public error surface so the variant is wire-stable.
    #[allow(dead_code)]
    #[error("transaction failed: {0:?}")]
    TxFailed(Box<solana_transaction_error::TransactionError>),
    /// Returned when `tx_status` was polled before the cluster has seen
    /// the signature. Public for the same reason as `TxFailed`.
    #[allow(dead_code)]
    #[error("transaction not landed yet")]
    TxPending,
}

/// Coarse status of a submitted transaction.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TxStatus {
    /// Not yet seen by the cluster.
    Pending,
    /// Landed in a processed slot but not yet confirmed.
    Processed,
    /// Reached the confirmed commitment level.
    Confirmed,
    /// Reached the finalized commitment level.
    Finalized,
    /// On-chain execution failed; see logs.
    Failed,
}

/// Thread-safe handle to a configured RPC client.
#[derive(Clone)]
pub struct SolanaClient {
    pub rpc_url: String,
    /// WebSocket endpoint used by the log indexer. Derived from
    /// `rpc_url` (https -> wss, http -> ws) unless `SOLANA_WS_URL`
    /// overrides it.
    pub ws_url: String,
    pub program_id: Pubkey,
    pub usdc_mint: Pubkey,
    /// `Arc` because the indexer holds one and route handlers hold another.
    rpc: Arc<RpcClient>,
}

impl std::fmt::Debug for SolanaClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use crate::url_privacy::public_origin;
        f.debug_struct("SolanaClient")
            .field("rpc_url", &public_origin(&self.rpc_url))
            .field("ws_url", &public_origin(&self.ws_url))
            .field("program_id", &self.program_id.to_string())
            .field("usdc_mint", &self.usdc_mint.to_string())
            .finish()
    }
}

impl SolanaClient {
    /// Build a client from environment variables. Returns `None` if the
    /// minimum config (`SOLANA_RPC_URL`) is absent.
    ///
    /// - `SOLANA_RPC_URL` (required): e.g. `https://api.devnet.solana.com`.
    /// - `NYXBID_PROGRAM_ID` (optional): override the bundled program ID.
    /// - `NYXBID_USDC_MINT` (optional): defaults to the devnet USDC faucet mint.
    pub fn from_env() -> Option<Self> {
        let rpc_url = env::var("SOLANA_RPC_URL").ok()?;
        let ws_url = env::var("SOLANA_WS_URL").unwrap_or_else(|_| derive_ws_url(&rpc_url));
        let program_id = match env::var("NYXBID_PROGRAM_ID") {
            Ok(s) => match Pubkey::from_str(&s) {
                Ok(pk) => {
                    if pk != np::id::PROGRAM {
                        tracing::warn!(
                            env_program_id = %pk,
                            crate_program_id = %np::id::PROGRAM,
                            "NYXBID_PROGRAM_ID does not match nyxbid-program::id::PROGRAM"
                        );
                    }
                    pk
                }
                Err(e) => {
                    tracing::warn!(error = %e, "invalid NYXBID_PROGRAM_ID, falling back to crate const");
                    np::id::PROGRAM
                }
            },
            Err(_) => np::id::PROGRAM,
        };
        let usdc_mint = env::var("NYXBID_USDC_MINT")
            .ok()
            .and_then(|s| Pubkey::from_str(&s).ok())
            .unwrap_or_else(|| {
                Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU")
                    .expect("hard-coded mint parses")
            });

        let rpc = RpcClient::new_with_commitment(
            rpc_url.clone(),
            CommitmentConfig::confirmed(),
        );

        Some(Self {
            rpc_url,
            ws_url,
            program_id,
            usdc_mint,
            rpc: Arc::new(rpc),
        })
    }

    /// Latest blockhash **and** the last valid block height for it.
    /// The tx builder bakes both into [`PreparedTx`] so the wallet
    /// knows when to re-prepare if the user is slow to sign.
    pub async fn latest_blockhash(
        &self,
    ) -> Result<(solana_hash::Hash, u64), SolanaError> {
        let (hash, height) = self
            .rpc
            .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
            .await?;
        Ok((hash, height))
    }

    /// Fetch a single account; returns `Ok(None)` when the account does
    /// not exist (RPC returns null), bubbling other errors up.
    pub async fn get_account(&self, pk: &Pubkey) -> Result<Option<Account>, SolanaError> {
        match self.rpc.get_account(pk).await {
            Ok(a) => Ok(Some(a)),
            Err(e) => {
                use solana_client::client_error::ClientErrorKind;
                use solana_client::rpc_request::RpcResponseErrorData;
                if let ClientErrorKind::RpcError(
                    solana_client::rpc_request::RpcError::RpcResponseError {
                        data: RpcResponseErrorData::Empty,
                        ..
                    },
                ) = e.kind()
                {
                    return Ok(None);
                }
                if e.to_string().contains("AccountNotFound")
                    || e.to_string().contains("could not find account")
                {
                    return Ok(None);
                }
                Err(SolanaError::Rpc(e))
            }
        }
    }

    /// Decode an Anchor-style account in one call.
    pub async fn get_anchor_account<T: np::AnchorAccount>(
        &self,
        pk: &Pubkey,
    ) -> Result<Option<T>, SolanaError> {
        match self.get_account(pk).await? {
            Some(a) => Ok(Some(T::try_decode(&a.data)?)),
            None => Ok(None),
        }
    }

    /// Fetch every account owned by the program. Used at indexer cold
    /// start to backfill in-memory state. Filters out accounts that do
    /// not match `T`'s discriminator.
    pub async fn list_program_accounts<T: np::AnchorAccount>(
        &self,
    ) -> Result<Vec<(Pubkey, T)>, SolanaError> {
        let raw = self.rpc.get_program_accounts(&self.program_id).await?;
        let mut out = Vec::with_capacity(raw.len());
        for (pk, acct) in raw {
            if acct.data.len() < 8 || acct.data[..8] != T::DISCRIMINATOR {
                continue;
            }
            match T::try_decode(&acct.data) {
                Ok(decoded) => out.push((pk, decoded)),
                Err(e) => tracing::warn!(account = %pk, error = %e, "skipping malformed account"),
            }
        }
        Ok(out)
    }

    /// Submit a base64-encoded, signed transaction to the cluster.
    /// We never sign on the server; this is pure relay for clients that
    /// want the server to broadcast on their behalf.
    pub async fn send_signed_transaction(
        &self,
        tx_bytes: &[u8],
    ) -> Result<Signature, SolanaError> {
        let tx: Transaction = bincode::deserialize(tx_bytes)
            .map_err(|e| SolanaError::BadSignature(format!("decode tx: {e}")))?;
        let cfg = RpcSendTransactionConfig {
            skip_preflight: false,
            preflight_commitment: Some(CommitmentLevel::Confirmed),
            encoding: Some(UiTransactionEncoding::Base64),
            max_retries: Some(3),
            min_context_slot: None,
        };
        Ok(self
            .rpc
            .send_transaction_with_config(&tx, cfg)
            .await?)
    }

    /// Simulate a base64-encoded, signed transaction. Returns the raw
    /// simulation result so callers can surface program logs to the UI.
    pub async fn simulate_signed_transaction(
        &self,
        tx_bytes: &[u8],
    ) -> Result<RpcSimulateTransactionResult, SolanaError> {
        let tx: Transaction = bincode::deserialize(tx_bytes)
            .map_err(|e| SolanaError::BadSignature(format!("decode tx: {e}")))?;
        Ok(self.rpc.simulate_transaction(&tx).await?.value)
    }

    /// Coarse status for one signature. Maps the multi-state Solana
    /// commitment ladder onto a flat enum the UI can render.
    pub async fn tx_status(&self, sig: &Signature) -> Result<TxStatus, SolanaError> {
        let resp = self.rpc.get_signature_statuses(&[*sig]).await?;
        let Some(Some(status)) = resp.value.into_iter().next() else {
            return Ok(TxStatus::Pending);
        };
        if let Some(err) = status.err {
            tracing::debug!(signature = %sig, error = ?err, "tx returned error status");
            return Ok(TxStatus::Failed);
        }
        // confirmation_status is None on very-recent processed txs.
        match status.confirmation_status {
            Some(solana_transaction_status_client_types::TransactionConfirmationStatus::Finalized) => {
                Ok(TxStatus::Finalized)
            }
            Some(solana_transaction_status_client_types::TransactionConfirmationStatus::Confirmed) => {
                Ok(TxStatus::Confirmed)
            }
            Some(solana_transaction_status_client_types::TransactionConfirmationStatus::Processed)
            | None => Ok(TxStatus::Processed),
        }
    }

    /// Block until `sig` reaches `target` or `timeout` elapses. Polls
    /// every 400ms (one Solana slot, roughly). Used by the tx tracker.
    #[allow(dead_code)] // exposed for Phase 3/4 maker bots
    pub async fn await_status(
        &self,
        sig: &Signature,
        target: TxStatus,
        timeout: Duration,
    ) -> Result<TxStatus, SolanaError> {
        let deadline = std::time::Instant::now() + timeout;
        let target_rank = rank(target);
        loop {
            let s = self.tx_status(sig).await?;
            if matches!(s, TxStatus::Failed) || rank(s) >= target_rank {
                return Ok(s);
            }
            if std::time::Instant::now() >= deadline {
                return Ok(s);
            }
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    }

    /// Convenience: parse a base58 signature from the wire.
    pub fn parse_signature(s: &str) -> Result<Signature, SolanaError> {
        Signature::from_str(s).map_err(|e| SolanaError::BadSignature(e.to_string()))
    }

    /// Convenience: parse a base58 pubkey from the wire.
    #[allow(dead_code)] // SDK-style helper, used by maker bots
    pub fn parse_pubkey(s: &str) -> Result<Pubkey, SolanaError> {
        Pubkey::from_str(s).map_err(|e| SolanaError::BadPubkey(e.to_string()))
    }

    /// Underlying client, exposed for power users (the indexer needs it
    /// to subscribe to logs over WebSocket).
    #[allow(dead_code)]
    pub fn raw(&self) -> Arc<RpcClient> {
        Arc::clone(&self.rpc)
    }

    /// Optional helper for `getTransaction` calls used by the tx tracker.
    #[allow(dead_code)] // used by the upcoming fill-detail route
    pub async fn get_transaction_meta(
        &self,
        sig: &Signature,
    ) -> Result<
        Option<solana_transaction_status_client_types::EncodedConfirmedTransactionWithStatusMeta>,
        SolanaError,
    > {
        let cfg = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };
        match self.rpc.get_transaction_with_config(sig, cfg).await {
            Ok(t) => Ok(Some(t)),
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(None)
                } else {
                    Err(SolanaError::Rpc(e))
                }
            }
        }
    }
}

/// Map an HTTP RPC URL to its WebSocket equivalent.
fn derive_ws_url(rpc: &str) -> String {
    if let Some(rest) = rpc.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = rpc.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        // Already ws/wss/something else; pass through.
        rpc.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::derive_ws_url;

    #[test]
    fn ws_url_https_to_wss() {
        assert_eq!(
            derive_ws_url("https://api.devnet.solana.com"),
            "wss://api.devnet.solana.com"
        );
    }

    #[test]
    fn ws_url_http_to_ws() {
        assert_eq!(derive_ws_url("http://127.0.0.1:8899"), "ws://127.0.0.1:8899");
    }

    #[test]
    fn ws_url_passthrough_for_already_ws() {
        assert_eq!(
            derive_ws_url("wss://example.com"),
            "wss://example.com"
        );
    }
}

#[allow(dead_code)] // referenced from await_status
fn rank(s: TxStatus) -> u8 {
    match s {
        TxStatus::Pending => 0,
        TxStatus::Processed => 1,
        TxStatus::Confirmed => 2,
        TxStatus::Finalized => 3,
        TxStatus::Failed => 0,
    }
}
