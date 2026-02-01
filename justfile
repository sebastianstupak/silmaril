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

# Run ECS benchmarks
bench-ecs:
    cargo bench --package engine-core

# Run network benchmarks (when implemented)
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

# === Docker (Phase 2.1 Part C - coming soon) ===

# Start development environment
# dev:
#     docker-compose -f docker-compose.dev.yml up

# Stop development environment
# dev-stop:
#     docker-compose -f docker-compose.dev.yml down

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
