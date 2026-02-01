# Phase 0.4: Development Tools

**Status:** ⚪ Not Started
**Estimated Time:** 1 day
**Priority:** High (improves developer experience)

---

## 🎯 **Objective**

Create development scripts, Docker configurations, and tooling to streamline the development workflow with hot-reload, multi-platform testing, and debugging utilities.

---

## 📋 **Tasks**

### **1. Development Environment Script**

**File:** `scripts/dev.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-local}"

case "$MODE" in
  local)
    echo "Starting local development environment..."

    # Start server with hot-reload
    cargo watch -x 'run --bin server' &
    SERVER_PID=$!

    # Wait for server startup
    sleep 2

    # Start client with hot-reload
    cargo watch -x 'run --bin client' &
    CLIENT_PID=$!

    echo "Development environment started!"
    echo "Server PID: $SERVER_PID"
    echo "Client PID: $CLIENT_PID"
    echo "Press Ctrl+C to stop..."

    # Wait for interrupt
    trap "kill $SERVER_PID $CLIENT_PID 2>/dev/null" EXIT
    wait
    ;;

  docker)
    echo "Starting Docker development environment..."
    docker-compose -f docker-compose.dev.yml up --build
    ;;

  *)
    echo "Usage: $0 [local|docker]"
    exit 1
    ;;
esac
```

**File:** `scripts/dev.ps1` (Windows)

```powershell
param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("local", "docker")]
    [string]$Mode = "local"
)

switch ($Mode) {
    "local" {
        Write-Host "Starting local development environment..."

        # Start server
        Start-Process -FilePath "cargo" -ArgumentList "watch -x 'run --bin server'" -PassThru

        # Wait for server startup
        Start-Sleep -Seconds 2

        # Start client
        Start-Process -FilePath "cargo" -ArgumentList "watch -x 'run --bin client'" -PassThru

        Write-Host "Development environment started!"
        Write-Host "Press Ctrl+C to stop..."

        # Keep script running
        while ($true) { Start-Sleep -Seconds 1 }
    }

    "docker" {
        Write-Host "Starting Docker development environment..."
        docker-compose -f docker-compose.dev.yml up --build
    }
}
```

---

### **2. Docker Development Compose**

**File:** `docker-compose.dev.yml`

```yaml
version: '3.8'

services:
  server:
    build:
      context: .
      dockerfile: engine/dev-tools/docker/Dockerfile.dev
      target: server
    volumes:
      - .:/workspace:cached
      - cargo-cache:/usr/local/cargo
      - target-cache:/workspace/target
    environment:
      - RUST_LOG=debug
      - RUST_BACKTRACE=1
    ports:
      - "7777:7777"  # Game server
      - "8080:8080"  # Metrics
    command: cargo watch -x 'run --bin server'

  client:
    build:
      context: .
      dockerfile: engine/dev-tools/docker/Dockerfile.dev
      target: client
    volumes:
      - .:/workspace:cached
      - cargo-cache:/usr/local/cargo
      - target-cache:/workspace/target
      - /tmp/.X11-unix:/tmp/.X11-unix  # X11 forwarding
    environment:
      - RUST_LOG=debug
      - RUST_BACKTRACE=1
      - DISPLAY=${DISPLAY}
    depends_on:
      - server
    command: cargo watch -x 'run --bin client'

volumes:
  cargo-cache:
  target-cache:
```

---

### **3. Development Dockerfile**

**File:** `engine/dev-tools/docker/Dockerfile.dev`

```dockerfile
FROM rust:1.75-bullseye as base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    libvulkan-dev \
    vulkan-tools \
    libxcb1-dev \
    libx11-dev \
    cmake \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install Vulkan SDK
RUN wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | apt-key add - && \
    wget -qO /etc/apt/sources.list.d/lunarg-vulkan-bullseye.list \
    https://packages.lunarg.com/vulkan/lunarg-vulkan-bullseye.list && \
    apt-get update && \
    apt-get install -y vulkan-sdk && \
    rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot-reload
RUN cargo install cargo-watch

WORKDIR /workspace

# Server target
FROM base as server
EXPOSE 7777 8080
CMD ["cargo", "watch", "-x", "run --bin server"]

# Client target
FROM base as client
# X11 for rendering
ENV DISPLAY=:0
CMD ["cargo", "watch", "-x", "run --bin client"]
```

