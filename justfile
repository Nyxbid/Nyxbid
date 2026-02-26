default:
    @just --list

dev-server:
    cargo run -p nyxbid-server

dev-client:
    cd apps/client && bun dev

dev:
    just dev-server &
    just dev-client

build-server:
    cargo build --release -p nyxbid-server

build-client:
    cd apps/client && bun run build

build-chain:
    cd chain && anchor build

build: build-server build-client

test-chain:
    cd chain && anchor test

deploy-devnet:
    cd chain && anchor deploy --provider.cluster devnet

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
    cd chain && rm -rf target .anchor
