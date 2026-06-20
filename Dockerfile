# ── Stage 1: Build React dashboard ───────────────────────────────────────────
FROM node:22-alpine AS dashboard-builder
WORKDIR /dashboard
COPY dashboard/package*.json ./
RUN npm ci --prefer-offline
COPY dashboard/ ./
RUN npm run build

# ── Stage 2: Build Rust API with embedded dashboard ───────────────────────────
FROM rust:1.87-slim AS rust-builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
# Copy full workspace (Cargo.toml, Cargo.lock, all crates)
COPY Cargo.toml ./
COPY crates/ ./crates/
# Copy built dashboard so the embed feature can find it
COPY --from=dashboard-builder /dashboard/dist ./dashboard/dist
RUN cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# ── Stage 3: Minimal runtime image ───────────────────────────────────────────
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/target/release/hsip-api /usr/local/bin/hsip-api

# Railway injects $PORT; bind on all interfaces so the container is reachable
ENV HOST=0.0.0.0
ENV PORT=8080
ENV CORS_ALLOW_ALL=1

# Data directory for master key, admin key, and SQLite DB
RUN mkdir -p /data
ENV HOME=/data

EXPOSE 8080
CMD ["hsip-api"]
