# Deploy

Production reverse-proxy config for Nyxbid. The stack:

- `app.nyxbid.trade` — Next.js dashboard (port 3000)
- `api.nyxbid.trade` — Rust server (port 8080)
- `docs.nyxbid.trade` — Next.js docs (same binary as app)
- `nyxbid.trade`      — marketing / landing (same binary as app)

## What lives in this folder

- `Caddyfile` — opinionated TLS + reverse-proxy + rate-limit config. Safe to
  commit; it contains no secrets.
- `README.md` — this file.

Secrets live in environment variables (`ACME_EMAIL`, `*_HOST`, `*_UPSTREAM`)
passed to the Caddy process, not in this repo.

## DNS

Point four A/AAAA records at the box running Caddy:

```
nyxbid.trade        A  <server ip>
app.nyxbid.trade    A  <server ip>
api.nyxbid.trade    A  <server ip>
docs.nyxbid.trade   A  <server ip>
```

Caddy will auto-provision Let's Encrypt certs on first request.

## Run Caddy

```
export ACME_EMAIL=ops@nyxbid.trade
caddy run --config deploy/Caddyfile
```

For a systemd unit, see Caddy's docs.

## Rate limiting

The `Caddyfile` uses `caddy-rate-limit` (build Caddy with this plugin via
`xcaddy`). Default: 60 req/min per IP on `/api/*`. Tune the window/events in
the `rate_limit` snippet for your traffic profile.

## Backend CORS

The `nyxbid-server` currently allows any origin via `tower_http::cors`. Tighten
this in production to the public app/docs origins.

## Process model

- `nyxbid-server` on `127.0.0.1:8080` (single binary, stateless apart from
  in-memory auction state; use supervisor/systemd to restart).
- `nyxbid-client` (Next.js standalone) on `127.0.0.1:3000`.
- `caddy` fronts both over TLS.

For container-based deployments, use the included `docker-compose.yml` as a
starting point and put Caddy in front of the two published ports.
