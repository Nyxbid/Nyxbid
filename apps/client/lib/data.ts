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

export interface DashboardResponse {
  stats: DashboardStats;
  recent_receipts: SpendReceipt[];
}
