FROM rust:1.86-slim AS builder
WORKDIR /src
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY apps/server apps/server
COPY crates crates
RUN cargo build --release -p payq-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /src/target/release/payq-server /usr/local/bin/payq-server
ENV RUST_LOG=info
EXPOSE 3001
CMD ["payq-server"]
