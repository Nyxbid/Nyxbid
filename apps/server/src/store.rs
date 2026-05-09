//! Read-through cache of every Anchor account the program owns.
//!
//! The store is the single source of truth the REST routes read from.
//! It is fed by two pipelines:
//!
//! 1. **Cold-start backfill** ([`Store::cold_start`]): on boot we call
//!    `getProgramAccounts` once for each account type (Intent, Quote,
//!    Receipt) and load decoded structs into the in-memory maps. This
//!    catches accounts that existed before the indexer connected.
//!
//! 2. **Live updates** ([`Store::apply_event`]): the indexer task
//!    forwards every `ChainEnvelope` here. We re-fetch the touched
//!    account(s) over RPC and overwrite the cached entry. Re-fetch
//!    instead of trusting the event payload alone because the account
//!    state contains fields the event does not (e.g. status transitions
//!    on `Settled`/`Cancelled`).
//!
//! Anything that fails to land on chain never appears in this cache,
//! which keeps the public surface aligned with the chain by construction.
//!
//! When the in-memory maps below are eventually replaced by a real DB
//! (Phase 6), the public methods here are the seam to swap.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use nyxbid_program as np;
use nyxbid_types as dto;
use solana_pubkey::Pubkey;
use tokio::sync::RwLock;

use crate::indexer::{ChainEnvelope, ChainEvent};
use crate::solana::{SolanaClient, SolanaError};

