FROM rust:1.89-slim AS builder
WORKDIR /src
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    dpkg-dev \
 && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY apps/server apps/server
COPY crates crates
RUN cargo build --release -p nyxbid-server

RUN ARCH=$(dpkg-architecture -qDEB_BUILD_MULTIARCH) && \
    mkdir -p /export/usr/lib/${ARCH} && \
    cp /usr/lib/${ARCH}/libssl.so.3 /export/usr/lib/${ARCH}/ && \
    cp /usr/lib/${ARCH}/libcrypto.so.3 /export/usr/lib/${ARCH}/

FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /src/target/release/nyxbid-server /usr/local/bin/nyxbid-server
COPY --from=builder /export/usr/lib/ /usr/lib/
ENV RUST_LOG=info
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/nyxbid-server"]
