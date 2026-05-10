FROM rust:1.89-slim AS builder
WORKDIR /src
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
 && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY apps/server apps/server
COPY crates crates
RUN cargo build --release -p nyxbid-server

# Runtime stage. We deliberately use debian-slim (≈80MB) instead of
# distroless/cc because the Solana / A2A dependency tree pulls in
# libz, libzstd, libcrypto and friends transitively. Distroless ships
# only glibc + libgcc, so each new transitive dep would mean another
# cherry-picked lib copy. Slim is small enough and makes the runtime
# closure honest.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    zlib1g \
    libzstd1 \
 && rm -rf /var/lib/apt/lists/*
RUN groupadd --system --gid 65532 nonroot \
 && useradd  --system --uid 65532 --gid 65532 --no-create-home nonroot
COPY --from=builder /src/target/release/nyxbid-server /usr/local/bin/nyxbid-server
USER nonroot:nonroot
ENV RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/nyxbid-server"]
