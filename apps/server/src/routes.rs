//! HTTP surface. Read paths come from the chain-indexed [`Store`];
//! write paths are pure tx-prep — the server returns an unsigned legacy
//! `Transaction` for the wallet/agent to sign and broadcast.
//!
//! No route accepts user-supplied off-chain `Intent` data anymore; the
//! old `POST /api/intents` (which seeded an in-memory fake) is gone.

use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::Stream;
use serde::Serialize;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use nyxbid_types::{DashboardStats, Fill, Intent, Market, Quote};

use crate::{
    indexer::ChainEnvelope,
    state::SharedState,
    tx::{
        self, CancelRequest, CreateIntentRequest, ExpireNoMakerRequest,
        ExpireWithMakerRequest, FundMakerEscrowRequest, PreparedTx, RevealQuoteRequest,
        SettleRequest, SubmitQuoteRequest, TxBuildError,
    },
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard))
        .route("/api/markets", get(list_markets))
        .route("/api/intents", get(list_intents))
        .route("/api/intents/{id}", get(get_intent))
        .route("/api/intents/{id}/quotes", get(list_quotes_for_intent))
        .route("/api/fills", get(list_fills))
        .route("/api/tx/create_intent", post(prepare_create_intent))
        .route("/api/tx/submit_quote", post(prepare_submit_quote))
        .route("/api/tx/reveal_quote", post(prepare_reveal_quote))
        .route("/api/tx/fund_maker_escrow", post(prepare_fund_maker_escrow))
        .route("/api/tx/settle", post(prepare_settle))
        .route("/api/tx/cancel", post(prepare_cancel))
        .route("/api/tx/expire_with_maker", post(prepare_expire_with_maker))
        .route("/api/tx/expire_no_maker", post(prepare_expire_no_maker))
        .route("/api/events", get(events))
}

#[derive(Serialize)]
struct Health {
    name: &'static str,
    version: &'static str,
    status: &'static str,
    solana_configured: bool,
    program_id: Option<String>,
    indexer: Option<crate::indexer::IndexerMetricsSnapshot>,
}

async fn health(State(state): State<SharedState>) -> Json<Health> {
    let s = state.read().await;
    Json(Health {
        name: "nyxbid-server",
        version: env!("CARGO_PKG_VERSION"),
        status: "ok",
        solana_configured: s.solana.is_some(),
        program_id: s.solana.as_ref().map(|x| x.program_id.to_string()),
        indexer: s.indexer_metrics.as_ref().map(|m| m.snapshot()),
    })
}

async fn dashboard(State(state): State<SharedState>) -> Json<DashboardStats> {
    let s = state.read().await;
    let store = s.store.read().await;
    Json(store.dashboard_stats())
}

async fn list_markets(State(state): State<SharedState>) -> Json<Vec<Market>> {
    Json(state.read().await.markets.clone())
}

async fn list_intents(State(state): State<SharedState>) -> Json<Vec<Intent>> {
    let s = state.read().await;
    let store = s.store.read().await;
    Json(store.list_intents())
}

async fn get_intent(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<Intent>, StatusCode> {
    let s = state.read().await;
    let store = s.store.read().await;
    store.get_intent(&id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

async fn list_quotes_for_intent(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Json<Vec<Quote>> {
    let s = state.read().await;
    let store = s.store.read().await;
    Json(store.list_quotes_for(&id))
}

async fn list_fills(State(state): State<SharedState>) -> Json<Vec<Fill>> {
    let s = state.read().await;
    let store = s.store.read().await;
    Json(store.list_fills())
}

// ---- tx-prep routes ----------------------------------------------------

async fn prepare_create_intent(
    State(state): State<SharedState>,
    Json(req): Json<CreateIntentRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_create_intent(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

async fn prepare_submit_quote(
    State(state): State<SharedState>,
    Json(req): Json<SubmitQuoteRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_submit_quote(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

async fn prepare_reveal_quote(
    State(state): State<SharedState>,
    Json(req): Json<RevealQuoteRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_reveal_quote(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

async fn prepare_fund_maker_escrow(
    State(state): State<SharedState>,
    Json(req): Json<FundMakerEscrowRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_fund_maker_escrow(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

async fn prepare_settle(
    State(state): State<SharedState>,
    Json(req): Json<SettleRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_settle(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

async fn prepare_cancel(
    State(state): State<SharedState>,
    Json(req): Json<CancelRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_cancel(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

async fn prepare_expire_with_maker(
    State(state): State<SharedState>,
    Json(req): Json<ExpireWithMakerRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_expire_with_maker(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

async fn prepare_expire_no_maker(
    State(state): State<SharedState>,
    Json(req): Json<ExpireNoMakerRequest>,
) -> Result<Json<PreparedTx>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let Some(sol) = s.solana.as_ref() else {
        return Err(solana_unconfigured());
    };
    tx::build_expire_no_maker(sol, req)
        .await
        .map(Json)
        .map_err(map_build_error)
}

fn solana_unconfigured() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(serde_json::json!({
            "error": "solana_unconfigured",
            "message": "set SOLANA_RPC_URL to enable on-chain tx prep"
        })),
    )
}

fn map_build_error(err: TxBuildError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, kind) = match &err {
        TxBuildError::BadPubkey { .. }
        | TxBuildError::BadHex { .. }
        | TxBuildError::WrongLength { .. }
        | TxBuildError::BadSide
        | TxBuildError::ZeroValue
        | TxBuildError::BadDeadlines => (StatusCode::BAD_REQUEST, "bad_request"),
        TxBuildError::Borsh(_) | TxBuildError::Bincode(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "encode_error")
        }
        TxBuildError::Solana(_) => (StatusCode::BAD_GATEWAY, "solana_error"),
    };
    (
        status,
        Json(serde_json::json!({
            "error": kind,
            "message": err.to_string(),
        })),
    )
}

/// SSE stream of decoded chain events. The full WebSocket is added in
/// commit 8; SSE remains here as a no-build-step fallback for browsers
/// and curl-based debugging.
async fn events(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.read().await.chain_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| {
        let env: ChainEnvelope = res.ok()?;
        serde_json::to_string(&env)
            .ok()
            .map(|s| Ok(Event::default().data(s)))
    });
    Sse::new(stream)
}
