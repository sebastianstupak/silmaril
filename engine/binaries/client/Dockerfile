# Multi-stage build for client binary
FROM rust:1.75-slim as builder

# Install only essential build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    pkg-config \
    libvulkan-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY engine ./engine

# Build with release profile
RUN cargo build --bin client --features client --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libvulkan1 \
    vulkan-tools \
    mesa-vulkan-drivers \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/client /usr/local/bin/client

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/client"]
