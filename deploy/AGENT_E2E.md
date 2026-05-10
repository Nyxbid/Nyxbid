# Agent end-to-end smoke test

Nyxbid exposes **Google A2A v1** on the API host. **Linear** is issue tracking — it does not run A2A clients. Use **curl**, a small script, or any A2A-capable agent runtime against your deployed `api.nyxbid.com` (or local `http://localhost:8080`).

## Prerequisites

- HTTPS (or localhost) reachable
- `NYXBID_USDC_MINT` / program ID consistent with devnet
- Optional: `A2A_SIGNING_KEY_PEM` for signed agent cards + JWKS

## 1. Discover the venue

```bash
API=https://api.nyxbid.com   # or http://YOUR_EC2_IP:8080 without Caddy

curl -sS "$API/.well-known/agent-card.json" | jq .
curl -sS "$API/.well-known/jwks.json" | jq .    # empty array if signing off
```

You should see `name`, `skills[]`, `url` (JSON-RPC endpoint), `capabilities`.

## 2. Health (non-A2A)

```bash
curl -sS "$API/health" | jq .
```

Expect `solana_configured: true` in production.

## 3. JSON-RPC: `message/send` (prepare a tx skill)

Replace `TAKER_PUBKEY` with a base58 Solana address (your devnet wallet).

```bash
curl -sS "$API/api/a2a/v1" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "message/send",
    "params": {
      "message": {
        "role": "user",
        "parts": [
          {
            "kind": "data",
            "data": {
              "skill": "post_intent",
              "taker": "TAKER_PUBKEY",
              "side": "buy",
              "base_mint": "So11111111111111111111111111111111111111112",
              "quote_mint": "YOUR_QUOTE_MINT_FROM_HEALTH_OR_MARKETS",
              "size": 100000000,
              "limit_price": 100000,
              "reveal_deadline": 2000000000,
              "resolve_deadline": 2000000060,
              "settle_deadline": 2000000120,
              "nonce_hex": "0123456789abcdef0123456789abcdef"
            }
          }
        ]
      }
    }
  }' | jq .
```

The response should include a **Task** with **artifacts** containing `tx_base64` (unsigned). Signing still happens in a wallet or bot keypair — the server never holds your secret key.

Deadline fields must be **Unix seconds in the future** and ordered `reveal < resolve < settle`. Use:

```bash
NOW=$(date +%s)
REVEAL=$((NOW + 120))
RESOLVE=$((REVEAL + 60))
SETTLE=$((RESOLVE + 120))
```

…and substitute into the JSON.

## 4. Live events: `message/stream` (SSE)

A2A streaming is a **long-lived HTTP request** returning `text/event-stream`. Easiest check with **curl**:

```bash
curl -N -sS "$API/api/a2a/v1" \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "message/stream",
    "params": {
      "message": {
        "role": "user",
        "parts": [{
          "kind": "data",
          "data": { "skill": "subscribe_events", "markets": ["SOL/USDC"] }
        }]
      }
    }
  }'
```

Leave it open; create an intent from the web app in another tab and watch for task/status events.

Production agents should use an **SSE client** (fetch + ReadableStream, or `eventsource` where applicable) and parse JSON lines per the A2A spec.

## 5. Task lifecycle

- `tasks/get` — fetch task by id  
- `tasks/cancel` — cancel in-flight task  
- `tasks/resubscribe` — replay + continue stream  

See `apps/server/src/a2a/rpc.rs` for exact method names and params (`camelCase` on the wire).

## 6. What “done” looks like

1. Card + JWKS load without error.  
2. `message/send` with `post_intent` returns a task with an unsigned tx artifact.  
3. After you sign + broadcast (separate step), the indexer shows the intent; `subscribe_events` fires updates.  
4. Maker path: `submit_quote` → `reveal_quote` → fund/settle skills, same pattern.

For a **full** sealed-bid loop you need at least one **maker** process (second wallet or bot) calling quote skills simultaneously. The web UI alone exercises the taker leg cleanly.