---

### **4. Multi-Platform Test Script**

**File:** `scripts/test-all-platforms.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Testing on all platforms..."

# Linux
echo "=== Testing on Linux ==="
docker run --rm -v "$(pwd)":/workspace -w /workspace rust:1.75 \
  bash -c "apt-get update && apt-get install -y libvulkan-dev && cargo test --all-features"

# Windows (requires Docker Desktop with Windows containers)
echo "=== Testing on Windows ==="
docker run --rm -v "$(pwd):c:/workspace" -w "c:/workspace" \
  mcr.microsoft.com/windows/servercore:ltsc2022 \
  powershell -Command "cargo test --all-features"

# macOS (can't be containerized, skip if not on macOS)
if [[ "$OSTYPE" == "darwin"* ]]; then
  echo "=== Testing on macOS ==="
  cargo test --all-features
else
  echo "=== Skipping macOS (not on macOS host) ==="
fi

echo "All platform tests complete!"
```

---

### **5. Benchmark Runner Script**

**File:** `scripts/bench.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

BASELINE="${1:-}"

if [[ -z "$BASELINE" ]]; then
  # Run benchmarks without comparison
  echo "Running benchmarks..."
  cargo bench --workspace
else
  # Compare against baseline
  echo "Running benchmarks and comparing to baseline: $BASELINE"
  cargo bench --workspace -- --save-baseline "$BASELINE"
fi

echo "Benchmark results saved to target/criterion/"
echo "Open target/criterion/report/index.html to view results"
```

---

### **6. Local CI Simulation Script**

**File:** `scripts/ci-local.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Running full CI checks locally..."

# Format
echo "=== Format Check ==="
cargo fmt --all -- --check
if [[ $? -ne 0 ]]; then
  echo "❌ Format check failed! Run 'cargo fmt' to fix."
  exit 1
fi
echo "✅ Format check passed"

# Clippy
echo "=== Clippy ==="
cargo clippy --workspace --all-features -- -D warnings
if [[ $? -ne 0 ]]; then
  echo "❌ Clippy failed!"
  exit 1
fi
echo "✅ Clippy passed"

# Tests
echo "=== Unit Tests ==="
cargo test --lib --workspace --all-features
if [[ $? -ne 0 ]]; then
  echo "❌ Unit tests failed!"
  exit 1
fi
echo "✅ Unit tests passed"

echo "=== Integration Tests ==="
cargo test --tests --workspace --all-features
if [[ $? -ne 0 ]]; then
  echo "❌ Integration tests failed!"
  exit 1
fi
echo "✅ Integration tests passed"

echo "=== Doc Tests ==="
cargo test --doc --workspace
if [[ $? -ne 0 ]]; then
  echo "❌ Doc tests failed!"
  exit 1
fi
echo "✅ Doc tests passed"

# Documentation
echo "=== Documentation Build ==="
cargo doc --no-deps --workspace --all-features
if [[ $? -ne 0 ]]; then
  echo "❌ Documentation build failed!"
  exit 1
fi
echo "✅ Documentation build passed"

# Security audit
echo "=== Security Audit ==="
if ! command -v cargo-audit &> /dev/null; then
  echo "Installing cargo-audit..."
  cargo install cargo-audit
fi
cargo audit
if [[ $? -ne 0 ]]; then
  echo "⚠️  Security audit found issues!"
  # Don't fail, just warn
fi
echo "✅ Security audit complete"

echo ""
echo "✅ All CI checks passed! Ready to push."
```

---

### **7. Performance Profiling Script**

**File:** `scripts/profile.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-flamegraph}"
TARGET="${2:-client}"

case "$MODE" in
  flamegraph)
    echo "Generating flamegraph for $TARGET..."
    if ! command -v flamegraph &> /dev/null; then
      echo "Installing flamegraph..."
      cargo install flamegraph
    fi
    cargo flamegraph --bin "$TARGET" -- "$@"
    echo "Flamegraph saved to flamegraph.svg"
    ;;

  tracy)
    echo "Starting Tracy profiling for $TARGET..."
    echo "Make sure Tracy profiler is running!"
    cargo build --bin "$TARGET" --features profiling --release
    ./target/release/"$TARGET"
    ;;

  perf)
    echo "Running perf profiling for $TARGET..."
    cargo build --bin "$TARGET" --release
    perf record -F 99 -g ./target/release/"$TARGET"
    perf report
    ;;

  *)
    echo "Usage: $0 [flamegraph|tracy|perf] [client|server]"
    exit 1
    ;;
esac
```

