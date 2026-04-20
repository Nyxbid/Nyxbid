# Nyxbid — Progress

Live checklist aligned with `PROJECT.md`. Update here every time you ship.
Legend: `[ ]` pending, `[~]` in progress, `[x]` done, `[!]` blocked.

---

## Phase 0 — pivot & scaffold (current)

### Brand & naming
- [x] Lock in project name: **Nyxbid**
- [x] Domains: `nyxbid.trade`, `nyxbid.com`, `nyxbid.fi` confirmed available
- [x] GitHub org: `github.com/Nyxbid/nyxbid`
- [ ] Register `nyxbid.trade` (primary surface)
- [ ] Register `nyxbid.com` (canonical redirect)
- [ ] Create GitHub org `Nyxbid`

### Demolition
- [x] Delete `chain/programs/payq`
- [x] Delete `crates/payq-types`
- [x] Delete `apps/server/src/{mock,x402,solana,routes}.rs`
- [x] Delete `chain/target`, `chain/.anchor`, `chain/tests/payq.ts`

### Workspace rename
- [x] Root `Cargo.toml` workspace members updated
- [x] Rename crate → `nyxbid-types`
- [x] Rename binary → `nyxbid-server`
- [x] `.env.example` updated (`NYXBID_PROGRAM_ID`, `NYXBID_USDC_MINT`)
- [x] `justfile` recipes updated
- [x] `apps/server/Dockerfile` updated for new binary name
- [x] `chain/Anchor.toml` renamed program to `nyxbid`, placeholder ID

### Anchor program — scaffold
- [x] `chain/programs/nyxbid/Cargo.toml` with Anchor 1.0
- [x] `lib.rs` with five instructions wired
- [x] `state.rs` — Intent, Quote, Escrow, Receipt accounts
- [x] `errors.rs` — custom errors
- [x] `events.rs` — IntentCreated, QuoteSubmitted, AuctionResolved, Settled, Cancelled
- [x] `instructions/mod.rs` and five stubs (create_intent, submit_quote, resolve_auction, settle, cancel)
- [ ] Commitment verify inside `resolve_auction`
- [ ] SPL token CPIs in `settle` (base + quote transfers)
- [ ] Escrow funding in `create_intent` and `submit_quote`
- [ ] Expiry path in a dedicated `expire` instruction
- [ ] Unit test per instruction
- [ ] End-to-end test: taker + 3 makers → fill

### Server — scaffold
- [x] `main.rs` boots, logs on `:8080`
- [x] `state.rs` with `AppState` + `StreamEvent` enum
- [x] `routes.rs` with REST + SSE endpoints
- [x] `intent.rs` with request + commitment helpers
- [x] `auction.rs` with winner selection helpers
- [x] `mcp.rs` module present (tool list documented)
- [x] `solana.rs` reads env + program/mint IDs
- [ ] Real `solana-client` RPC calls (read program accounts)
- [ ] Subscribe to `nyxbid` program logs for events
- [ ] MCP JSON-RPC dispatcher
- [ ] Idempotency on `POST /api/intents` (client-nonce dedupe)
- [ ] CORS tightened to the public origins

### Shared types
- [x] `nyxbid-types` — Intent, Quote, Fill, Market, DashboardStats

### Client — scaffold
- [x] Root `layout.tsx` metadata rebranded
- [x] Marketing `(marketing)/layout.tsx` with Nyxbid nav + GitHub link
- [x] Landing page rewritten for sealed-bid RFQ pitch
- [x] Docs page rewritten with intent flow + API + MCP
- [x] Sidebar + mobile-nav routes: Dashboard / Intents / Quotes / Fills
- [x] `lib/data.ts` rewritten with Nyxbid wire types
- [x] `hooks/use-sse.ts` rewritten for `StreamEvent`
- [x] Dashboard page rewritten (stats only, no fake data)
- [x] Intents / Quotes / Fills placeholder pages
- [ ] "Create intent" form on `/intents`
- [ ] Deep-link `/intents/:id` detail view with quote stream
- [ ] Wallet-connect → sign Anchor tx flow (`@solana/wallet-adapter-*` already installed)
- [ ] Live updates via SSE on dashboard + intents

### DevOps
- [x] `docker-compose.yml` publishes `8080:8080` and `3000:3000`
- [x] `deploy/Caddyfile` with TLS + rate-limit + `*.nyxbid.trade` subdomains
- [x] `deploy/README.md`
- [ ] `just demo` recipe for the sealed-bid smoke test
- [ ] GitHub Actions: `cargo check` + `cargo test` + `bun run build`
- [ ] Prometheus metrics endpoint
- [ ] Grafana dashboard JSON in `deploy/`

### Docs
- [x] `PROJECT.md` rewritten as Nyxbid spec
- [x] `PROGRESS.md` rewritten (this file)
- [ ] `BRAIN.md` rewritten for Nyxbid operator notes
- [ ] `README.md` at repo root (getting started, one screen)
- [ ] `CONTRIBUTING.md`
- [ ] `SECURITY.md` with responsible-disclosure

---

## Phase 1 — devnet happy path

- [ ] Anchor program deployed to devnet with a real program ID
- [ ] `NYXBID_PROGRAM_ID` updated in `Anchor.toml` + `.env.example`
- [ ] `nyxbid-server` sends real txs; signs with a hot service keypair
- [ ] Client can create an intent, watch quotes arrive, see a fill on explorer
- [ ] Smoke test script: spawn 1 taker + 3 maker agents, drive a full fill

## Phase 2 — private beta

- [ ] 3 takers, 5 makers onboarded
- [ ] Prometheus + Grafana running
- [ ] Soft maker reputation surfaced in UI
- [ ] Mainnet-beta deploy gated on 100 successful devnet fills

## Phase 3 — v1.0

- [ ] Partial fills (split intent across N quotes)
- [ ] Multi-mint expansion beyond SOL/USDC
- [ ] TypeScript + Python maker SDKs
- [ ] External audit of the Anchor program
