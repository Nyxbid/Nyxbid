# Nyxbid — Project Spec

> Private, sealed-bid RFQ venue for OTC-size trades on Solana.
> Atomic settlement on chain. Agent-native via the Model Context Protocol.
>
> Internal chain/program name: `nyxbid`.
> Public surface name: `nyxbid`, at `nyxbid.trade`.

---

## 0. TL;DR

Large trades ("OTC-size") lose 50–400 bps of price quality on public Solana
books: the intent leaks before the fill, liquidity fragments across venues,
and MEV searchers extract the difference. Nyxbid is a sealed-bid RFQ venue
where takers post an **intent PDA**, makers answer with **hash-committed
quotes**, and a winning quote settles **atomically** via HTLC-style escrow.

Three properties make Nyxbid different from anything shipped on Solana today:

1. **Pre-trade privacy by construction** — the book is never visible, even
   to the venue operator, until reveal.
2. **Atomic settlement** — no half-fills, no off-chain trust, no
   "quote, confirm, pay" round-trip.
3. **Agent-native surface** — the entire lifecycle (`create_intent`,
   `submit_quote`, `list_quotes`, `settle`, `cancel`) is exposed as
   MCP tools, so any MCP-capable agent (Claude, GPT, Gemini) can trade
   directly without bespoke adapters.

Nyxbid is not a DEX aggregator, not an AMM, not a CLOB, and not a dark pool
in the TradFi sense. It is a **Solana-native, RFQ venue optimised for
trade sizes where public books are the wrong tool**.

---

## 1. Product definition

### 1.1 Who it's for

- **Taker side**
  - **Agents** running cash-management or rebalance strategies that need
    to move 5 to 500-figure notionals without leaking intent.
  - **Treasuries / funds** moving inventory between base/quote pairs.
  - **Strategy vaults** doing periodic rebalances larger than the top of
    book of any single Solana venue.
- **Maker side**
  - **Market-making firms** who want latent, off-book inventory demand.
  - **Agents** running a quoting strategy against a set of
    observed intents.
  - **Prop desks** looking for size without paying a book-taker fee.

### 1.2 The job users hire Nyxbid for

> "I need to fill <X> of <Y> at a price no worse than <Z>, without
> telling the whole market first."

That job is structurally underserved on Solana today.

### 1.3 Scope boundaries (v0)

- **In scope**: SPL token to SPL token (starting with SOL/USDC),
  sealed-bid RFQ, atomic settlement in a single tx, MCP surface, dashboard.
- **Out of scope (later)**: cross-chain legs, options, perps, margin,
  on-chain maker reputation, KYC, fiat rails.

---

## 2. Architecture

### 2.1 Components

```
┌───────────────┐           ┌──────────────────┐           ┌──────────────┐
│  taker        │──intent──▶│                  │──tx──────▶│  nyxbid       │
│  (agent/UI)   │           │  nyxbid-server   │           │  Anchor       │
└───────────────┘           │  (Rust / Axum)   │◀──events──│  program      │
                            │                  │           └──────────────┘
┌───────────────┐           │  intent  auction │           Intent / Quote /
│  maker agents │──quotes──▶│  mcp     solana  │           Escrow / Receipt
│  (MCP / HTTP) │◀──quotes──│                  │
└───────────────┘           └────────┬─────────┘
                                     │ SSE
                            ┌────────▼─────────┐
                            │  nyxbid-client    │
                            │  (Next.js)        │
                            └──────────────────┘
```

- **Anchor program `nyxbid`** — source of truth. Owns four account types
  (`Intent`, `Quote`, `Escrow`, `Receipt`) and five instructions
  (`create_intent`, `submit_quote`, `resolve_auction`, `settle`, `cancel`).
  Settlement is atomic; no off-chain custody.
- **`nyxbid-server` (Rust / Axum)** — a stateless coordinator. It indexes
  program events, serves the REST + SSE API, exposes the MCP surface,
  and can act as a permissionless resolver (anyone can resolve an expired
  intent — this is just a convenience, not a trust anchor).
