use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Spend receipt
// ---------------------------------------------------------------------------

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
    /// USDC minor units.
    pub amount: u64,
    pub tx_hash: Option<String>,
    pub status: SpendStatus,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    pub proposal_hash: String,
}

// ---------------------------------------------------------------------------
// Policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    /// USDC minor units.
    pub daily_limit: u64,
    /// USDC minor units.
    pub per_tx_limit: u64,
    pub allowed_tools: Vec<String>,
    pub active: bool,
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    /// USDC minor units.
    pub total_spent_today: u64,
    pub active_agents: u32,
    pub receipts_today: u32,
    pub active_policies: u32,
}

// ---------------------------------------------------------------------------
// API responses
// ---------------------------------------------------------------------------

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