---

### **8. Clean Script**

**File:** `scripts/clean.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Cleaning build artifacts..."

# Cargo clean
cargo clean

# Remove profiling artifacts
rm -f flamegraph.svg
rm -f perf.data*
rm -rf target/criterion

# Remove log files
rm -f *.log

# Remove temp files
rm -rf tmp/
rm -rf temp/

echo "✅ Clean complete!"
```

---

### **9. Setup Script (First-Time Setup)**

**File:** `scripts/setup.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Setting up development environment..."

# Check Rust installation
if ! command -v rustc &> /dev/null; then
  echo "❌ Rust not found! Install from https://rustup.rs/"
  exit 1
fi
echo "✅ Rust $(rustc --version)"

# Check Cargo
if ! command -v cargo &> /dev/null; then
  echo "❌ Cargo not found!"
  exit 1
fi
echo "✅ Cargo $(cargo --version)"

# Install required components
echo "Installing Rust components..."
rustup component add rustfmt clippy

# Install development tools
echo "Installing development tools..."
cargo install cargo-watch || true
cargo install cargo-audit || true
cargo install flamegraph || true
cargo install cargo-tarpaulin || true

# Check Vulkan SDK
if ! command -v vulkaninfo &> /dev/null; then
  echo "⚠️  Vulkan SDK not found!"
  echo "Please install from:"
  echo "  - Windows: https://vulkan.lunarg.com/"
  echo "  - Linux: sudo apt install vulkan-sdk"
  echo "  - macOS: brew install molten-vk"
else
  echo "✅ Vulkan SDK found"
  vulkaninfo --summary
fi

# Check Docker (optional)
if command -v docker &> /dev/null; then
  echo "✅ Docker $(docker --version)"
else
  echo "⚠️  Docker not found (optional for containerized development)"
fi

# Build workspace
echo "Building workspace..."
cargo build --workspace

echo ""
echo "✅ Setup complete!"
echo ""
echo "Quick start:"
echo "  ./scripts/dev.sh local    # Start local development"
echo "  ./scripts/dev.sh docker   # Start Docker development"
echo "  cargo test --workspace    # Run tests"
```

**File:** `scripts/setup.ps1` (Windows)

```powershell
Write-Host "Setting up development environment..."

# Check Rust
if (!(Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust not found! Install from https://rustup.rs/" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Rust $(rustc --version)" -ForegroundColor Green

# Check Cargo
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Cargo not found!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Cargo $(cargo --version)" -ForegroundColor Green

# Install components
Write-Host "Installing Rust components..."
rustup component add rustfmt clippy

# Install tools
Write-Host "Installing development tools..."
cargo install cargo-watch --force
cargo install cargo-audit --force

# Check Vulkan
if (!(Get-Command vulkaninfo -ErrorAction SilentlyContinue)) {
    Write-Host "⚠️  Vulkan SDK not found!" -ForegroundColor Yellow
    Write-Host "Please install from https://vulkan.lunarg.com/"
} else {
    Write-Host "✅ Vulkan SDK found" -ForegroundColor Green
}

# Check Docker
if (Get-Command docker -ErrorAction SilentlyContinue) {
    Write-Host "✅ Docker $(docker --version)" -ForegroundColor Green
} else {
    Write-Host "⚠️  Docker not found (optional)" -ForegroundColor Yellow
}

# Build workspace
Write-Host "Building workspace..."
cargo build --workspace

Write-Host ""
Write-Host "✅ Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Quick start:"
Write-Host "  .\scripts\dev.ps1 local    # Start local development"
Write-Host "  .\scripts\dev.ps1 docker   # Start Docker development"
Write-Host "  cargo test --workspace     # Run tests"
```

---

### **10. VS Code Configuration**

**File:** `.vscode/settings.json`

