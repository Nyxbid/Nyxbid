use payq_types::*;
use tokio::sync::broadcast;

use crate::AppState;

pub fn seed() -> AppState {
    let agents = vec![
        Agent {
            id: "agent-1".into(),
            name: "Atlas".into(),
            role: AgentRole::Planner,
            status: AgentStatus::Active,
            daily_budget: 50_000_000,
            spent_today: 12_340_000,
        },
        Agent {
            id: "agent-2".into(),
            name: "Sentinel".into(),
            role: AgentRole::Monitor,
            status: AgentStatus::Active,
            daily_budget: 20_000_000,
            spent_today: 4_200_000,
        },
        Agent {
            id: "agent-3".into(),
            name: "Relay".into(),
            role: AgentRole::Executor,
            status: AgentStatus::Idle,
            daily_budget: 100_000_000,
            spent_today: 0,
        },
        Agent {
            id: "agent-4".into(),
            name: "Prism".into(),
            role: AgentRole::Analyst,
            status: AgentStatus::Active,
            daily_budget: 30_000_000,
            spent_today: 8_750_000,
        },
    ];

    let receipts = vec![
        SpendReceipt {
            id: "rcpt-001".into(),
            agent_id: "agent-1".into(),
            agent_name: "Atlas".into(),
            tool: "openai/gpt-4o".into(),
            amount: 2_500_000,
            tx_hash: Some("5Kz...x8Qp".into()),
            status: SpendStatus::Confirmed,
            timestamp: "2026-04-12T08:14:00Z".into(),
            proposal_hash: "a3f1...9c02".into(),
        },
        SpendReceipt {
            id: "rcpt-002".into(),
            agent_id: "agent-4".into(),
            agent_name: "Prism".into(),
            tool: "coingecko/price-feed".into(),
            amount: 500_000,
            tx_hash: Some("3Rw...m7Bk".into()),
            status: SpendStatus::Confirmed,
            timestamp: "2026-04-12T08:12:30Z".into(),
            proposal_hash: "d7e4...1ab8".into(),
        },
        SpendReceipt {
            id: "rcpt-003".into(),
            agent_id: "agent-2".into(),
            agent_name: "Sentinel".into(),
            tool: "helius/rpc-enhanced".into(),
            amount: 1_200_000,
            tx_hash: None,
            status: SpendStatus::Pending,
            timestamp: "2026-04-12T08:16:05Z".into(),
            proposal_hash: "b8c2...f340".into(),
        },
        SpendReceipt {
            id: "rcpt-004".into(),
            agent_id: "agent-1".into(),
            agent_name: "Atlas".into(),
            tool: "anthropic/claude-4".into(),
            amount: 4_100_000,
            tx_hash: Some("9Hq...t2Lz".into()),
            status: SpendStatus::Confirmed,
            timestamp: "2026-04-12T07:58:00Z".into(),
            proposal_hash: "ee01...7d53".into(),
        },
        SpendReceipt {
            id: "rcpt-005".into(),
            agent_id: "agent-4".into(),
            agent_name: "Prism".into(),
            tool: "pyth/price-oracle".into(),
            amount: 750_000,
            tx_hash: Some("2Dn...k5Wp".into()),
            status: SpendStatus::Confirmed,
            timestamp: "2026-04-12T07:45:12Z".into(),
            proposal_hash: "4af9...c128".into(),
        },
    ];

    let policies = vec![
        Policy {
            id: "pol-1".into(),
            name: "Default spend cap".into(),
            daily_limit: 100_000_000,
            per_tx_limit: 10_000_000,
            allowed_tools: vec![
                "openai/*".into(),
                "anthropic/*".into(),
                "coingecko/*".into(),
                "gemini/*".into(),
                "groq/*".into(),
            ],
            active: true,
        },
        Policy {
            id: "pol-2".into(),
            name: "Oracle allowlist".into(),
            daily_limit: 50_000_000,
            per_tx_limit: 5_000_000,
            allowed_tools: vec![
                "pyth/*".into(),
                "switchboard/*".into(),
                "helius/*".into(),
            ],
            active: true,
        },
        Policy {
            id: "pol-3".into(),
            name: "High-value approval".into(),
            daily_limit: 500_000_000,
            per_tx_limit: 50_000_000,
            allowed_tools: vec!["*".into()],
            active: false,
        },
    ];

    let (tx, _) = broadcast::channel(64);

    AppState {
        agents,
        receipts,
        policies,
        solana: None,
        tx,
    }
}
