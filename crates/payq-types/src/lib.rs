use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Planner,
    Executor,
    Analyst,
    Monitor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Active,
    Idle,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub status: AgentStatus,
    /// USDC minor units (6 decimals). 1 USDC = 1_000_000.
    pub daily_budget: u64,
    pub spent_today: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SpendStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendReceipt {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub tool: String,
    pub amount: u64,
    pub tx_hash: Option<String>,
    pub status: SpendStatus,
    pub timestamp: String,
    pub proposal_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub daily_limit: u64,
    pub per_tx_limit: u64,
    pub allowed_tools: Vec<String>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_spent_today: u64,
    pub active_agents: u32,
    pub receipts_today: u32,
    pub active_policies: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub service: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardResponse {
    pub stats: DashboardStats,
    pub recent_receipts: Vec<SpendReceipt>,
}

/// POST /api/proposals request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalRequest {
    pub agent_id: String,
    pub tool: String,
    pub prompt: String,
}

/// POST /api/proposals response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalResponse {
    pub receipt: SpendReceipt,
    pub tool_response: String,
}