- **`nyxbid-client` (Next.js)** — surface for operators and takers without
  an agent. Landing at `/`, docs at `/docs`, app at `/dashboard`.
- **`nyxbid-types`** — shared Rust types, wire format and for the server.

### 2.2 Trust model

- **On-chain**: intent, quote commitment, reveal, resolve, settle. No
  trust in the server for price quality or execution.
- **Off-chain (server)**: just ordering, indexing, and MCP tool surface.
  A malicious server can censor or reorder, but it cannot steal or
  frontrun — that's guaranteed by the program.
- **Privacy model**: pre-reveal, quote prices are only present as
  `commitment = H(price || size || nonce)`. Post-reveal, everything is
  visible on chain. This is *pre-trade privacy*, not post-trade privacy;
  it closes the MEV/frontrunning window, not the audit trail.

### 2.3 Process model

- One `nyxbid-server` binary. Stateless apart from in-memory
  auction state + SSE fanout. Crash-safe because Solana is the source
  of truth on restart.
- Single Anchor program deployed on devnet first, then mainnet-beta.
- Next.js standalone build served behind Caddy.

---

## 3. Repo layout

```
/
├── apps/
│   ├── server/                nyxbid-server (Rust / Axum)
│   │   ├── Cargo.toml
│   │   ├── Dockerfile         distroless runtime
│   │   └── src/
│   │       ├── main.rs        entry, state init
│   │       ├── state.rs       AppState + StreamEvent
│   │       ├── routes.rs      REST + SSE
│   │       ├── intent.rs      request shape, commitment hashing
│   │       ├── auction.rs     winner selection, limit checks
│   │       ├── mcp.rs         MCP tool surface
│   │       └── solana.rs      RPC client, program IDs
│   │
│   └── client/                nyxbid-client (Next.js)
│       ├── Dockerfile
│       └── app/
│           ├── (marketing)/
│           │   ├── layout.tsx
│           │   ├── page.tsx   landing
│           │   └── docs/
│           └── (dashboard)/
│               ├── layout.tsx
│               ├── dashboard/
│               ├── intents/
│               ├── quotes/
│               └── fills/
│
├── chain/
│   ├── Anchor.toml
│   ├── package.json
│   ├── programs/
│   │   └── nyxbid/
│   │       ├── Cargo.toml
│   │       └── src/
│   │           ├── lib.rs
│   │           ├── state.rs
│   │           ├── errors.rs
│   │           ├── events.rs
│   │           └── instructions/
│   │               ├── mod.rs
│   │               ├── create_intent.rs
│   │               ├── submit_quote.rs
│   │               ├── resolve_auction.rs
│   │               ├── settle.rs
│   │               └── cancel.rs
│   └── tests/
│       └── nyxbid.ts
│
├── crates/
│   └── nyxbid-types/          shared Rust DTOs
│
├── deploy/
│   ├── Caddyfile              TLS + rate limit
│   └── README.md
│
├── docker-compose.yml
├── Cargo.toml                 workspace: apps/server + crates/nyxbid-types
├── justfile                   dev / build / test / docker recipes
├── .env.example
├── BRAIN.md                   operator notes (living)
├── PROGRESS.md                checklist aligned with this spec
└── PROJECT.md                 this file
```

---

## 4. Sealed-bid RFQ protocol

### 4.1 Lifecycle

```
 t0                t1                 t2                 t3
 │                 │                  │                  │
 │  create_intent  │  submit_quote…   │  resolve         │  settle
 │ ──────────────▶ │ ──────────────▶  │ ──────────────▶  │ ───────
 │                 │                  │                  │
 │   taker posts   │   makers post    │  makers reveal,  │  atomic
 │   Intent PDA    │   Quote PDAs     │  program picks   │  escrow
 │   (open)        │   (commitment    │  winner that     │  swap,
 │                 │    only)         │  clears limit    │  Receipt
 │                 │                  │                  │  PDA
```

