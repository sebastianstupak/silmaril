# Multi-stage build for server binary
FROM rust:1.75 as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY engine ./engine

# Build server in release mode
RUN cargo build --bin agent_game_engine_server --features server --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 gameserver

# Copy binary from builder
COPY --from=builder /app/target/release/agent_game_engine_server /usr/local/bin/server

# Set ownership
RUN chown gameserver:gameserver /usr/local/bin/server

# Switch to non-root user
USER gameserver

# Expose ports
EXPOSE 7777/tcp
EXPOSE 7778/udp

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/server"]
