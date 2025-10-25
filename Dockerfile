# Multi-stage build for GrepDB
# Using cargo-chef for efficient dependency caching

# Stage 1: Chef - Base image with cargo-chef using debian bookworm
FROM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef
WORKDIR /app

# Stage 2: Planner - Prepare dependency recipe
FROM chef AS planner
COPY . .
# Prepare a recipe.json for dependency caching
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Build dependencies and application
FROM chef AS builder
# Copy the recipe.json from planner stage
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is cached unless Cargo.toml/Cargo.lock change
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY . .

# Build the application
RUN cargo build --release

# Create default config if it doesn't exist
RUN if [ ! -f config.yaml ]; then \
    echo "data_volume_dir: /app/data_volume_dir" > config.yaml; \
    fi

# Stage 4: Runtime - Final slim image
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    grep \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/grepdb /usr/local/bin/grepdb

# Copy config file
COPY --from=builder /app/config.yaml /app/config.yaml

# Copy test scripts (optional, useful for testing)
COPY --from=builder /app/*.sh /app/

# Create data and docs directories
RUN mkdir -p /app/data_volume_dir /app/docs

# Create a non-root user to run the application
RUN useradd -m -u 1001 -s /bin/bash grepdb && \
    chown -R grepdb:grepdb /app

USER grepdb

# Expose the port the app runs on
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD grep --version || exit 1

# Run the application
CMD ["grepdb", "--config", "/app/config.yaml"]