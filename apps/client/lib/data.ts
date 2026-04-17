export type Side = "buy" | "sell";

export type IntentStatus =
  | "open"
  | "resolved"
  | "settled"
  | "cancelled"
  | "expired";

export interface Intent {
  id: string;
  taker: string;
  side: Side;
  base_mint: string;
  quote_mint: string;
  size: number;
  limit_price: number;
  reveal_deadline: string;
  resolve_deadline: string;
  commitment_root: string;
  status: IntentStatus;
  winning_quote: string | null;
  created_at: string;
}

export interface Quote {
  id: string;
  intent_id: string;
  maker: string;
  commitment: string;
  revealed_price: number | null;
  revealed_size: number | null;
  revealed: boolean;
  created_at: string;
}

export interface Fill {
  id: string;
  intent_id: string;
  taker: string;
  maker: string;
  base_mint: string;
  quote_mint: string;
  size: number;
  price: number;
  tx_signature: string | null;
  settled_at: string;
}

export interface Market {
  symbol: string;
  base_mint: string;
  quote_mint: string;
  min_size: number;
}

export interface DashboardStats {
  open_intents: number;
  resolved_intents: number;
  total_fills: number;
  notional_24h: number;
  avg_makers_per_intent: number;
}

export type StreamEvent =
  | { type: "intent_created"; value: Intent }
  | { type: "quote_submitted"; value: Quote }
  | { type: "auction_resolved"; intent_id: string }
  | { type: "filled"; value: Fill };
