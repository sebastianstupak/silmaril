# Agent Game Engine - Examples

This directory contains complete example games demonstrating different capabilities of the Agent Game Engine.

## Available Examples

### 1. Singleplayer (`singleplayer/`)
A singleplayer game showcasing core ECS architecture, agent behaviors, and game loop mechanics.

**Features:**
- Player-controlled character
- AI-controlled NPCs
- Basic physics and collision
- Asset management

**Quick Start:**
```bash
cd singleplayer
cargo run --release
```

**Documentation:** See `singleplayer/CLAUDE.md`

---

### 2. MMORPG (`mmorpg/`)
A massively multiplayer online RPG demonstrating advanced networking and server architecture.

**Features:**
- Client-server architecture
- Multiple concurrent players
- Zone-based world management
- Persistence
- Chat and social features

**Quick Start:**
```bash
cd mmorpg
docker-compose up
```

**Documentation:** See `mmorpg/CLAUDE.md`

---

### 3. Turn-Based Strategy (`turn-based/`)
A turn-based strategy game showing state management and strategic AI.

**Features:**
- Grid-based tactics
- Turn-based game loop
- Strategic AI decision-making
- Undo/redo functionality
- Action point systems

**Quick Start:**
```bash
cd turn-based
cargo run --release
```

**Documentation:** See `turn-based/CLAUDE.md`

---

### 4. MOBA (`moba/`)
A Multiplayer Online Battle Arena showcasing real-time competitive multiplayer.

**Features:**
- Real-time 5v5 combat
- Hero abilities and items
- Interest management
- Matchmaking system
- Replay system
- Spectator mode

**Quick Start:**
```bash
cd moba
docker-compose up
```

**Documentation:** See `moba/CLAUDE.md`

---

## Learning Path

We recommend exploring the examples in this order:

1. **Start with `singleplayer/`** - Learn the basics of the engine, ECS patterns, and agent behaviors
2. **Move to `turn-based/`** - Understand state management and strategic AI
3. **Try `mmorpg/`** - Learn networking fundamentals and client-server architecture
4. **Finish with `moba/`** - Explore advanced networking, interest management, and competitive features

## General Structure

Each example follows a similar structure:

```
example-name/
├── CLAUDE.md           # Detailed documentation and references
├── README.md          # Quick start guide
├── Cargo.toml         # Rust dependencies
├── src/               # Source code
└── assets/            # Game assets (where applicable)
```

Multiplayer examples (MMORPG, MOBA) use a workspace structure:

```
example-name/
├── CLAUDE.md
├── README.md
├── Cargo.toml         # Workspace configuration
├── docker-compose.yml # Docker deployment
├── client/            # Client application
├── server/            # Server application
└── shared/            # Shared code (protocol, entities)
```

## Contributing

When creating new examples:

1. Follow the existing structure
2. Create a comprehensive CLAUDE.md with:
   - Overview and features
   - What the example demonstrates
   - Related documentation references
   - Build/run instructions
   - Next steps for learners
3. Keep examples focused on specific features
4. Include clear comments in code
5. Provide assets or asset placeholders

## Documentation References

All examples reference task files in `.claude/tasks/`:
- Core architecture: `architecture.md`
- Networking: `phase1-networking-foundation.md`, `phase2-*.md`, etc.
- Example implementations: `phase5-*.md`

See each example's CLAUDE.md for specific documentation references.
