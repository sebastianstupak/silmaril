# Development Workflow Quick Start

> **Fast reference for the `just dev` workflow system**

---

## Installation

### 1. Install Just (Command Runner)

```bash
cargo install just
```

### 2. Install Cargo Watch (Auto-Reload)

```bash
cargo install cargo-watch
```

### 3. Install Python Dependencies

```bash
pip install -r scripts/dev/requirements.txt
```

---

## Common Commands

### Basic Development

```bash
# Start full dev environment (client + server)
just dev

# Client only
just dev-client

# Server only
just dev-server
```

### Enhanced Modes

```bash
# Pretty log formatting
just dev-logs-live

# With profiler (Puffin)
just dev-profiler

# Debug mode (full symbols)
just dev-debug

# Release mode (fast)
just dev-release

# With validation layers
just dev-validation
```

### Multiplayer Testing

```bash
# Run 3 clients + 1 server
just dev-multi 3
```

### Utilities

```bash
# Check status
just dev-status

# Stop all processes
just dev-stop-all

# Clean everything
just dev-clean

# Quick benchmarks
just dev-benchmark
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

### `just dev`
**Full development environment**
- Runs both client and server
- Auto-reload enabled
- Best for general development

### `just dev-client`
**Client development**
- Only runs client
- Useful for UI/rendering work
- Connect to separate server

### `just dev-server`
**Server development**
- Only runs server
- Useful for game logic work
- Test with separate client

### `just dev-logs-live`
**Log analysis**
- Pretty-printed logs
- Color-coded by level (ERROR=red, WARN=yellow, INFO=green)
- Filter by module

### `just dev-profiler`
**Performance profiling**
- Puffin profiler enabled
- Connect viewer to localhost:8585
- Real-time performance metrics

### `just dev-debug`
**Debugging**
- Full debug symbols
- Ready for debugger attachment
- Shows debugger instructions

### `just dev-release`
**Performance testing**
- Optimized build
- Still allows profiling
- Faster than dev builds

### `just dev-validation`
**Bug hunting**
- Vulkan validation layers
- Extra error checking
- Memory leak detection
- Slower but catches issues early

### `just dev-headless`
**CI/Testing**
- No window/renderer
- Faster startup
- Good for automated testing

### `just dev-multi <count>`
**Multiplayer testing**
- Spawns N clients + 1 server
- Each on different port
- Local multiplayer testing

---

## Troubleshooting

### Port Already in Use

```bash
# Check which ports are in use
just dev-status

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
just dev-stop-all

# Clean stale entries
just dev-clean
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
RUST_LOG=debug just dev
RUST_LOG=trace just dev-server
RUST_LOG=info,agent_game_engine_networking=debug just dev
```

### Profiling

```bash
# Enable profiling
ENGINE_PROFILE=1 just dev
```

### Validation

```bash
# Enable Vulkan validation
VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation just dev-client
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
      "command": "just dev",
      "problemMatcher": [],
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    },
    {
      "label": "Dev Client Only",
      "type": "shell",
      "command": "just dev-client",
      "problemMatcher": []
    },
    {
      "label": "Dev Server Only",
      "type": "shell",
      "command": "just dev-server",
      "problemMatcher": []
    }
  ]
}
```

### Terminal Workflow

Recommended terminal setup:

**Terminal 1:**
```bash
just dev  # Main dev environment
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
just dev-status  # Check before starting
```

### 2. Use Auto-Reload

Install `cargo-watch` for the best experience.

### 3. Stop Cleanly

Press `Ctrl+C` instead of killing processes manually.

### 4. Clean Regularly

```bash
just dev-clean  # Weekly cleanup
```

### 5. Check Ports

If dev won't start, check port availability:
```bash
just dev-status
```

---

## Next Steps

- Read full documentation: [docs/development-workflow.md](development-workflow.md)
- Learn about testing: [docs/testing-strategy.md](testing-strategy.md)
- Understand architecture: [docs/architecture.md](architecture.md)
- Follow coding standards: [docs/rules/coding-standards.md](rules/coding-standards.md)

---

**Last Updated:** 2026-02-01
