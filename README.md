# Nyxbid

Private, sealed-bid RFQ venue for OTC-size trades on Solana. Atomic
settlement on chain. Agent-native via A2A and gRPC.

- Deploy notes: [`deploy/README.md`](./deploy/README.md)

## Architecture

- **Solana** — signed transactions for writes and atomic settlement.
- **A2A** — agent identity, task negotiation, signed quote metadata.
- **gRPC** — low-latency event stream for professional makers.
- **REST / WebSocket** — web app and standard integrations.

The server coordinates the off-chain experience but never custodies funds
or signs user transactions. Money movement belongs to Solana.

## Quick start

Prerequisites: Rust, Bun, Solana CLI, Anchor 1.0, Just.

```
git clone https://github.com/Nyxbid/Nyxbid.git
cd Nyxbid
cp .env.example .env
just dev
```

- Landing: `http://localhost:3000`
- Docs: `http://localhost:3000/docs`
- Dashboard: `http://localhost:3000/dashboard`
- API: `http://localhost:8080`

## Layout

```
apps/server     nyxbid-server (Rust / Axum)
apps/client     nyxbid-client (Next.js)
chain           Anchor program (nyxbid)
crates          nyxbid-types (shared wire types)
deploy          Caddyfile + deploy notes
```

## Development

```
just dev            server + client locally
just test-chain     anchor test
just docker-up      both containers
just build          release build for both apps
```

## License

Apache-2.0.
