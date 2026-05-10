// Wire types shared with `crates/nyxbid-types`. Mirror them by hand so
// the client has no Rust toolchain dependency. Field names match the
// snake_case JSON the server emits.

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
  /** Base mint decimals (e.g. WSOL = 9). */
  base_decimals: number;
  /** Quote mint decimals (e.g. USDC = 6). */
  quote_decimals: number;
}

export interface DashboardStats {
  open_intents: number;
  resolved_intents: number;
  total_fills: number;
  notional_24h: number;
  avg_makers_per_intent: number;
}

// ----- live wire events ---------------------------------------------------
//
// The server's `ChainEvent` enum is serialised with a tagged
// representation (`#[serde(tag = "kind", rename_all = "snake_case")]`)
// so the on-the-wire JSON looks like:
//   { "kind": "intent_created", "intent": "...", "taker": "...", ... }
// And every event is wrapped in a `ChainEnvelope` that adds the tx
// signature + slot.
//
// Naming on the client mirrors the server module structure: `ChainEvent`
// is the discriminated payload, `ChainEnvelope` is the wrapped form
// that comes off `/api/events` (SSE) and `/ws` (WebSocket).

export type ChainEvent =
  | {
      kind: "intent_created";
      intent: string;
      taker: string;
      side: number;
      size: number;
      limit_price: number;
      reveal_deadline: number;
    }
  | {
      kind: "quote_submitted";
      intent: string;
      quote: string;
      maker: string;
    }
  | {
      kind: "quote_revealed";
      intent: string;
      quote: string;
      maker: string;
      revealed_price: number;
      revealed_size: number;
      is_best: boolean;
    }
  | {
      kind: "auction_resolved";
      intent: string;
      winning_quote: string;
      clearing_price: number;
      filled_size: number;
    }
  | {
      kind: "settled";
      intent: string;
      receipt: string;
      maker: string;
      taker: string;
      filled_price: number;
      filled_size: number;
    }
  | {
      kind: "cancelled";
      intent: string;
      reason: number;
    };

export interface ChainEnvelope {
  signature: string;
  slot: number;
  event: ChainEvent;
}

/** Envelope sent by the WS handler when greeting a new client. */
export interface WsHello {
  kind: "hello";
}

/** Envelope sent by the WS handler when the broadcast queue dropped events. */
export interface WsLagged {
  kind: "lagged";
  skipped: number;
}

export type WsMessage = ChainEnvelope | WsHello | WsLagged;

export function isChainEnvelope(m: WsMessage): m is ChainEnvelope {
  return (m as ChainEnvelope).event !== undefined;
}

// ----- prepared-tx wire shape (mirror of `tx::PreparedTx`) ---------------

export interface PreparedAccounts {
  intent?: string;
  escrow?: string;
  taker_vault?: string;
  maker_vault?: string;
  quote?: string;
  receipt?: string;
  reputation?: string;
}

export interface PreparedTx {
  tx_base64: string;
  message_base64: string;
  blockhash: string;
  last_valid_block_height: number;
  fee_payer: string;
  accounts: PreparedAccounts;
}
