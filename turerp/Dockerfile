# Build stage
FROM rust:1.89-bookworm as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY Cargo.toml Cargo.lock ./
COPY src src/
COPY migrations migrations/

# Build dependencies first (for caching)
WORKDIR /build
RUN cargo fetch

# Build release binary
RUN cargo build --release --bin turerp

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
# - wget: required for HEALTHCHECK below. NOT shipped in `debian:bookworm-slim`
#   (only in the full `debian:bookworm` image), so we install it here with
#   --no-install-recommends to keep the layer small (~200 KB).
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 appuser

WORKDIR /app

# Copy binary from builder. The binary is self-contained: SQL migrations are
# embedded at compile time via include_str! in src/db/pool.rs, so the runtime
# image needs neither src/ nor migrations/ on disk. (An earlier revision also
# copied src/domain, src/api, src/config and migrations/ here — but src/config
# is a file (config.rs), not a directory, which broke this COPY step, and the
# others had no runtime consumer.)
COPY --from=builder /build/target/release/turerp /app/

# Create directories for runtime
RUN mkdir -p /app/logs /app/data && chown -R appuser:appuser /app

USER appuser

EXPOSE 8080

ENV RUST_LOG=info

# /health/live: liveness — process is up and accepting requests. Use for Docker HEALTHCHECK
# (orchestrator restarts the container if this fails).
# /health/ready: readiness — DB/cache/scheduler are reachable. Use for Kubernetes
# readinessProbe or compose `condition: service_healthy` gating.
# `wget` is preferred over `curl` here: curl is not in bookworm-slim either, and
# wget is smaller (~200 KB vs curl's ~400 KB with its bundled resolver).
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://127.0.0.1:8080/health/live >/dev/null 2>&1 || exit 1

ENTRYPOINT ["/app/turerp"]