# MOBA Example

## Overview
This example demonstrates a Multiplayer Online Battle Arena (MOBA) game built with the Silmaril. It showcases real-time multiplayer combat, team coordination, advanced AI behaviors, lane mechanics, and competitive gameplay systems.

## What This Demonstrates
- Real-time multiplayer combat (5v5 or similar)
- Client-server architecture for competitive games
- Advanced interest management for optimized networking
- Team-based gameplay mechanics
- Lane and minion systems
- Hero abilities and cooldown management
- AI for computer-controlled heroes and minions
- Matchmaking and lobby systems
- Spectator mode
- Replay recording and playback
- Anti-cheat measures

## Features
- Multiple heroes with unique abilities
- Three-lane map with towers and bases
- AI-controlled minions
- Jungle camps and objectives
- Team coordination mechanics
- Real-time combat with skill shots
- Item shop and inventory
- Experience and leveling system
- Matchmaking and ranked play
- In-game voice/text chat

## Related Documentation
- **Implementation**: `../../.claude/tasks/phase5-moba-example.md` - MOBA-specific implementation
- **Interest Management**: `../../.claude/tasks/phase3-interest-advanced.md` - Advanced networking optimization
- **Networking**: Core networking phases for multiplayer functionality
- **Architecture**: `../../.claude/tasks/architecture.md` - Core engine architecture

## Building and Running

### Prerequisites
- Rust 1.70 or later
- Docker and Docker Compose (for containerized deployment)
- PostgreSQL (for matchmaking and player data)

### Build
```bash
cd examples/moba

# Build all components
cargo build --release

# Or build individually
cargo build --release --bin moba-server
cargo build --release --bin moba-client
```

### Run with Docker Compose
```bash
# Start server, matchmaking, and supporting services
docker-compose up

# The game server will be available on the configured port (default: 7778)
```

### Run Manually

#### Start Server
```bash
# Run dedicated server
cargo run --release --bin moba-server

# Or with custom configuration
RUST_LOG=info SERVER_PORT=7778 MAX_PLAYERS=10 cargo run --release --bin moba-server
```

#### Start Client(s)
```bash
# Run client
cargo run --release --bin moba-client

# Connect to custom server
SERVER_HOST=localhost SERVER_PORT=7778 cargo run --release --bin moba-client
```

### Development
```bash
# Run with debug logging
RUST_LOG=debug cargo run --bin moba-server

# Run tests
cargo test

# Run bot match (for AI testing)
cargo run --release --bin moba-server -- --bot-match

# Run performance profiling
cargo run --release --bin moba-server -- --profile
```

## Project Structure
```
moba/
├── client/
│   └── src/
│       └── main.rs      # Client application
├── server/
│   └── src/
│       └── main.rs      # Game server and matchmaking
├── shared/
│   └── src/
│       └── lib.rs       # Shared code (heroes, abilities, game rules)
├── Cargo.toml          # Workspace configuration
├── docker-compose.yml  # Docker deployment setup
└── README.md          # User-facing documentation
```

## Client Controls
- Right Click: Move/Attack
- Q/W/E/R: Hero abilities
- D/F: Summoner spells
- 1-6: Item hotkeys
- Tab: Scoreboard
- Y: Team chat
- Enter: All chat
- B: Recall to base

## Server Configuration

Key environment variables:
- `SERVER_PORT`: Port to listen on (default: 7778)
- `MAX_PLAYERS`: Maximum players per match (default: 10)
- `TICK_RATE`: Server update rate (default: 30 Hz)
- `MATCHMAKING_ENABLED`: Enable matchmaking (default: true)
- `DATABASE_URL`: PostgreSQL connection string
- `RUST_LOG`: Logging level

## Architecture Notes

### Client
- Renders game state with interpolation
- Handles input and ability usage
- Predicts player actions for responsiveness
- Receives authoritative state from server
- Implements spectator camera

### Server
- Authoritative game state (50-60 tick rate)
- Validates all client actions
- Processes hero abilities and combat
- Manages minion spawning and AI
- Implements interest management for network efficiency
- Handles matchmaking and lobby creation
- Records replays

### Shared
- Hero definitions and ability data
- Game rules and constants (damage, cooldowns, etc.)
- Map layout and objective data
- Network protocol
- Matchmaking algorithms

## Game Flow

1. **Lobby**: Players queue for matchmaking
2. **Draft**: Hero selection phase (optional)
3. **Loading**: Load map and initialize game state
4. **Game Start**: 0:00 - Minions spawn at 1:00
5. **Laning Phase**: Early game farming and trading
6. **Mid Game**: Team fights and objective control
7. **Late Game**: Push for victory
8. **Victory/Defeat**: Game ends, stats recorded

## Interest Management

The server uses advanced interest management to optimize network traffic:
- Players only receive updates for nearby entities
- Different update rates for different zones (e.g., fog of war)
- Priority system for critical updates (hero abilities, combat)
- Bandwidth allocation based on player connection quality

## Next Steps
After exploring this example, consider:
- Adding more heroes with unique mechanics
- Implementing ranked matchmaking with ELO
- Creating custom game modes
- Adding replay analysis tools
- Implementing tournament/spectator features
- Optimizing for esports requirements
- Adding cosmetics and progression systems
- Implementing behavior score/trust system
