# Development Scripts

Helper scripts for the development workflow system.

## Installation

Install Python dependencies:

```bash
pip install -r requirements.txt
```

Or if you prefer using a virtual environment:

```bash
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
```

## Scripts

### orchestrator.py

Main process orchestrator for managing client/server processes.

**Usage:**
```bash
# Run both client and server
python orchestrator.py full

# Run client only
python orchestrator.py client

# Run server only
python orchestrator.py server

# Run multiple clients
python orchestrator.py multi 3
```

**Features:**
- Graceful shutdown on Ctrl+C
- Color-coded output per process
- Automatic cleanup on exit

### port-checker.py

Checks if required ports are available.

**Usage:**
```bash
# Check default ports (7777, 7778, 8080)
python port-checker.py

# Check custom ports
python port-checker.py 8000 8001 8002
```

**Features:**
- Detects which process is using a port
- Cross-platform support (Windows, Linux, macOS)

### process-manager.py

Manages process lifecycle with PID tracking.

**Usage:**
```bash
# Register a process
python process-manager.py register server 12345 "cargo run --bin server"

# Unregister a process
python process-manager.py unregister server

# Check status
python process-manager.py status

# Stop all registered processes
python process-manager.py stop-all

# Clean stale entries
python process-manager.py clean
```

**Features:**
- PID file management
- Process status tracking
- Graceful shutdown with timeout

### log-formatter.py

Pretty-prints structured logs with color coding.

**Usage:**
```bash
# Pipe logs through formatter
cargo run --bin server | python log-formatter.py

# Filter by level
cargo run --bin server | python log-formatter.py --level DEBUG

# Filter by module
cargo run --bin server | python log-formatter.py --module networking

# Disable colors
cargo run --bin server | python log-formatter.py --no-color
```

**Features:**
- Auto-detects JSON and plain text logs
- Color-coded by log level
- Module filtering
- Level filtering

## Platform Support

All scripts are cross-platform and tested on:
- Windows 10/11
- Linux (Ubuntu, Debian, Arch)
- macOS (Intel and Apple Silicon)

## Dependencies

- Python 3.7+
- psutil (for process management)

## Integration with Cargo XTask

These scripts are integrated into the cargo xtask workflow:

```bash
# Use via cargo xtask commands
cargo xtask dev full         # Uses orchestrator.py
cargo xtask dev status       # Uses process-manager.py and port-checker.py
cargo xtask dev logs         # Uses log-formatter.py
```

## Error Handling

All scripts provide clear error messages and exit codes:
- 0: Success
- 1: Error (with explanation)

## Logging

Scripts use structured output with clear symbols:
- ✅ Success
- ❌ Error
- ⚠️  Warning
- ℹ️  Info
- 🚀 Starting
- 🛑 Stopping
- 🧹 Cleaning

## Contributing

When adding new dev scripts:
1. Follow Python 3.7+ syntax
2. Use type hints
3. Add docstrings
4. Handle cross-platform differences
5. Provide clear error messages
6. Update this README
