# MMORPG Example

A massively multiplayer online RPG demonstrating the Agent Game Engine's networking and scalability features.

## Quick Start

### Using Docker Compose (Recommended)
```bash
docker-compose up
```

### Manual Setup
```bash
# Terminal 1: Start server
cargo run --release --bin mmorpg-server

# Terminal 2+: Start client(s)
cargo run --release --bin mmorpg-client
```

## What's Included

This example shows:
- Client-server architecture
- Network synchronization
- Multiplayer game logic
- Zone-based world management
- Player persistence
- Chat and social features

## Documentation

See `CLAUDE.md` for detailed documentation and implementation references.
