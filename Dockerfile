# ==============================================================================
# RampOS Server - Multi-stage Docker Build with cargo-chef
# ==============================================================================

# Stage 1: Chef - Install cargo-chef
FROM rust:latest AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Planner - Analyze dependencies
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Cache dependencies + build
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cook dependencies (this layer is cached unless deps change)
RUN cargo chef cook --release --recipe-path recipe.json --package ramp-api
# Copy source and build
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY migrations ./migrations
RUN cargo build --release --package ramp-api

# Stage 4: Runtime
FROM debian:trixie-slim AS runtime

LABEL org.opencontainers.image.source="https://github.com/rampos/rampos"
LABEL org.opencontainers.image.description="RampOS - Open-source payment infrastructure for Southeast Asia"
LABEL org.opencontainers.image.licenses="MIT"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/rampos-server /app/rampos-server

# Create non-root user
RUN useradd -m -u 1000 rampos
USER rampos

ENV RUST_LOG=info
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/rampos-server"]
