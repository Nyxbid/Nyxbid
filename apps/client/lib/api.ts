// Isomorphic fetch helper. The browser hits NEXT_PUBLIC_API_URL (the
// publicly reachable origin); Next SSR/ISR runs *inside* the client
// container in Docker, so it must use INTERNAL_API_URL to reach the
// server service over Docker DNS. Outside Docker both fall back to
// NEXT_PUBLIC_API_URL, which is normally http://localhost:8080.

import type { PreparedTx } from "@/lib/data";

const PUBLIC_API_URL =
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

const INTERNAL_API_URL =
  process.env.INTERNAL_API_URL ?? PUBLIC_API_URL;

/** Choose the right base URL depending on where the call runs. */
function baseUrl(): string {
  return typeof window === "undefined" ? INTERNAL_API_URL : PUBLIC_API_URL;
}

/** Origin the *browser* should use. Always public, even on the server. */
export function publicApiUrl(): string {
  return PUBLIC_API_URL;
}

export class ApiError extends Error {
  status: number;
  payload?: unknown;
  constructor(status: number, message: string, payload?: unknown) {
    super(message);
    this.status = status;
    this.payload = payload;
  }
}

async function parseJsonOrThrow<T>(res: Response): Promise<T> {
  if (!res.ok) {
    let payload: unknown;
    try {
      payload = await res.json();
    } catch {
      payload = await res.text();
    }
    const message =
      typeof payload === "object" && payload && "message" in payload
        ? String((payload as { message: unknown }).message)
        : `${res.status} ${res.statusText}`;
    throw new ApiError(res.status, message, payload);
  }
  return res.json() as Promise<T>;
}

export async function fetchJson<T>(
  path: string,
  init?: RequestInit,
): Promise<T> {
  const res = await fetch(`${baseUrl()}${path}`, {
    cache: "no-store",
    ...init,
  });
  return parseJsonOrThrow<T>(res);
}

export async function postJson<T, B = unknown>(
  path: string,
  body: B,
): Promise<T> {
  const res = await fetch(`${baseUrl()}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return parseJsonOrThrow<T>(res);
}

// ----- /api/tx/* prep helpers --------------------------------------------

export interface CreateIntentBody {
  taker: string;
  side: "buy" | "sell";
  base_mint: string;
  quote_mint: string;
  size: number;
  limit_price: number;
  reveal_deadline: number;
  resolve_deadline: number;
  settle_deadline: number;
  commitment_root_hex?: string;
  nonce_hex: string;
}

export interface SubmitQuoteBody {
  maker: string;
  intent: string;
  commitment_hex: string;
  nonce_hex: string;
}

export interface RevealQuoteBody {
  maker: string;
  intent: string;
  quote: string;
  revealed_price: number;
  revealed_size: number;
  commit_nonce_hex: string;
}

export interface FundMakerEscrowBody {
  maker: string;
  intent: string;
  quote: string;
  amount: number;
}

export interface SettleBody {
  payer: string;
  intent: string;
}

export interface CancelBody {
  taker: string;
  intent: string;
}

export interface ExpireBody {
  payer: string;
  intent: string;
}

export const tx = {
  createIntent: (b: CreateIntentBody) =>
    postJson<PreparedTx, CreateIntentBody>("/api/tx/create_intent", b),
  submitQuote: (b: SubmitQuoteBody) =>
    postJson<PreparedTx, SubmitQuoteBody>("/api/tx/submit_quote", b),
  revealQuote: (b: RevealQuoteBody) =>
    postJson<PreparedTx, RevealQuoteBody>("/api/tx/reveal_quote", b),
  fundMakerEscrow: (b: FundMakerEscrowBody) =>
    postJson<PreparedTx, FundMakerEscrowBody>(
      "/api/tx/fund_maker_escrow",
      b,
    ),
  settle: (b: SettleBody) =>
    postJson<PreparedTx, SettleBody>("/api/tx/settle", b),
  cancel: (b: CancelBody) =>
    postJson<PreparedTx, CancelBody>("/api/tx/cancel", b),
  expireWithMaker: (b: ExpireBody) =>
    postJson<PreparedTx, ExpireBody>("/api/tx/expire_with_maker", b),
  expireNoMaker: (b: ExpireBody) =>
    postJson<PreparedTx, ExpireBody>("/api/tx/expire_no_maker", b),
};

/** Open a SSE connection. Browser-only — has no SSR meaning. */
export function eventSourceUrl(path = "/api/events"): string {
  return `${PUBLIC_API_URL}${path}`;
}

/** Convert NEXT_PUBLIC_API_URL to its WebSocket equivalent. */
export function websocketUrl(path = "/ws"): string {
  const u = new URL(PUBLIC_API_URL);
  u.protocol = u.protocol === "https:" ? "wss:" : "ws:";
  return `${u.toString().replace(/\/$/, "")}${path}`;
}
