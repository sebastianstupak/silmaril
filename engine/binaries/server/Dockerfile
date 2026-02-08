# Multi-stage build for server binary
# Target: <50MB image size
FROM rust:1.93-slim as builder

# Install only essential build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY engine ./engine

# Build with size-optimized profile
# release-server profile: strip = true, opt-level = "z", lto = true
RUN cargo build --bin server --profile release-server --features server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 gameserver

# Copy binary from builder (release-server profile output)
COPY --from=builder /app/target/release-server/server /usr/local/bin/server

# Set ownership
RUN chown gameserver:gameserver /usr/local/bin/server

# Switch to non-root user
USER gameserver

# Expose ports
EXPOSE 7777/tcp
EXPOSE 7778/udp

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/server"]
