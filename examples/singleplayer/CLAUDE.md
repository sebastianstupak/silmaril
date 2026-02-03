# Singleplayer Example Game

## Overview
This example demonstrates a complete singleplayer game built with the Silmaril. It showcases core ECS patterns, agent behaviors, and game loop mechanics in a simple, focused context.

## What This Demonstrates
- Basic ECS architecture with components, systems, and entities
- Agent AI behaviors using the agent system
- Game loop and update cycles
- Asset loading and management
- Input handling for player-controlled entities
- Simple physics and collision detection
- State management for singleplayer games

## Features
- Player-controlled character with basic movement
- AI-controlled NPCs with simple behaviors
- Collision detection and response
- Simple combat or interaction system
- Asset management (sprites, sounds)

## Related Documentation
- **Architecture**: `../../.claude/tasks/architecture.md` - Core engine architecture
- **Implementation**: `../../.claude/tasks/phase5-singleplayer-example.md` - Detailed implementation plan

## Building and Running

### Prerequisites
- Rust 1.70 or later
- Cargo

### Build
```bash
cd examples/singleplayer
cargo build --release
```

### Run
```bash
cargo run --release
```

### Development
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test
```

## Project Structure
```
singleplayer/
├── src/
│   └── main.rs          # Game entry point
├── assets/              # Game assets (sprites, sounds, etc.)
├── Cargo.toml          # Project dependencies
└── README.md           # User-facing documentation
```

## Controls
- Arrow Keys / WASD: Move player
- Space: Interact/Attack
- ESC: Pause/Menu

## Next Steps
After exploring this example, consider:
- Extending agent behaviors with more complex AI
- Adding new game mechanics (inventory, quests, etc.)
- Implementing save/load functionality
- Creating custom components and systems
