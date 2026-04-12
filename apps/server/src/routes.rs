use axum::{extract::State, routing::get, Json, Router};
use payq_types::*;

use crate::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard))
        .route("/api/agents", get(agents))
        .route("/api/receipts", get(receipts))
        .route("/api/policies", get(policies))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "payq-server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

async fn dashboard(State(state): State<SharedState>) -> Json<DashboardResponse> {
    let active_agents = state.agents.iter().filter(|a| a.status == AgentStatus::Active).count() as u32;
    let active_policies = state.policies.iter().filter(|p| p.active).count() as u32;
    let total_spent_today: u64 = state.agents.iter().map(|a| a.spent_today).sum();
    let receipts_today = state.receipts.len() as u32;

    Json(DashboardResponse {
        stats: DashboardStats {
            total_spent_today,
            active_agents,
            receipts_today,
            active_policies,
        },
        recent_receipts: state.receipts.clone(),
    })
}

async fn agents(State(state): State<SharedState>) -> Json<Vec<Agent>> {
    Json(state.agents.clone())
}

async fn receipts(State(state): State<SharedState>) -> Json<Vec<SpendReceipt>> {
    Json(state.receipts.clone())
}

async fn policies(State(state): State<SharedState>) -> Json<Vec<Policy>> {
    Json(state.policies.clone())
}