/// Cached snapshot of an `Intent` account plus the off-chain context
/// the DTO needs (first-seen timestamp).
#[derive(Clone, Debug)]
pub struct IntentRow {
    pub pubkey: Pubkey,
    pub data: np::state::Intent,
    pub observed_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct QuoteRow {
    pub pubkey: Pubkey,
    pub data: np::state::Quote,
    pub observed_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct ReceiptRow {
    pub pubkey: Pubkey,
    pub data: np::state::Receipt,
    /// Signature of the `settle` transaction that produced this receipt.
    /// `None` for receipts pulled in via cold-start (we don't reconstruct
    /// the tx from the account data).
    pub tx_signature: Option<String>,
}

/// In-memory chain-indexed cache. All read paths use `RwLock` for
/// cheap concurrent fan-out; writes happen only from the indexer task
/// and the cold-start coroutine.
#[derive(Default)]
pub struct Store {
    intents: HashMap<Pubkey, IntentRow>,
    quotes: HashMap<Pubkey, QuoteRow>,
    receipts: HashMap<Pubkey, ReceiptRow>,
}

pub type SharedStore = Arc<RwLock<Store>>;

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    // ---- read paths used by routes -------------------------------------

    pub fn list_intents(&self) -> Vec<dto::Intent> {
        let mut v: Vec<&IntentRow> = self.intents.values().collect();
        // Newest first by observed_at; useful default for the dashboard.
        v.sort_by(|a, b| b.observed_at.cmp(&a.observed_at));
        v.into_iter().map(intent_to_dto).collect()
    }

    pub fn get_intent(&self, id: &str) -> Option<dto::Intent> {
        let pk = id.parse::<Pubkey>().ok()?;
        self.intents.get(&pk).map(intent_to_dto)
    }

    pub fn list_quotes_for(&self, intent_id: &str) -> Vec<dto::Quote> {
        let Ok(intent_pk) = intent_id.parse::<Pubkey>() else {
            return Vec::new();
        };
        self.quotes
            .values()
            .filter(|q| q.data.intent == intent_pk)
            .map(quote_to_dto)
            .collect()
    }

    pub fn list_fills(&self) -> Vec<dto::Fill> {
        let mut v: Vec<&ReceiptRow> = self.receipts.values().collect();
        v.sort_by(|a, b| b.data.settled_at.cmp(&a.data.settled_at));
        // Resolve maker/taker via the receipt itself; `intent_id` is the
        // intent pubkey base58 string so the UI can deep-link.
        v.into_iter().map(receipt_to_dto).collect()
    }

    pub fn dashboard_stats(&self) -> dto::DashboardStats {
        let mut open = 0u64;
        let mut resolved = 0u64;
        for row in self.intents.values() {
            match row.data.status {
                np::state::intent_status::OPEN => open += 1,
                np::state::intent_status::RESOLVED => resolved += 1,
                _ => {}
            }
        }
        let total_fills = self.receipts.len() as u64;
        let cutoff_24h = Utc::now().timestamp() - 86_400;
        let notional_24h: u64 = self
            .receipts
            .values()
            .filter(|r| r.data.settled_at >= cutoff_24h)
            .map(|r| {
                (r.data.filled_size as u128 * r.data.filled_price as u128
                    / np::state::PRICE_SCALE as u128) as u64
            })
            .sum();
        let avg_makers_per_intent = if self.intents.is_empty() {
            0.0
        } else {
            self.quotes.len() as f64 / self.intents.len() as f64
        };
        dto::DashboardStats {
            open_intents: open,
            resolved_intents: resolved,
            total_fills,
            notional_24h,
            avg_makers_per_intent,
        }
    }

    // ---- write paths ---------------------------------------------------

    /// Pull every Intent / Quote / Receipt the program currently owns
    /// and load them into the cache. Run once at boot before the
    /// indexer's live stream starts emitting.
    pub async fn cold_start(&mut self, sol: &SolanaClient) -> Result<(), SolanaError> {
        let now = Utc::now();
        for (pk, intent) in sol
            .list_program_accounts::<np::state::Intent>()
            .await?
        {
            self.intents.insert(
                pk,
                IntentRow {
                    pubkey: pk,
                    data: intent,
                    observed_at: now,
                },
            );
        }
        for (pk, quote) in sol
            .list_program_accounts::<np::state::Quote>()
            .await?
        {
            self.quotes.insert(
                pk,
                QuoteRow {
                    pubkey: pk,
                    data: quote,
                    observed_at: now,
                },
            );
        }
        for (pk, receipt) in sol
            .list_program_accounts::<np::state::Receipt>()
            .await?
        {
            self.receipts.insert(
                pk,
                ReceiptRow {
                    pubkey: pk,
                    data: receipt,
                    tx_signature: None,
                },
            );
        }
        tracing::info!(
            intents = self.intents.len(),
            quotes = self.quotes.len(),
            receipts = self.receipts.len(),
            "store cold-start backfill complete"
        );
        Ok(())
    }

    /// Apply a batch of pre-fetched account updates to the cache.
    ///
    /// This is the **only** method that mutates the cache after
    /// cold-start. It holds `&mut self` (= the write lock) only for
    /// the in-memory inserts/removes — no I/O happens here. The RPC
    /// calls live in the free-standing [`fetch_updates`] function,
    /// which runs with **no lock** held.
    pub fn apply_updates(&mut self, updates: Vec<StoreUpdate>) {
        let now = Utc::now();
        for update in updates {
            match update {
                StoreUpdate::Intent(pk, Some(data)) => {
                    let observed_at = self
                        .intents
                        .get(&pk)
                        .map(|r| r.observed_at)
                        .unwrap_or(now);
                    self.intents.insert(pk, IntentRow { pubkey: pk, data, observed_at });
                }
                StoreUpdate::Intent(pk, None) => {
                    self.intents.remove(&pk);
                }
                StoreUpdate::Quote(pk, Some(data)) => {
                    let observed_at = self
                        .quotes
                        .get(&pk)
                        .map(|r| r.observed_at)
                        .unwrap_or(now);
                    self.quotes.insert(pk, QuoteRow { pubkey: pk, data, observed_at });
                }
                StoreUpdate::Quote(pk, None) => {
                    self.quotes.remove(&pk);
                }
                StoreUpdate::Receipt(pk, Some(data), sig) => {
                    self.receipts.insert(
                        pk,
                        ReceiptRow { pubkey: pk, data, tx_signature: sig },
                    );
                }
                StoreUpdate::Receipt(pk, None, _) => {
                    self.receipts.remove(&pk);
                }
            }
        }
    }
}

/// What an RPC call fetched for one account.
pub enum StoreUpdate {
    Intent(Pubkey, Option<np::state::Intent>),
    Quote(Pubkey, Option<np::state::Quote>),
    Receipt(Pubkey, Option<np::state::Receipt>, Option<String>),
}

/// Determine which accounts a chain event touched, fetch them over RPC,
/// and return the results for a later [`Store::apply_updates`] call.
///
/// This function is **async** and makes one or more RPC calls, but it
/// does **not** hold any lock on the store. Callers acquire the write
/// lock only after this returns, keeping it held for sub-microsecond
/// in-memory mutations.
pub async fn fetch_updates(
    env: &ChainEnvelope,
    sol: &SolanaClient,
) -> Result<Vec<StoreUpdate>, SolanaError> {
    let mut out = Vec::with_capacity(2);
    match &env.event {
        ChainEvent::IntentCreated(e) => {
            let d = sol.get_anchor_account::<np::state::Intent>(&e.intent).await?;
            out.push(StoreUpdate::Intent(e.intent, d));
        }
        ChainEvent::QuoteSubmitted(e) => {
            let d = sol.get_anchor_account::<np::state::Quote>(&e.quote).await?;
            out.push(StoreUpdate::Quote(e.quote, d));
        }
        ChainEvent::QuoteRevealed(e) => {
            let d = sol.get_anchor_account::<np::state::Quote>(&e.quote).await?;
            out.push(StoreUpdate::Quote(e.quote, d));
            let i = sol.get_anchor_account::<np::state::Intent>(&e.intent).await?;
            out.push(StoreUpdate::Intent(e.intent, i));
        }
        ChainEvent::AuctionResolved(e) => {
            let i = sol.get_anchor_account::<np::state::Intent>(&e.intent).await?;
            out.push(StoreUpdate::Intent(e.intent, i));
            let q = sol.get_anchor_account::<np::state::Quote>(&e.winning_quote).await?;
            out.push(StoreUpdate::Quote(e.winning_quote, q));
        }
        ChainEvent::Settled(e) => {
            let i = sol.get_anchor_account::<np::state::Intent>(&e.intent).await?;
            out.push(StoreUpdate::Intent(e.intent, i));
            let r = sol.get_anchor_account::<np::state::Receipt>(&e.receipt).await?;
            out.push(StoreUpdate::Receipt(e.receipt, r, Some(env.signature.clone())));
        }
        ChainEvent::Cancelled(e) => {
            let i = sol.get_anchor_account::<np::state::Intent>(&e.intent).await?;
            out.push(StoreUpdate::Intent(e.intent, i));
        }
    }
    Ok(out)
}

// ---- DTO projection -----------------------------------------------------

fn intent_to_dto(row: &IntentRow) -> dto::Intent {
    let i = &row.data;
    let side = match i.side {
        np::params::side::SELL => dto::Side::Sell,
        _ => dto::Side::Buy,
    };
    let status = match i.status {
        np::state::intent_status::OPEN => dto::IntentStatus::Open,
        np::state::intent_status::RESOLVED => dto::IntentStatus::Resolved,
        np::state::intent_status::SETTLED => dto::IntentStatus::Settled,
        np::state::intent_status::CANCELLED => dto::IntentStatus::Cancelled,
        np::state::intent_status::EXPIRED => dto::IntentStatus::Expired,
        _ => dto::IntentStatus::Open,
    };
    let winning_quote = if i.winning_quote == Pubkey::default() {
        None
    } else {
        Some(i.winning_quote.to_string())
    };
    dto::Intent {
        id: row.pubkey.to_string(),
        taker: i.taker.to_string(),
        side,
        base_mint: i.base_mint.to_string(),
        quote_mint: i.quote_mint.to_string(),
        size: i.size,
        limit_price: i.limit_price,
        reveal_deadline: ts_to_utc(i.reveal_deadline),
        resolve_deadline: ts_to_utc(i.resolve_deadline),
        commitment_root: hex::encode(i.commitment_root),
        status,
        winning_quote,
        created_at: row.observed_at,
    }
}

fn quote_to_dto(row: &QuoteRow) -> dto::Quote {
    let q = &row.data;
    dto::Quote {
        id: row.pubkey.to_string(),
        intent_id: q.intent.to_string(),
        maker: q.maker.to_string(),
        commitment: hex::encode(q.commitment),
        revealed_price: q.revealed.then_some(q.revealed_price),
        revealed_size: q.revealed.then_some(q.revealed_size),
        revealed: q.revealed,
        created_at: row.observed_at,
    }
}

fn receipt_to_dto(row: &ReceiptRow) -> dto::Fill {
    let r = &row.data;
    dto::Fill {
        id: row.pubkey.to_string(),
        intent_id: r.intent.to_string(),
        taker: r.taker.to_string(),
        maker: r.maker.to_string(),
        base_mint: r.base_mint.to_string(),
        quote_mint: r.quote_mint.to_string(),
        size: r.filled_size,
        price: r.filled_price,
        tx_signature: row.tx_signature.clone(),
        settled_at: ts_to_utc(r.settled_at),
    }
}

/// Convert a chain `i64` unix timestamp into a `DateTime<Utc>`.
///
/// Logs a warning and falls back to `Utc::now()` when the value is out
/// of the representable range (e.g. 0, negative, or far-future from a
/// buggy program). This makes the substitution detectable in logs
/// rather than silently stamping DTOs with the server's wall time.
fn ts_to_utc(ts: i64) -> DateTime<Utc> {
    match Utc.timestamp_opt(ts, 0).single() {
        Some(dt) => dt,
        None => {
            tracing::warn!(
                raw_timestamp = ts,
                "on-chain timestamp out of range; falling back to Utc::now()"
            );
            Utc::now()
        }
    }
}
