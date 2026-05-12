//! HTTP surface. Read paths come from the chain-indexed [`Store`];
//! write paths are pure tx-prep — the server returns an unsigned legacy
//! `Transaction` for the wallet/agent to sign and broadcast.
//!
//! No route accepts user-supplied off-chain `Intent` data anymore; the
//! old `POST /api/intents` (which seeded an in-memory fake) is gone.

use std::convert::Infallible;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures_util::Stream;
use serde::Serialize;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use nyxbid_types::{DashboardStats, Fill, Intent, Market, Quote};

use crate::{
    indexer::ChainEnvelope,
    solana::SolanaClient,
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
        .route("/.well-known/agent-card.json", get(crate::a2a::agent_card))
        .route("/.well-known/jwks.json", get(crate::a2a::jwks))
        .route(
            "/agent/authenticatedExtendedCard",
            get(crate::a2a::extended_agent_card),
        )
        .route(crate::a2a::A2A_RPC_PATH, post(crate::a2a::rpc_handler))
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
        .route("/api/tx/send", post(send_tx))
        .route("/api/tx/simulate", post(simulate_tx))
        .route("/api/tx/status/{signature}", get(tx_status))
        .route("/api/events", get(events))
        .route("/ws", get(ws_upgrade))
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
            "message": "Solana RPC is not configured on the server. \
                        Set SOLANA_RPC_URL in apps/server/.env (or the \
                        workspace-root .env) and restart the server."
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

// ---- relay + simulate + status ----------------------------------------

#[derive(serde::Deserialize)]
struct SignedTxBody {
    /// Base64-encoded, fully signed legacy `Transaction`. Same shape as
    /// what `/api/tx/*` returns after the wallet adds its signature(s).
    tx_base64: String,
}

/// Decode a base64-encoded transaction body into raw bytes.
fn decode_tx(body: &SignedTxBody) -> Result<Vec<u8>, (StatusCode, Json<serde_json::Value>)> {
    B64.decode(&body.tx_base64).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "bad_base64",
                "message": e.to_string(),
            })),
        )
    })
}

fn use_solana<'a>(
    s: &'a tokio::sync::RwLockReadGuard<'a, crate::state::AppState>,
) -> Result<&'a SolanaClient, (StatusCode, Json<serde_json::Value>)> {
    s.solana.as_ref().ok_or_else(solana_unconfigured)
}

/// Relay a signed transaction to the cluster. The server never holds
/// keys; this is purely a network convenience for clients that don't
/// want their own RPC connection.
async fn send_tx(
    State(state): State<SharedState>,
    Json(body): Json<SignedTxBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let bytes = decode_tx(&body)?;
    let s = state.read().await;
    let sol = use_solana(&s)?;
    match sol.send_signed_transaction(&bytes).await {
        Ok(sig) => Ok(Json(serde_json::json!({ "signature": sig.to_string() }))),
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": "send_failed",
                "message": e.to_string(),
            })),
        )),
    }
}

/// Simulate a signed transaction so the wallet can preview program
/// logs + compute units before broadcasting.
async fn simulate_tx(
    State(state): State<SharedState>,
    Json(body): Json<SignedTxBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let bytes = decode_tx(&body)?;
    let s = state.read().await;
    let sol = use_solana(&s)?;
    match sol.simulate_signed_transaction(&bytes).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "err": result.err.map(|e| format!("{e:?}")),
            "logs": result.logs.unwrap_or_default(),
            "units_consumed": result.units_consumed,
        }))),
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": "simulate_failed",
                "message": e.to_string(),
            })),
        )),
    }
}

/// Coarse status of a transaction by signature. The wallet UI polls
/// this to render "pending -> processed -> confirmed -> finalized".
async fn tx_status(
    State(state): State<SharedState>,
    Path(signature): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let s = state.read().await;
    let sol = use_solana(&s)?;
    let sig = SolanaClient::parse_signature(&signature).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "bad_signature",
                "message": e.to_string(),
            })),
        )
    })?;
    match sol.tx_status(&sig).await {
        Ok(status) => Ok(Json(serde_json::json!({
            "signature": signature,
            "status": status,
        }))),
        Err(e) => Err((
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({
                "error": "rpc_error",
                "message": e.to_string(),
            })),
        )),
    }
}

// ---- websocket --------------------------------------------------------

/// `GET /ws`: upgrade to WebSocket, then push every chain event as a
/// JSON message. The browser side gets push-based updates without the
/// reconnection ergonomics of SSE EventSource.
///
/// Subscribes to `ui_tx` rather than `chain_tx` so each event delivered
/// here is guaranteed to be visible in the indexed REST store. Without
/// this, a client that refetches `/api/...` on receipt of the event
/// would race the state-apply task and frequently see stale data — the
/// "I had to refresh after settle/fund/bid" bug.
async fn ws_upgrade(
    State(state): State<SharedState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let rx = state.read().await.ui_tx.subscribe();
    ws.on_upgrade(move |socket| ws_pump(socket, rx))
}

async fn ws_pump(
    mut socket: WebSocket,
    mut rx: tokio::sync::broadcast::Receiver<ChainEnvelope>,
) {
    // Greet the client so it knows the connection is alive even before
    // the next chain event arrives.
    let hello = serde_json::json!({ "kind": "hello" });
    if socket
        .send(Message::Text(hello.to_string().into()))
        .await
        .is_err()
    {
        return;
    }
    loop {
        tokio::select! {
            // Outbound: chain events -> client.
            msg = rx.recv() => match msg {
                Ok(env) => {
                    let json = match serde_json::to_string(&env) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    let warning = serde_json::json!({
                        "kind": "lagged",
                        "skipped": n,
                    });
                    if socket
                        .send(Message::Text(warning.to_string().into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            // Inbound: react to client-side close/ping.
            client_msg = socket.recv() => match client_msg {
                Some(Ok(Message::Close(_))) | None => break,
                Some(Ok(Message::Ping(p))) => {
                    let _ = socket.send(Message::Pong(p)).await;
                }
                Some(Ok(_)) => { /* ignore other inbound frames */ }
                Some(Err(_)) => break,
            }
        }
    }
}

/// SSE stream of decoded chain events. The full WebSocket is at /ws;
/// SSE remains as a no-build-step fallback for browsers and curl-based
/// debugging.
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
