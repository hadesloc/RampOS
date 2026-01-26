# RampOS Server
FROM rust:1.75-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build dependencies first (for caching)
RUN cargo build --release --package ramp-api

# Final image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rampos-server /app/rampos-server

# Create non-root user
RUN useradd -m -u 1000 rampos
USER rampos

ENV RUST_LOG=info
EXPOSE 8080

ENTRYPOINT ["/app/rampos-server"]