Concretely:

1. **`create_intent`** — taker submits side, base/quote mints, size,
   limit price, `reveal_deadline`, `resolve_deadline`. Program creates
   `Intent PDA [b"intent", taker, nonce]` in state `Open`.
2. **`submit_quote`** — each maker submits
   `commitment = sha256(price_le || size_le || nonce)` along with their
   pubkey. Program creates `Quote PDA [b"quote", intent, maker, nonce]`.
   No price is on chain yet.
3. **`resolve_auction`** — after `reveal_deadline`, any party (usually
   the server acting as a resolver, or the winning maker themselves) reveals
   `(price, size, nonce)` for their quote. The program verifies
   `sha256(…) == commitment`, verifies the revealed price clears the
   taker's `limit_price` for the correct side, and stamps the intent
   as `Resolved` with `winning_quote` set.
4. **`settle`** — in the same tx (or next block): Escrow PDAs hold taker's
   quote-mint and maker's base-mint. On settle, the program CPIs into the
   SPL Token program twice, crediting each counterparty, and writes a
   `Receipt PDA` with `(price, size, settled_at)`.

### 4.2 Commitment construction

```
commitment = sha256( price_le_u64 || size_le_u64 || nonce_32 )
```

- `price_le_u64` — 6-decimal quote-mint minor units (USDC style).
- `size_le_u64` — 6/9-decimal base-mint minor units.
- `nonce_32` — random 32 bytes, stored off-chain by the maker, revealed on
  `resolve_auction`. Nonce prevents rainbow-table attacks on small
  commitment spaces.

### 4.3 Winner selection

Pure on-chain logic, deterministic:

- For a **Buy** intent, pick the lowest revealed price ≤ `limit_price`.
- For a **Sell** intent, pick the highest revealed price ≥ `limit_price`.
- Ties broken by earliest `submit_quote` slot, then by maker pubkey.

If no revealed quote clears the limit, intent transitions to `Expired` at
`resolve_deadline` and any escrowed funds are returned.

### 4.4 Escrow model (HTLC-style)

- Taker deposits quote-mint into `Escrow PDA [b"escrow", intent]` at
  `create_intent` time.
- Winning maker must deposit base-mint into the same escrow before the
  settle instruction succeeds.
- `settle` is all-or-nothing: either both transfers occur and a
  `Receipt` is written, or the tx aborts and balances are untouched.
- Cancel/expiry paths return escrowed funds to their original owners.

### 4.5 Failure & cancel paths

| Trigger                                        | Effect                                      |
|------------------------------------------------|---------------------------------------------|
| `cancel` called by taker before reveal_deadline | Intent → `Cancelled`, escrow returned       |
| No quotes revealed by `resolve_deadline`       | Intent → `Expired`, escrow returned         |
| Revealed quotes, none clear limit              | Intent → `Expired`, escrow returned         |
| Winner fails to fund within settle window      | Intent → `Expired`, taker escrow returned   |

---

## 5. Anchor program

### 5.1 Accounts

All PDAs. Seeds documented in `state.rs`.

| Account  | Seeds                                           | Purpose                                   |
|----------|-------------------------------------------------|-------------------------------------------|
| `Intent` | `["intent", taker, nonce]`                      | Taker's order; status machine lives here. |
| `Quote`  | `["quote", intent, maker, nonce]`               | Sealed commitment, later revealed.        |
| `Escrow` | `["escrow", intent]`                            | Token vault holding both sides.           |
| `Receipt`| `["receipt", intent]`                           | Post-settlement audit record.             |

### 5.2 Instructions

| Instruction       | Signer(s)   | Writes                     | Pre-conditions                            |
|-------------------|-------------|----------------------------|-------------------------------------------|
| `create_intent`   | taker       | Intent, Escrow             | Valid mints, size > 0, limit > 0          |
| `submit_quote`    | maker       | Quote                      | Intent is `Open`, before `reveal_deadline`|
| `resolve_auction` | any         | Intent, Quote              | After `reveal_deadline`, valid commitment |
| `settle`          | any         | Intent, Receipt, tokens    | Intent is `Resolved`, funding complete    |
| `cancel`          | taker       | Intent                     | Status is `Open`                          |

