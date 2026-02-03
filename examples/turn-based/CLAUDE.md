# Turn-Based Strategy Example

## Overview
This example demonstrates a turn-based strategy game built with the Silmaril. It showcases turn-based game logic, state management, AI decision-making, and strategic gameplay mechanics.

## What This Demonstrates
- Turn-based game loop and state management
- Strategic AI decision-making
- Grid-based movement and positioning
- Action point/resource management
- Turn order and initiative systems
- Fog of war implementation
- Undo/redo functionality
- Save/load game state
- Replay system

## Features
- Grid-based tactical combat
- Multiple unit types with unique abilities
- AI opponents with strategic planning
- Turn-based resource management
- Terrain effects and line of sight
- Victory conditions and objectives
- Campaign or scenario mode
- Turn replay and review

## Related Documentation
- **Implementation**: `../../.claude/tasks/phase5-turnbased-example.md` - Turn-based specific implementation
- **Architecture**: `../../.claude/tasks/architecture.md` - Core engine architecture
- **Agent System**: For AI decision-making components

## Building and Running

### Prerequisites
- Rust 1.70 or later
- Cargo

### Build
```bash
cd examples/turn-based
cargo build --release
```

### Run
```bash
cargo run --release
```

### Run with specific scenario
```bash
cargo run --release -- --scenario tutorial
cargo run --release -- --scenario campaign01
```

### Development
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run tests
cargo test

# Run AI benchmark
cargo run --release -- --benchmark-ai
```

## Project Structure
```
turn-based/
├── src/
│   └── main.rs          # Game entry point and main loop
├── assets/
│   ├── scenarios/       # Scenario definitions
│   ├── units/          # Unit configurations
│   └── maps/           # Map data
├── Cargo.toml          # Project dependencies
└── README.md           # User-facing documentation
```

## Gameplay Controls
- Mouse Click: Select unit
- Right Click: Show move/attack options
- Space: End turn
- Z: Undo last action
- ESC: Menu
- Tab: Cycle through units

## Game Concepts

### Turn Structure
1. Start of turn phase (income, status effects)
2. Player action phase (move, attack, abilities)
3. End of turn phase (cleanup, AI turn)

### Action Points
Each unit has action points (AP) that limit what they can do per turn:
- Movement costs AP based on distance and terrain
- Attacks and abilities have AP costs
- Unused AP may provide defensive bonuses

### AI Behavior
The AI uses:
- Strategic goal evaluation
- Tactical position scoring
- Minimax or Monte Carlo tree search
- Dynamic difficulty adjustment

## Configuration

Edit configuration in assets/config.toml:
```toml
[game]
grid_size = [20, 20]
turn_time_limit = 60  # seconds, 0 for unlimited

[ai]
difficulty = "medium"  # easy, medium, hard
think_time = 1.0      # seconds
```

## Next Steps
After exploring this example, consider:
- Implementing multiplayer (hot-seat or networked)
- Adding more unit types and abilities
- Creating a level/scenario editor
- Implementing advanced AI strategies
- Adding different victory conditions
- Creating a campaign with story progression
