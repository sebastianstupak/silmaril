# MMORPG Example

## Overview
This example demonstrates a massively multiplayer online RPG built with the Silmaril. It showcases advanced networking, server architecture, client-server communication, and scalable multiplayer game design.

## What This Demonstrates
- Client-server architecture for MMORPGs
- Networking protocols and message passing
- Server-side game logic and validation
- Client-side prediction and interpolation
- Player authentication and session management
- Zone/area management for large worlds
- Entity replication and synchronization
- Scalable server architecture with load balancing
- Persistent world state
- Chat and social features

## Features
- Multiple concurrent players in a shared world
- Real-time player movement and interactions
- Server-authoritative game logic
- Client-side prediction for responsive controls
- Zone-based world partitioning
- Player chat and parties
- Basic combat system
- NPC spawning and AI
- Persistence (player data, world state)

## Related Documentation
- **Implementation**: `../../.claude/tasks/phase5-mmorpg-example.md` - MMORPG-specific implementation
- **Networking Foundations**: `../../.claude/tasks/phase1-networking-foundation.md` - Core networking
- **Message Protocol**: `../../.claude/tasks/phase2-message-protocol.md` - Network messages
- **Entity Sync**: `../../.claude/tasks/phase2-entity-sync.md` - Entity replication
- **Architecture**: `../../.claude/tasks/architecture.md` - Core engine architecture

## Building and Running

### Prerequisites
- Rust 1.70 or later
- Docker and Docker Compose (for containerized deployment)
- PostgreSQL (for persistence) or use Docker setup

### Build
```bash
cd examples/mmorpg

# Build all components
cargo build --release

# Or build individually
cargo build --release --bin mmorpg-server
cargo build --release --bin mmorpg-client
```

### Run with Docker Compose
```bash
# Start server, database, and supporting services
docker-compose up

# The server will be available on the configured port (default: 7777)
```

### Run Manually

#### Start Server
```bash
# Run server
cargo run --release --bin mmorpg-server

# Or with custom configuration
RUST_LOG=info SERVER_PORT=7777 cargo run --release --bin mmorpg-server
```

#### Start Client(s)
```bash
# Run client
cargo run --release --bin mmorpg-client

# Connect to custom server
SERVER_HOST=localhost SERVER_PORT=7777 cargo run --release --bin mmorpg-client
```

### Development
```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin mmorpg-server

# Run tests
cargo test

# Run specific component tests
cargo test --bin mmorpg-server
cargo test --bin mmorpg-client
```

## Project Structure
```
mmorpg/
├── client/
│   └── src/
│       └── main.rs      # Client application
├── server/
│   └── src/
│       └── main.rs      # Server application
├── shared/
│   └── src/
│       └── lib.rs       # Shared code (protocol, entities, etc.)
├── Cargo.toml          # Workspace configuration
├── docker-compose.yml  # Docker deployment setup
└── README.md          # User-facing documentation
```

## Client Controls
- Arrow Keys / WASD: Move character
- Mouse Click: Interact with world/NPCs
- T: Open chat
- I: Open inventory
- ESC: Menu

## Server Configuration

Key environment variables:
- `SERVER_PORT`: Port to listen on (default: 7777)
- `MAX_PLAYERS`: Maximum concurrent players (default: 1000)
- `DATABASE_URL`: PostgreSQL connection string
- `RUST_LOG`: Logging level (debug, info, warn, error)

## Architecture Notes

### Client
- Handles rendering and input
- Predicts player movement locally
- Receives authoritative state from server
- Interpolates other players' positions

### Server
- Authoritative game state
- Validates all client actions
- Broadcasts state updates to clients
- Manages zones/areas for scalability
- Handles persistence

### Shared
- Common data structures (entities, components)
- Network protocol definitions
- Serialization/deserialization logic
- Game rules and constants

## Next Steps
After exploring this example, consider:
- Implementing additional game systems (crafting, trading, etc.)
- Adding more sophisticated AI for NPCs
- Implementing instanced dungeons
- Creating a guild/clan system
- Adding voice chat integration
- Implementing anti-cheat measures
- Optimizing for larger player counts
