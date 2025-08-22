# Multi-stage build for smaller image size
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev libpq-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy manifest files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy real source code
COPY src/ ./src/

# Build the actual application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        libpq5 \
        ca-certificates \
        libssl3 \
        curl && \
    rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -m -u 1001 appuser

# Create directories and set permissions
RUN mkdir -p /app/uploads /app/logs && \
    chown -R appuser:appuser /app

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /usr/src/app/target/release/kanban-be /app/kanban-be
RUN chmod +x /app/kanban-be && \
    chown appuser:appuser /app/kanban-be

# Switch to non-root user
USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080

ENTRYPOINT ["/app/kanban-be"]
