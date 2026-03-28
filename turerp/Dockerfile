# Build stage
FROM rust:1.75-bookworm as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy dependency files
COPY Cargo.toml Cargo.lock ./
COPY turerp/Cargo.toml turerp/Cargo.lock ./
COPY turerp/src turerp/src/

# Build dependencies first (for caching)
WORKDIR /build/turerp
RUN cargo fetch

# Build release binary
RUN cargo build --release --bin turerp

# Production stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 appuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/turerp/target/release/turerp /app/
COPY --from=builder /build/turerp/src/domain /app/src/domain/
COPY --from=builder /build/turerp/src/api /app/src/api/ 2>/dev/null || true
COPY --from=builder /build/turerp/src/config /app/src/config/ 2>/dev/null || true
COPY --from=builder /build/turerp/migrations /app/migrations/ 2>/dev/null || true

# Create directories for runtime
RUN mkdir -p /app/logs /app/data && chown -R appuser:appuser /app

USER appuser

EXPOSE 8080

ENV RUST_LOG=info
ENV PORT=8080

ENTRYPOINT ["/app/turerp"]