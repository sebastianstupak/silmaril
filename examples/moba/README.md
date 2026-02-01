# MOBA Example

A Multiplayer Online Battle Arena game demonstrating the Agent Game Engine's real-time multiplayer and competitive features.

## Quick Start

### Using Docker Compose (Recommended)
```bash
docker-compose up
```

### Manual Setup
```bash
# Terminal 1: Start server
cargo run --release --bin moba-server

# Terminal 2+: Start client(s)
cargo run --release --bin moba-client
```

## What's Included

This example shows:
- Real-time 5v5 multiplayer combat
- Hero abilities and mechanics
- Interest management for performance
- Matchmaking system
- AI for minions and bots
- Replay system
- Spectator mode

## Documentation

See `CLAUDE.md` for detailed documentation and implementation references.
