# HSIP Phase 2 - Production Dockerfile
# Multi-stage build for minimal image size

# Stage 1: Build
FROM rust:1.87-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build release binary
RUN cargo build --release --bin hsip-cli

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 hsip && \
    mkdir -p /home/hsip/.hsip && \
    chown -R hsip:hsip /home/hsip

WORKDIR /home/hsip

# Copy binary from builder
COPY --from=builder /app/target/release/hsip-cli /usr/local/bin/

# Switch to non-root user
USER hsip

# Create data directory
VOLUME ["/home/hsip/.hsip"]

# Expose daemon port
EXPOSE 8787

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8787/status || exit 1

# Default command: run daemon
CMD ["hsip-cli", "daemon", "--status_addr", "0.0.0.0:8787"]
