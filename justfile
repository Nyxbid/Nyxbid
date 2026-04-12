default:
    @just --list

dev-server:
    cargo run -p payq-server

dev-client:
    cd apps/client && bun dev

dev:
    just dev-server &
    just dev-client

build-server:
    cargo build --release -p payq-server

build-client:
    cd apps/client && bun run build

build-chain:
    cd chain && anchor build

build: build-server build-client

test-chain:
    cd chain && anchor test

deploy-devnet:
    cd chain && anchor program deploy --provider.cluster devnet

docker-build:
    docker compose build

docker-up:
    docker compose up -d

docker-down:
    docker compose down

docker-logs:
    docker compose logs -f

check:
    cargo check --workspace
    cargo clippy --workspace -- -D warnings

fmt:
    cargo fmt --all

clean:
    cargo clean
    cd apps/client && rm -rf .next
    cd chain && rm -rf target

demo:
    @echo "Starting server in background..."
    just dev-server &
    @sleep 3
    @echo ""
    @echo "Sending test proposal..."
    curl -s -X POST http://localhost:8080/api/proposals \
      -H "Content-Type: application/json" \
      -d '{"agent_id":"agent-alpha","tool":"groq/llama-3.3-70b-versatile","prompt":"What is Solana in one sentence?"}' | python3 -m json.tool
    @echo ""
    @echo "Dashboard stats:"
    curl -s http://localhost:8080/api/dashboard | python3 -m json.tool
