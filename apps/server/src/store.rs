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
        let notional_24h = self
            .receipts
            .values()
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

    /// React to a single chain event by re-fetching the touched
    /// account(s). We re-fetch instead of trusting the event payload so
    /// the cache reflects post-handler status transitions.
    pub async fn apply_event(
        &mut self,
        env: &ChainEnvelope,
        sol: &SolanaClient,
    ) -> Result<(), SolanaError> {
        match &env.event {
            ChainEvent::IntentCreated(e) => {
                self.refresh_intent(&e.intent, sol).await?;
            }
            ChainEvent::QuoteSubmitted(e) => {
                self.refresh_quote(&e.quote, sol).await?;
                // Intent doesn't change but UI reads quote count via
                // `list_quotes_for`, so no re-fetch needed.
            }
            ChainEvent::QuoteRevealed(e) => {
                self.refresh_quote(&e.quote, sol).await?;
                self.refresh_intent(&e.intent, sol).await?;
            }
            ChainEvent::AuctionResolved(e) => {
                self.refresh_intent(&e.intent, sol).await?;
                self.refresh_quote(&e.winning_quote, sol).await?;
            }
            ChainEvent::Settled(e) => {
                self.refresh_intent(&e.intent, sol).await?;
                self.refresh_receipt(&e.receipt, Some(env.signature.clone()), sol)
                    .await?;
            }
            ChainEvent::Cancelled(e) => {
                self.refresh_intent(&e.intent, sol).await?;
            }
        }
        Ok(())
    }

    async fn refresh_intent(
        &mut self,
        pk: &Pubkey,
        sol: &SolanaClient,
    ) -> Result<(), SolanaError> {
        match sol.get_anchor_account::<np::state::Intent>(pk).await? {
            Some(data) => {
                let row = IntentRow {
                    pubkey: *pk,
                    data,
                    observed_at: self
                        .intents
                        .get(pk)
                        .map(|r| r.observed_at)
                        .unwrap_or_else(Utc::now),
                };
                self.intents.insert(*pk, row);
            }
            None => {
                // Account was closed (cancel/expire). Drop from cache.
                self.intents.remove(pk);
            }
        }
        Ok(())
    }

    async fn refresh_quote(
        &mut self,
        pk: &Pubkey,
        sol: &SolanaClient,
    ) -> Result<(), SolanaError> {
        match sol.get_anchor_account::<np::state::Quote>(pk).await? {
            Some(data) => {
                let row = QuoteRow {
                    pubkey: *pk,
                    data,
                    observed_at: self
                        .quotes
                        .get(pk)
                        .map(|r| r.observed_at)
                        .unwrap_or_else(Utc::now),
                };
                self.quotes.insert(*pk, row);
            }
            None => {
                self.quotes.remove(pk);
            }
        }
        Ok(())
    }

    async fn refresh_receipt(
        &mut self,
        pk: &Pubkey,
        signature: Option<String>,
        sol: &SolanaClient,
    ) -> Result<(), SolanaError> {
        if let Some(data) = sol.get_anchor_account::<np::state::Receipt>(pk).await? {
            self.receipts.insert(
                *pk,
                ReceiptRow {
                    pubkey: *pk,
                    data,
                    tx_signature: signature,
                },
            );
        }
        Ok(())
    }
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
fn ts_to_utc(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(Utc::now)
}
