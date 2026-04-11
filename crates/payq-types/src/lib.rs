//! Shared types for Payq (agentic payments + on-chain receipts).

use serde::{Deserialize, Serialize};

/// API health and version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub service: String,
    pub version: String,
}
