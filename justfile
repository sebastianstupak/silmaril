# Agent Game Engine - Build Commands
# Install `just`: cargo install just
# Run `just` to see all commands

# Default: show available commands
default:
    @just --list

# === Build Commands ===

# Build client (dev)
build-client:
    cargo build --bin client

# Build server (dev)
build-server:
    cargo build --bin server

# Build both binaries (dev)
build:
    cargo build --bin client
    cargo build --bin server

# Build client (release - optimized for performance)
build-client-release:
    cargo build --bin client --release

# Build server (release - size-optimized)
build-server-release:
    cargo build --bin server --profile release-server

# Build both binaries (release)
build-release:
    cargo build --bin client --release
    cargo build --bin server --profile release-server

# Clean build artifacts
clean:
    cargo clean

# === Run Commands ===

# Run client
run-client:
    cargo run --bin client

# Run server
run-server:
    cargo run --bin server

# === Test Commands ===

# Run all tests
test:
    cargo test --all-features

# Run tests for client code only
test-client:
    cargo test --features client

# Run tests for server code only
test-server:
    cargo test --features server

# Run macro tests
test-macros:
    cargo test --package engine-macros

# Run with verbose output
test-verbose:
    cargo test --all-features -- --nocapture

# === Code Quality ===

# Format all code
fmt:
    cargo fmt --all

# Check formatting
fmt-check:
    cargo fmt --all --check

# Run clippy lints
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Fix clippy issues automatically
clippy-fix:
    cargo clippy --all-targets --all-features --fix

# Run all checks (format + clippy + test)
check: fmt-check clippy test

# === Benchmarks ===

# Run all benchmarks
bench:
    cargo bench --all-features

# Run all benchmarks and save baseline
bench-all:
    cargo bench --all-features -- --save-baseline current

# Run platform-specific benchmarks only
bench-platform:
    cargo bench --package engine-core --bench platform_benches
    cargo bench --package engine-renderer --bench vulkan_context_bench

# Run ECS benchmarks only
bench-ecs:
    cargo bench --package engine-core --bench ecs_simple
    cargo bench --package engine-core --bench ecs_comprehensive
    cargo bench --package engine-core --bench query_benches
    cargo bench --package engine-core --bench world_benches

# Run physics benchmarks
bench-physics:
    cargo bench --package engine-physics

# Run renderer benchmarks
bench-renderer:
    cargo bench --package engine-renderer

# Run math benchmarks
bench-math:
    cargo bench --package engine-math

# Run profiling overhead benchmarks
bench-profiling:
    cargo bench --package engine-profiling

# Run industry comparison benchmarks
bench-compare:
    cargo bench --package engine-core --bench game_engine_comparison

# Compare current benchmarks with saved baseline
bench-baseline:
    cargo bench --all-features -- --baseline current

# Save current benchmarks as main baseline
bench-save-baseline:
    cargo bench --all-features -- --save-baseline main

# Run quick benchmark smoke test (fast, for CI)
bench-smoke:
    cargo bench --package engine-core --bench ecs_simple -- --sample-size 10

# Run benchmarks with profiling enabled
bench-profile:
    cargo bench --all-features --features profiling-puffin

# Open benchmark report in browser
bench-report:
    @echo "Opening benchmark report..."
    @if [ -f "target/criterion/report/index.html" ]; then \
        xdg-open target/criterion/report/index.html 2>/dev/null || \
        open target/criterion/report/index.html 2>/dev/null || \
        start target/criterion/report/index.html 2>/dev/null || \
        echo "Could not open browser. View report at: target/criterion/report/index.html"; \
    else \
        echo "No benchmark report found. Run 'just bench' first."; \
    fi

# Network benchmarks (when implemented)
bench-network:
    cargo bench --package engine-networking

# === Documentation ===

# Build documentation
doc:
    cargo doc --no-deps --all-features

# Build and open documentation
doc-open:
    cargo doc --no-deps --all-features --open

# === Development ===

# Watch for changes and rebuild (requires cargo-watch)
watch:
    cargo watch -x 'build --bin server'

# Watch and run tests
watch-test:
    cargo watch -x 'test --all-features'

# Check project compiles (fast, no codegen)
check-compile:
    cargo check --all-targets --all-features

# === Docker ===

# Start development environment (with hot-reload)
dev:
    docker-compose -f docker-compose.dev.yml up

# Start development environment (detached)
dev-detached:
    docker-compose -f docker-compose.dev.yml up -d

# Stop development environment
dev-stop:
    docker-compose -f docker-compose.dev.yml down

# Start production environment
prod:
    docker-compose up -d

# Stop production environment
prod-stop:
    docker-compose down

# View server logs (dev)
dev-logs:
    docker-compose -f docker-compose.dev.yml logs -f server

# View server logs (production)
prod-logs:
    docker-compose logs -f server

# Rebuild Docker images
docker-rebuild:
    docker-compose build --no-cache

# Show Docker image sizes
docker-sizes:
    @echo "Development images:"
    @docker images | grep agent-game.*dev || echo "  No dev images"
    @echo "\nProduction images:"
    @docker images | grep agent-game-engine || echo "  No prod images"

# Clean Docker artifacts
docker-clean:
    docker-compose down -v
    docker-compose -f docker-compose.dev.yml down -v

# === Platform-Specific ===

# Build for Windows (from Linux/Mac)
build-windows:
    cargo build --bin client --target x86_64-pc-windows-msvc
    cargo build --bin server --target x86_64-pc-windows-msvc

# Build for Linux (from Windows/Mac)
build-linux:
    cargo build --bin client --target x86_64-unknown-linux-gnu
    cargo build --bin server --target x86_64-unknown-linux-gnu

# Build for macOS (from Linux/Windows)
build-macos:
    cargo build --bin client --target x86_64-apple-darwin
    cargo build --bin server --target x86_64-apple-darwin

# === Utilities ===

# Show binary sizes
sizes:
    @echo "Client (dev):"
    @ls -lh target/debug/client* 2>/dev/null || echo "Not built"
    @echo "\nServer (dev):"
    @ls -lh target/debug/server* 2>/dev/null || echo "Not built"
    @echo "\nClient (release):"
    @ls -lh target/release/client* 2>/dev/null || echo "Not built"
    @echo "\nServer (release-server):"
    @ls -lh target/release-server/server* 2>/dev/null || echo "Not built"

# Update dependencies
update:
    cargo update

# Show outdated dependencies
outdated:
    cargo outdated
