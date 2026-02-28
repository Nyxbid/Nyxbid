# Nyxbid

Private, sealed-bid RFQ venue for OTC-size trades on Solana. Atomic
settlement on chain. Agent-native via the Model Context Protocol.

- Deploy notes: [`deploy/README.md`](./deploy/README.md)

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
