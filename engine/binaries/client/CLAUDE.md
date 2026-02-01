# Client Binary

## Purpose

The client binary is the player-facing application that handles rendering, input, client-side prediction, and communication with the game server. It provides a responsive gaming experience by predicting player movement locally and reconciling with authoritative server state.

## Architecture

The client is built on top of the engine's core systems:

- **ECS Core**: Entity management and component storage
- **Renderer**: Vulkan-based rendering pipeline for visuals
- **Networking Client**: TCP/UDP connection to game server
- **Prediction System**: Client-side movement prediction and reconciliation
- **Input System**: Captures and processes player input
- **Audio Engine**: 3D spatial audio playback

### Data Flow

```
Input Capture
    |
    v
Client Prediction (apply locally)
    |
    +---> Send to Server (UDP)
    |
    v
Receive Server Update
    |
    v
Reconcile State (if mismatch)
    |
    v
Render Frame
    |
    v
Optional: Capture Frame (for agents)
```

## Feature Flags

The client binary is compiled with the `client` feature flag enabled:

```toml
[features]
default = ["client"]
client = []
```

This ensures that:
- Client-only code (rendering, UI, etc.) is included
- Server-only code (physics simulation, AI, etc.) is excluded
- Shared code (ECS, math, etc.) is available

### Component Availability

With `#[client_only]` macro:
- `MeshRenderer` - Available
- `Camera` - Available
- `AudioListener` - Available
- `InputController` - Available

With `#[server_only]` macro:
- `ServerAuthority` - NOT available
- `PhysicsBackend` - NOT available (client uses prediction only)
- `AIController` - NOT available

With `#[shared]` macro:
- `Transform` - Available
- `Velocity` - Available
- `Health` - Available

## Build Instructions

### Development Build

```bash
# Build client with debug symbols
cargo build --bin client --features client

# Run client
cargo run --bin client --features client
```

### Release Build

```bash
# Build optimized client
cargo build --bin client --features client --release

# Strip symbols for smaller binary
cargo build --bin client --features client --release --config strip=symbols
```

### Build Scripts

Use the provided build scripts:

```bash
# Build client only
./scripts/build-client.sh

# Build both client and server
./scripts/build-both.sh
```

### Docker Build

```bash
# Build Docker image
docker build -f engine/binaries/client/Dockerfile -t agent-game-engine-client .

# Run in container
docker run -it --rm \
  -e DISPLAY=$DISPLAY \
  -v /tmp/.X11-unix:/tmp/.X11-unix \
  agent-game-engine-client
```

## Platform-Specific Considerations

### Windows

- Vulkan SDK required (install from LunarG)
- Visual Studio Build Tools recommended
- DirectX 12 runtime needed for some Vulkan implementations

### Linux

- Vulkan drivers required (mesa-vulkan-drivers or nvidia drivers)
- X11 or Wayland support
- Audio: ALSA or PulseAudio

### macOS

- MoltenVK required (Vulkan-on-Metal)
- Metal runtime (included in macOS)
- Code signing may be required for distribution

### Cross-Compilation

The client supports cross-compilation for all platforms:

```bash
# Windows from Linux
cargo build --bin client --features client --target x86_64-pc-windows-gnu

# Linux from Windows (WSL2)
cargo build --bin client --features client --target x86_64-unknown-linux-gnu

# macOS from Linux (with osxcross)
cargo build --bin client --features client --target x86_64-apple-darwin
```

## Configuration

The client can be configured via:

1. **Config file** (`client_config.toml`):
```toml
[renderer]
backend = "vulkan"
vsync = true
resolution = [1920, 1080]
fullscreen = false

[networking]
server_url = "127.0.0.1:7777"
udp_port = 7778

[input]
mouse_sensitivity = 1.0
```

2. **Environment variables**:
```bash
SERVER_URL=192.168.1.100:7777 cargo run --bin client
RUST_LOG=debug cargo run --bin client
```

3. **Command-line arguments**:
```bash
cargo run --bin client -- --server 192.168.1.100:7777 --resolution 1920x1080
```

## Performance Targets

- **Frame Rate**: 60 FPS minimum (1080p, medium settings)
- **Input Latency**: < 16ms (1 frame)
- **Network Latency**: < 100ms to server
- **Memory Usage**: < 1 GB (typical gameplay)
- **Startup Time**: < 3 seconds (to main menu)

## Agent Integration

The client supports agent-based gameplay:

- **Frame Capture**: Capture rendered frames for analysis
- **Programmatic Control**: Send inputs via API
- **State Inspection**: Export world state to YAML
- **Headless Mode**: Run without rendering for faster simulation

```rust
// Example: Agent-controlled client
let client = Client::new(ClientConfig {
    headless: true,
    capture_frames: true,
    ..Default::default()
})?;

// Agent sends input
client.send_input(PlayerInput {
    movement: Vec3::new(0.0, 0.0, 1.0), // Move forward
    look_delta: Vec3::ZERO,
    buttons: 0,
});

// Capture frame for analysis
if let Some(frame) = client.capture_frame() {
    agent.analyze_frame(frame);
}
```

## Related Documentation

- [D:\dev\agent-game-engine\docs\architecture.md](../../docs/architecture.md) - Overall system architecture
- [D:\dev\agent-game-engine\docs\tasks\phase2-proc-macros.md](../../docs/tasks/phase2-proc-macros.md) - Client/server code splitting with macros
- [D:\dev\agent-game-engine\docs\tasks\phase2-client-prediction.md](../../docs/tasks/phase2-client-prediction.md) - Client-side prediction and reconciliation
- [D:\dev\agent-game-engine\docs\tasks\phase1-vulkan-context.md](../../docs/tasks/phase1-vulkan-context.md) - Vulkan rendering setup
- [D:\dev\agent-game-engine\docs\tasks\phase1-basic-rendering.md](../../docs/tasks/phase1-basic-rendering.md) - Basic rendering pipeline
- [D:\dev\agent-game-engine\docs\tasks\phase1-mesh-rendering.md](../../docs/tasks/phase1-mesh-rendering.md) - Mesh rendering
- [D:\dev\agent-game-engine\docs\tasks\phase1-frame-capture.md](../../docs/tasks/phase1-frame-capture.md) - Frame capture for agents
- [D:\dev\agent-game-engine\docs\platform-abstraction.md](../../docs/platform-abstraction.md) - Cross-platform support
