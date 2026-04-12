use std::convert::Infallible;

use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use futures_util::{stream::Stream, StreamExt};
use payq_types::*;
use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;
use uuid::Uuid;

use crate::{x402, SharedState};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/health", get(health))
        .route("/api/dashboard", get(dashboard))
        .route("/api/agents", get(agents))
        .route("/api/receipts", get(receipts))
        .route("/api/policies", get(policies))
        .route("/api/proposals", post(create_proposal))
        .route("/api/events", get(sse_handler))
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "payq-server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

async fn dashboard(State(state): State<SharedState>) -> Json<DashboardResponse> {
    let s = state.read().await;
    let active_agents = s.agents.iter().filter(|a| a.status == AgentStatus::Active).count() as u32;
    let active_policies = s.policies.iter().filter(|p| p.active).count() as u32;
    let total_spent_today: u64 = s.agents.iter().map(|a| a.spent_today).sum();
    let receipts_today = s.receipts.len() as u32;

    Json(DashboardResponse {
        stats: DashboardStats {
            total_spent_today,
            active_agents,
            receipts_today,
            active_policies,
        },
        recent_receipts: s.receipts.clone(),
    })
}

async fn agents(State(state): State<SharedState>) -> Json<Vec<Agent>> {
    Json(state.read().await.agents.clone())
}

async fn receipts(State(state): State<SharedState>) -> Json<Vec<SpendReceipt>> {
    Json(state.read().await.receipts.clone())
}

async fn policies(State(state): State<SharedState>) -> Json<Vec<Policy>> {
    Json(state.read().await.policies.clone())
}

async fn create_proposal(
    State(state): State<SharedState>,
    Json(req): Json<ProposalRequest>,
) -> Result<Json<ProposalResponse>, (StatusCode, String)> {
    let (agent, policy) = {
        let s = state.read().await;

        let agent = s
            .agents
            .iter()
            .find(|a| a.id == req.agent_id)
            .cloned()
            .ok_or((StatusCode::NOT_FOUND, format!("agent {} not found", req.agent_id)))?;

        let policy = s
            .policies
            .iter()
            .find(|p| p.active && p.allowed_tools.iter().any(|pat| tool_matches(pat, &req.tool)))
            .cloned()
            .ok_or((StatusCode::FORBIDDEN, "no active policy allows this tool".into()))?;

        (agent, policy)
    };

    let tool_result = x402::call_tool(&req.tool, &req.prompt)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("tool call failed: {e}")))?;

    let amount = tool_result.cost;

    if amount > policy.per_tx_limit {
        return Err((StatusCode::FORBIDDEN, "amount exceeds per-tx limit".into()));
    }

    let proposal_hash_bytes: [u8; 32] = {
        let mut h = Sha256::new();
        h.update(req.agent_id.as_bytes());
        h.update(req.tool.as_bytes());
        h.update(req.prompt.as_bytes());
        h.update(Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        h.finalize().into()
    };
    let proposal_hash_hex = hex::encode(proposal_hash_bytes);

    let receipt_id = format!("rcpt-{}", &Uuid::new_v4().to_string()[..8]);
    let now = Utc::now().to_rfc3339();

    // Try recording on-chain
    let tx_hash = {
        let s = state.read().await;
        if let Some(ref sol) = s.solana {
            let vault_pubkey = std::env::var("PAYQ_VAULT_PUBKEY")
                .ok()
                .and_then(|v| v.parse::<Pubkey>().ok());

            if let Some(vault) = vault_pubkey {
                match sol
                    .record_spend(
                        vault,
                        req.agent_id.clone(),
                        req.tool.clone(),
                        amount,
                        proposal_hash_bytes,
                    )
                    .await
                {
                    Ok(sig) => {
                        tracing::info!(%sig, "spend recorded on-chain");
                        Some(sig.to_string())
                    }
                    Err(e) => {
                        tracing::error!(err = %e, "on-chain record_spend failed");
                        None
                    }
                }
            } else {
                tracing::warn!("PAYQ_VAULT_PUBKEY not set, skipping on-chain recording");
                None
            }
        } else {
            None
        }
    };

    let status = if tx_hash.is_some() {
        SpendStatus::Confirmed
    } else {
        SpendStatus::Pending
    };

    let receipt = SpendReceipt {
        id: receipt_id,
        agent_id: agent.id.clone(),
        agent_name: agent.name.clone(),
        tool: req.tool.clone(),
        amount,
        tx_hash,
        status,
        timestamp: now,
        proposal_hash: proposal_hash_hex,
    };

    {
        let mut s = state.write().await;
        if let Some(a) = s.agents.iter_mut().find(|a| a.id == agent.id) {
            a.spent_today += amount;
        }
        s.receipts.insert(0, receipt.clone());
        let _ = s.tx.send(receipt.clone());
    }

    Ok(Json(ProposalResponse {
        receipt,
        tool_response: tool_result.body,
    }))
}

async fn sse_handler(
    State(state): State<SharedState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.read().await.tx.subscribe();

    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(|result| {
        futures_util::future::ready(match result {
            Ok(receipt) => {
                let json = serde_json::to_string(&receipt).unwrap_or_default();
                Some(Ok(Event::default().event("receipt").data(json)))
            }
            Err(_) => None,
        })
    });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn tool_matches(pattern: &str, tool: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix("/*") {
        return tool.starts_with(prefix);
    }
    pattern == tool
}