```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--all-features"],
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.procMacro.enable": true,
  "rust-analyzer.inlayHints.typeHints.enable": true,
  "rust-analyzer.inlayHints.parameterHints.enable": true,
  "rust-analyzer.inlayHints.chainingHints.enable": true,

  "editor.formatOnSave": true,
  "editor.rulers": [100],
  "editor.tabSize": 4,
  "editor.insertSpaces": true,

  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer",
    "editor.formatOnSave": true
  },

  "files.exclude": {
    "**/target": true,
    "**/.git": true
  },

  "files.watcherExclude": {
    "**/target/**": true
  },

  "search.exclude": {
    "**/target": true
  }
}
```

**File:** `.vscode/launch.json`

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Client",
      "cargo": {
        "args": ["build", "--bin=client", "--package=client"],
        "filter": {
          "name": "client",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug",
        "RUST_BACKTRACE": "1"
      }
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Server",
      "cargo": {
        "args": ["build", "--bin=server", "--package=server"],
        "filter": {
          "name": "server",
          "kind": "bin"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug",
        "RUST_BACKTRACE": "1"
      }
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Unit Tests",
      "cargo": {
        "args": ["test", "--no-run", "--lib"],
        "filter": {
          "kind": "lib"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

**File:** `.vscode/tasks.json`

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "cargo build",
      "type": "shell",
      "command": "cargo build --workspace",
      "group": "build",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo test",
      "type": "shell",
      "command": "cargo test --workspace --all-features",
      "group": "test",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo clippy",
      "type": "shell",
      "command": "cargo clippy --workspace --all-features -- -D warnings",
      "group": "build",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo fmt",
      "type": "shell",
      "command": "cargo fmt --all",
      "group": "build"
    },
    {
      "label": "cargo run (client)",
      "type": "shell",
      "command": "cargo run --bin client",
      "group": "build",
      "problemMatcher": ["$rustc"]
    },
    {
      "label": "cargo run (server)",
      "type": "shell",
      "command": "cargo run --bin server",
      "group": "build",
      "problemMatcher": ["$rustc"]
    }
  ]
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Development scripts created (Linux + Windows)
- [ ] Docker development environment working
- [ ] Multi-platform test script functional
- [ ] Benchmark runner script created
- [ ] Local CI simulation script working
- [ ] Profiling scripts for multiple tools
- [ ] Clean script implemented
- [ ] Setup scripts for first-time setup
- [ ] VS Code configuration complete
- [ ] All scripts executable (`chmod +x`)
- [ ] Scripts tested on all platforms

---

## 🎯 **Developer Experience Goals**

| Task | Time Without Tools | Time With Tools | Improvement |
|------|-------------------|-----------------|-------------|
| Setup dev environment | 30+ min | < 5 min | 6x faster |
| Start dev environment | Manual (multi-step) | 1 command | Streamlined |
| Run tests locally | Manual per platform | 1 script | Automated |
| Pre-commit checks | Manual 5+ commands | 1 script | 5x faster |
| Profile performance | Complex setup | 1 command | Easy |
| Clean workspace | Manual (many commands) | 1 command | Simple |

---

## 💡 **Implementation Notes**

1. **Hot-Reload:**
   - Use `cargo-watch` for file watching
   - Rebuilds and restarts on code changes
   - Faster iteration (no manual rebuild)

2. **Docker Development:**
   - Isolated environment
   - Consistent across developers
   - Volume mounts for hot-reload
   - Separate containers for client/server

3. **Multi-Platform Testing:**
   - Docker containers for Linux/Windows
   - Native execution for macOS
   - Automated in CI for all platforms

4. **VS Code Integration:**
   - rust-analyzer for IDE features
   - Debugger configurations for client/server
   - Tasks for common operations
   - Format on save enabled

5. **Profiling:**
   - Multiple tools supported (flamegraph, Tracy, perf)
   - Easy one-command profiling
   - Results automatically opened

---

## 🔧 **Usage Examples**

```bash
# First-time setup
./scripts/setup.sh

# Start development (hot-reload)
./scripts/dev.sh local

# Run all tests locally (like CI)
./scripts/ci-local.sh

# Benchmark and save baseline
./scripts/bench.sh main

# Profile with flamegraph
./scripts/profile.sh flamegraph client

# Clean workspace
./scripts/clean.sh

# Test on all platforms
./scripts/test-all-platforms.sh
```

---

**Dependencies:** [phase0-cicd.md](phase0-cicd.md)
**Next:** [phase1-serialization.md](phase1-serialization.md)