### 5.3 Events

`IntentCreated`, `QuoteSubmitted`, `AuctionResolved`, `Settled`,
`Cancelled`. Each event carries the bare minimum for an indexer to
reconstruct state without extra RPC calls.

### 5.4 Errors

`IntentNotOpen`, `RevealDeadlineNotReached`, `ResolveDeadlineNotReached`,
`CommitmentMismatch`, `AlreadyRevealed`, `LimitBreached`,
`InsufficientDeposit`, `AlreadySettled`, `Unauthorized`.

### 5.5 Security model

- **Replay**: each instruction binds to a PDA derived from a nonce the
  caller supplies; re-using a nonce collides with an existing PDA and
  fails account `init`.
- **MEV**: winner is picked by on-chain, deterministic logic from
  revealed quotes. There is no "last-look" path for the server or the
  winner to drop a fill.
- **Insider leak**: the server never sees plaintext prices before
  reveal — it only carries commitments.
- **Griefing**: deposit requirement for both sides gates griefers.
  Failure paths return escrow; they do not slash.

---

## 6. `nyxbid-server`

### 6.1 Modules

- `main.rs` — boot, tracing, state, router.
- `state.rs` — `AppState` (intents, quotes, fills, markets, solana,
  broadcast tx), `StreamEvent` enum for SSE.
- `routes.rs` — REST + SSE endpoints.
- `intent.rs` — `CreateIntentRequest`, commitment-root hashing.
- `auction.rs` — winner selection helpers mirroring on-chain logic.
- `mcp.rs` — MCP tool surface (JSON-RPC).
- `solana.rs` — RPC config, program ID, mint addresses.

### 6.2 REST API

All on `:8080`.

| Method | Path                          | Purpose                           |
|--------|-------------------------------|-----------------------------------|
| GET    | `/health`                     | Server version + status           |
| GET    | `/api/dashboard`              | Aggregate stats                   |
| GET    | `/api/markets`                | Supported markets                 |
| GET    | `/api/intents`                | List intents                      |
| POST   | `/api/intents`                | Create a new intent               |
| GET    | `/api/intents/:id`            | Get one intent                    |
| GET    | `/api/intents/:id/quotes`     | Quotes on one intent              |
| GET    | `/api/fills`                  | Settled fills                     |
| GET    | `/api/events`                 | SSE lifecycle stream              |

### 6.3 SSE stream

`GET /api/events` emits one JSON per message matching `StreamEvent`:

```json
{"type":"intent_created", "value": {...Intent}}
{"type":"quote_submitted", "value": {...Quote}}
{"type":"auction_resolved", "intent_id": "int_..."}
{"type":"filled", "value": {...Fill}}
```

### 6.4 MCP surface

```
nyxbid.list_markets       ()                                         -> Market[]
nyxbid.create_intent      (symbol, side, size, limit)                -> Intent
nyxbid.list_quotes        (intent_id)                                -> Quote[]
nyxbid.get_receipt        (intent_id)                                -> Fill?
nyxbid.cancel_intent      (intent_id)                                -> Intent
```

Transport: JSON-RPC over stdio for local agents, HTTP + SSE at `/mcp`
for hosted agents.

### 6.5 Observability

- `tracing` to stdout with `EnvFilter`; JSON format in container.
- Every REST handler logs a span with `intent_id`, `maker`, `taker`.
- Prometheus (Phase 2): `nyxbid_intents_total`, `nyxbid_quotes_total`,
  `nyxbid_fills_total`, `nyxbid_settle_latency_seconds`.

---

## 7. `nyxbid-client`

### 7.1 Public surfaces

- `/` — landing (marketing). Three-section story: problem, sealed-bid
  flow, architecture.
- `/docs` — technical documentation, mirrored content of this file
  scaled for a web audience.
