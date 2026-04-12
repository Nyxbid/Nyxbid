/**
 * Mock data matching payq-types shapes.
 * Replace with `fetch("http://localhost:3001/api/...")` when wiring the server.
 */

export interface Agent {
  id: string;
  name: string;
  role: string;
  status: string;
  daily_budget: number;
  spent_today: number;
}

export interface SpendReceipt {
  id: string;
  agent_id: string;
  agent_name: string;
  tool: string;
  amount: number;
  tx_hash: string | null;
  status: string;
  timestamp: string;
  proposal_hash: string;
}

export interface Policy {
  id: string;
  name: string;
  daily_limit: number;
  per_tx_limit: number;
  allowed_tools: string[];
  active: boolean;
}

export interface DashboardStats {
  total_spent_today: number;
  active_agents: number;
  receipts_today: number;
  active_policies: number;
}

// ── Data ────────────────────────────────────────────────────────────────

export const agents: Agent[] = [
  { id: "agent-1", name: "Atlas", role: "planner", status: "active", daily_budget: 50_000_000, spent_today: 12_340_000 },
  { id: "agent-2", name: "Sentinel", role: "monitor", status: "active", daily_budget: 20_000_000, spent_today: 4_200_000 },
  { id: "agent-3", name: "Relay", role: "executor", status: "idle", daily_budget: 100_000_000, spent_today: 0 },
  { id: "agent-4", name: "Prism", role: "analyst", status: "active", daily_budget: 30_000_000, spent_today: 8_750_000 },
];

export const receipts: SpendReceipt[] = [
  { id: "rcpt-001", agent_id: "agent-1", agent_name: "Atlas", tool: "openai/gpt-4o", amount: 2_500_000, tx_hash: "5Kz...x8Qp", status: "confirmed", timestamp: "2026-04-12T08:14:00Z", proposal_hash: "a3f1...9c02" },
  { id: "rcpt-002", agent_id: "agent-4", agent_name: "Prism", tool: "coingecko/price-feed", amount: 500_000, tx_hash: "3Rw...m7Bk", status: "confirmed", timestamp: "2026-04-12T08:12:30Z", proposal_hash: "d7e4...1ab8" },
  { id: "rcpt-003", agent_id: "agent-2", agent_name: "Sentinel", tool: "helius/rpc-enhanced", amount: 1_200_000, tx_hash: null, status: "pending", timestamp: "2026-04-12T08:16:05Z", proposal_hash: "b8c2...f340" },
  { id: "rcpt-004", agent_id: "agent-1", agent_name: "Atlas", tool: "anthropic/claude-4", amount: 4_100_000, tx_hash: "9Hq...t2Lz", status: "confirmed", timestamp: "2026-04-12T07:58:00Z", proposal_hash: "ee01...7d53" },
  { id: "rcpt-005", agent_id: "agent-4", agent_name: "Prism", tool: "pyth/price-oracle", amount: 750_000, tx_hash: "2Dn...k5Wp", status: "confirmed", timestamp: "2026-04-12T07:45:12Z", proposal_hash: "4af9...c128" },
];

export const policies: Policy[] = [
  { id: "pol-1", name: "Default spend cap", daily_limit: 100_000_000, per_tx_limit: 10_000_000, allowed_tools: ["openai/*", "anthropic/*", "coingecko/*"], active: true },
  { id: "pol-2", name: "Oracle allowlist", daily_limit: 50_000_000, per_tx_limit: 5_000_000, allowed_tools: ["pyth/*", "switchboard/*", "helius/*"], active: true },
  { id: "pol-3", name: "High-value approval", daily_limit: 500_000_000, per_tx_limit: 50_000_000, allowed_tools: ["*"], active: false },
];

export const dashboardStats: DashboardStats = {
  total_spent_today: agents.reduce((s, a) => s + a.spent_today, 0),
  active_agents: agents.filter((a) => a.status === "active").length,
  receipts_today: receipts.length,
  active_policies: policies.filter((p) => p.active).length,
};
