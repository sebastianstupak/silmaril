# Development Workflow Quick Start

> **Fast reference for the `cargo xtask dev` workflow system**

---

## Installation

### 1. Install Cargo Watch (Auto-Reload)

```bash
cargo install cargo-watch
```

**Note:** No need to install `just` - the engine uses `cargo xtask` which is built-in.

### 3. Install Python Dependencies

```bash
pip install -r scripts/dev/requirements.txt
```

---

## Common Commands

### Basic Development

```bash
# Start full dev environment (client + server)
cargo xtask dev full

# Client only
cargo xtask dev full-client

# Server only
cargo xtask dev full-server
```

### Enhanced Modes

```bash
# Pretty log formatting
cargo xtask dev full-logs-live

# With profiler (Puffin)
cargo xtask dev full-profiler

# Debug mode (full symbols)
cargo xtask dev full-debug

# Release mode (fast)
cargo xtask dev full-release

# With validation layers
cargo xtask dev full-validation
```

### Multiplayer Testing

```bash
# Run 3 clients + 1 server
cargo xtask dev full-multi 3
```

### Utilities

```bash
# Check status
cargo xtask dev full-status

# Stop all processes
cargo xtask dev full-stop-all

# Clean everything
cargo xtask dev full-clean

# Quick benchmarks
cargo xtask dev full-benchmark
```

---

## Features

### Auto-Reload

When `cargo-watch` is installed, the development environment automatically:
- Detects code changes
- Rebuilds affected components
- Restarts processes
- Preserves console output

### Color-Coded Output

Different processes have different colors:
- 🔵 **Server** - Blue
- 🟢 **Client** - Green
- 🟣 **Profiler** - Magenta

### Port Checking

Automatically checks if required ports are available:
- 7777 - Server TCP
- 7778 - Server UDP
- 8080 - Metrics/Health

### Graceful Shutdown

Press `Ctrl+C` to:
- Stop all processes gracefully
- Clean up PID files
- Save state

---

## Development Modes Explained

### `cargo xtask dev full`
**Full development environment**
- Runs both client and server
- Auto-reload enabled
- Best for general development

### `cargo xtask dev full-client`
**Client development**
- Only runs client
- Useful for UI/rendering work
- Connect to separate server

### `cargo xtask dev full-server`
**Server development**
- Only runs server
- Useful for game logic work
- Test with separate client

### `cargo xtask dev full-logs-live`
**Log analysis**
- Pretty-printed logs
- Color-coded by level (ERROR=red, WARN=yellow, INFO=green)
- Filter by module

### `cargo xtask dev full-profiler`
**Performance profiling**
- Puffin profiler enabled
- Connect viewer to localhost:8585
- Real-time performance metrics

### `cargo xtask dev full-debug`
**Debugging**
- Full debug symbols
- Ready for debugger attachment
- Shows debugger instructions

### `cargo xtask dev full-release`
**Performance testing**
- Optimized build
- Still allows profiling
- Faster than dev builds

### `cargo xtask dev full-validation`
**Bug hunting**
- Vulkan validation layers
- Extra error checking
- Memory leak detection
- Slower but catches issues early

### `cargo xtask dev full-headless`
**CI/Testing**
- No window/renderer
- Faster startup
- Good for automated testing

### `cargo xtask dev full-multi <count>`
**Multiplayer testing**
- Spawns N clients + 1 server
- Each on different port
- Local multiplayer testing

---

## Troubleshooting

### Port Already in Use

```bash
# Check which ports are in use
cargo xtask dev full-status

# The error will show which process is using the port
```

**Solution:**
1. Stop the process using the port
2. Or wait for it to be released
3. Or configure different ports

### Auto-Reload Not Working

**Check if cargo-watch is installed:**
```bash
cargo watch --version
```

**Install if missing:**
```bash
cargo install cargo-watch
```

### Processes Not Stopping

```bash
# Force stop all
cargo xtask dev full-stop-all

# Clean stale entries
cargo xtask dev full-clean
```

### Python Script Errors

**Install dependencies:**
```bash
pip install -r scripts/dev/requirements.txt
```

**Check Python version (requires 3.7+):**
```bash
python --version
```

---

## Environment Variables

### Logging Levels

```bash
# Set log level
RUST_LOG=debug cargo xtask dev full
RUST_LOG=trace cargo xtask dev full-server
RUST_LOG=info,silmaril_networking=debug cargo xtask dev full
```

### Profiling

```bash
# Enable profiling
ENGINE_PROFILE=1 cargo xtask dev full
```

### Validation

```bash
# Enable Vulkan validation
VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation cargo xtask dev full-client
```

---

## Integration with IDE

### VS Code

**Launch configurations** (`.vscode/launch.json`):

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Client",
      "cargo": {
        "args": ["build", "--bin", "client"]
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug"
      }
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Server",
      "cargo": {
        "args": ["build", "--bin", "server"]
      },
      "args": [],
      "cwd": "${workspaceFolder}",
      "env": {
        "RUST_LOG": "debug"
      }
    }
  ]
}
```

**Tasks** (`.vscode/tasks.json`):

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Dev Environment",
      "type": "shell",
      "command": "cargo xtask dev full",
      "problemMatcher": [],
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    },
    {
      "label": "Dev Client Only",
      "type": "shell",
      "command": "cargo xtask dev full-client",
      "problemMatcher": []
    },
    {
      "label": "Dev Server Only",
      "type": "shell",
      "command": "cargo xtask dev full-server",
      "problemMatcher": []
    }
  ]
}
```

### Terminal Workflow

Recommended terminal setup:

**Terminal 1:**
```bash
cargo xtask dev full  # Main dev environment
```

**Terminal 2:**
```bash
cargo watch -x test  # Continuous testing
```

**Terminal 3:**
```bash
# For manual testing/debugging
cargo run --bin client
```

---

## Performance Tips

### Faster Builds

```bash
# Use mold linker (Linux)
sudo apt install mold
export RUSTFLAGS="-C link-arg=-fuse-ld=mold"

# Use lld linker (Windows/macOS)
export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
```

### Faster Cargo Watch

Exclude unnecessary directories:

```bash
cargo watch \
  -x 'run --bin server' \
  -i target/ \
  -i .git/ \
  -i docs/
```

### Parallel Compilation

```bash
# Use all CPU cores
export CARGO_BUILD_JOBS=$(nproc)  # Linux
export CARGO_BUILD_JOBS=$(sysctl -n hw.ncpu)  # macOS
```

---

## Best Practices

### 1. Always Check Status First

```bash
cargo xtask dev full-status  # Check before starting
```

### 2. Use Auto-Reload

Install `cargo-watch` for the best experience.

### 3. Stop Cleanly

Press `Ctrl+C` instead of killing processes manually.

### 4. Clean Regularly

```bash
cargo xtask dev full-clean  # Weekly cleanup
```

### 5. Check Ports

If dev won't start, check port availability:
```bash
cargo xtask dev full-status
```

---

## Next Steps

- Read full documentation: [docs/development-workflow.md](development-workflow.md)
- Learn about testing: [docs/testing-strategy.md](testing-strategy.md)
- Understand architecture: [docs/architecture.md](architecture.md)
- Follow coding standards: [docs/rules/coding-standards.md](rules/coding-standards.md)

---

**Last Updated:** 2026-02-01