- `/dashboard` — aggregate view.
- `/intents` — list of intents, filterable by status.
- `/quotes` — quotes by intent (deep-link from `/intents/:id`).
- `/fills` — settled fills with explorer links.

### 7.2 Design language

- Tailwind v4 tokens: `background`, `foreground`, `muted`, `accent`,
  `border`, `card`. Dark-first; monochrome + single accent hue.
- Geist Sans + Geist Mono.
- No heavy animations. Skeletons on load. Tables for data. No sidebar
  on the marketing surfaces; sidebar only in `(dashboard)` group.
- Laws of UX we're committing to: Jakob's Law (tables look like tables),
  Hick's Law (≤ 5 nav items), Miller's Law (chunk dashboard cards by 4),
  Aesthetic-Usability Effect (polish everything first-time users see).

### 7.3 Deployment

- `nyxbid.trade` → landing
- `docs.nyxbid.trade` → docs
- `app.nyxbid.trade` → dashboard
- `api.nyxbid.trade` → `nyxbid-server`

---

## 8. Wire types (`nyxbid-types`)

| Type              | Fields                                                                 |
|-------------------|------------------------------------------------------------------------|
| `Side`            | `buy \| sell`                                                          |
| `IntentStatus`    | `open \| resolved \| settled \| cancelled \| expired`                   |
| `Intent`          | id, taker, side, base_mint, quote_mint, size, limit_price, deadlines, commitment_root, status, winning_quote, created_at |
| `Quote`           | id, intent_id, maker, commitment, revealed_price?, revealed_size?, revealed, created_at |
| `Fill`            | id, intent_id, taker, maker, mints, size, price, tx_signature?, settled_at |
| `Market`          | symbol, base_mint, quote_mint, min_size                                |
| `DashboardStats`  | open_intents, resolved_intents, total_fills, notional_24h, avg_makers_per_intent |

---

## 9. DevOps

### 9.1 Local dev

```
cp .env.example .env
just dev                  # server:8080 + client:3000
just test-chain           # anchor test
```

### 9.2 Docker

- `docker-compose.yml` runs both containers on the host network map
  `8080:8080` and `3000:3000`.
- `apps/server/Dockerfile` — multi-stage, distroless
  (`gcr.io/distroless/cc-debian12:nonroot`) runtime, OpenSSL libs copied
  from builder.
- `apps/client/Dockerfile` — Bun builder, Node 22 slim runner with
  Next.js standalone output.

### 9.3 Production

- Caddy terminates TLS on the four subdomains (Caddyfile in `deploy/`),
  reverse-proxies to the two internal ports, rate-limits `/api/*`.
- Devnet program ID lives in `Anchor.toml` and `.env`; mainnet program ID
  is promoted on the v0.2 milestone.

---

## 10. Roadmap

### v0.1 — "shape it" (this repo, current phase)
- Anchor program scaffolded with all five instructions and state/events.
- `nyxbid-server` compiles, serves REST + SSE, seeds in-memory state.
- Next.js client with marketing + docs + dashboard placeholders.
- Full specification (this file) + checklist (`PROGRESS.md`).
- Public repo under `github.com/Nyxbid/nyxbid`.

### v0.2 — "trade on devnet"
- Wire the Anchor program's five instructions to the server's intent.rs
  and auction.rs modules via `solana-client`.
- Real HTLC-style escrow with SPL token CPIs (base + quote).
- End-to-end happy path test (taker-1 maker-3) on devnet.
- MCP surface live at `/mcp`.
- Dashboard reads real program state.

### v0.3 — "invite the makers"
- Two-sided private beta: 3 takers, 5 makers.
- Prometheus + Grafana.
- Maker reputation (non-slashing, soft scores) surfaced in the UI.
- Mainnet deploy.

### v1.0 — "venue-grade"
- Cross-mint support beyond SOL/USDC.
- Partial fills (split an intent across N quotes).
- Published maker SDK in TypeScript + Python.
- Formal audit of the Anchor program.
