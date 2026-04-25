use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::Stream;
use serde::Serialize;
use std::convert::Infallible;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};

use nyxbid_types::{DashboardStats, Fill, Intent, Market, Quote};

use crate::{intent, state::SharedState};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard))
        .route("/api/markets", get(list_markets))
        .route("/api/intents", get(list_intents).post(create_intent))
        .route("/api/intents/{id}", get(get_intent))
        .route("/api/intents/{id}/quotes", get(list_quotes_for_intent))
        .route("/api/fills", get(list_fills))
        .route("/api/events", get(events))
}

#[derive(Serialize)]
struct Health {
    name: &'static str,
    version: &'static str,
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        name: "nyxbid-server",
        version: env!("CARGO_PKG_VERSION"),
        status: "ok",
    })
}

async fn dashboard(State(state): State<SharedState>) -> Json<DashboardStats> {
    let s = state.read().await;
    let open = s
        .intents
        .iter()
        .filter(|i| matches!(i.status, nyxbid_types::IntentStatus::Open))
        .count() as u64;
    let resolved = s
        .intents
        .iter()
        .filter(|i| matches!(i.status, nyxbid_types::IntentStatus::Resolved))
        .count() as u64;
    let notional: u64 = s.fills.iter().map(|f| f.size * f.price / 1_000_000).sum();
    let avg = if s.intents.is_empty() {
        0.0
    } else {
        s.quotes.len() as f64 / s.intents.len() as f64
    };
    Json(DashboardStats {
        open_intents: open,
        resolved_intents: resolved,
        total_fills: s.fills.len() as u64,
        notional_24h: notional,
        avg_makers_per_intent: avg,
    })
}

async fn list_markets(State(state): State<SharedState>) -> Json<Vec<Market>> {
    Json(state.read().await.markets.clone())
}

async fn list_intents(State(state): State<SharedState>) -> Json<Vec<Intent>> {
    Json(state.read().await.intents.clone())
}

async fn get_intent(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<Intent>, StatusCode> {
    state
        .read()
        .await
        .intents
        .iter()
        .find(|i| i.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn list_quotes_for_intent(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Json<Vec<Quote>> {
    let s = state.read().await;
    Json(s.quotes.iter().filter(|q| q.intent_id == id).cloned().collect())
}

async fn list_fills(State(state): State<SharedState>) -> Json<Vec<Fill>> {
    Json(state.read().await.fills.clone())
}

async fn create_intent(
    State(state): State<SharedState>,
    Json(req): Json<intent::CreateIntentRequest>,
) -> impl IntoResponse {
    let new_intent = intent::build_intent(req);
    let mut s = state.write().await;
    s.intents.push(new_intent.clone());
    let _ = s.tx.send(crate::state::StreamEvent::IntentCreated(new_intent.clone()));
    (StatusCode::CREATED, Json(new_intent))
}

async fn events(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.read().await.tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|res| {
        res.ok().and_then(|ev| {
            serde_json::to_string(&ev)
                .ok()
                .map(|s| Ok(Event::default().data(s)))
        })
    });
    Sse::new(stream)
}
