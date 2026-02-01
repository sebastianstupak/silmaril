# Multi-stage build for client binary
FROM rust:1.75 as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    libvulkan-dev \
    vulkan-tools \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY engine ./engine

# Build client in release mode
RUN cargo build --bin agent_game_engine_client --features client --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libvulkan1 \
    vulkan-tools \
    mesa-vulkan-drivers \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/agent_game_engine_client /usr/local/bin/client

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/client"]
