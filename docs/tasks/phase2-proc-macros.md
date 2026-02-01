# Phase 2.1: Procedural Macros (Client/Server Split)

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Critical (enables code splitting)

---

## 🎯 **Objective**

Implement procedural macros `#[client_only]`, `#[server_only]`, and `#[shared]` to automatically split code between client and server executables at compile time.

**Goal:** Write once, compile separately for client/server.

---

## 📋 **Detailed Tasks**

### **1. Macro Crate Setup** (Day 1)

**File:** `engine/macros/Cargo.toml`

```toml
[package]
name = "agent_game_engine_macros"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
syn = { version = "2.0", features = ["full"] }
quote = "1.0"
proc-macro2 = "1.0"
```

**File:** `engine/macros/src/lib.rs`

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct, ItemImpl};

/// Mark function/struct/impl as client-only
/// Compiles out when building for server
#[proc_macro_attribute]
pub fn client_only(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let output = if cfg!(feature = "client") {
        // Keep code when building client
        item
    } else {
        // Remove code when building server
        TokenStream::new()
    };

    output
}

/// Mark function/struct/impl as server-only
/// Compiles out when building for client
#[proc_macro_attribute]
pub fn server_only(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let output = if cfg!(feature = "server") {
        // Keep code when building server
        item
    } else {
        // Remove code when building client
        TokenStream::new()
    };

    output
}

/// Mark code as shared (available in both client and server)
/// This is a no-op, but makes intent explicit
#[proc_macro_attribute]
pub fn shared(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
```

---

### **2. Feature Flags** (Day 1)

**File:** `Cargo.toml` (workspace root)

```toml
[workspace.dependencies]
agent_game_engine_macros = { path = "engine/macros" }

[features]
default = []
client = []
server = []
```

**File:** `engine/binaries/client/Cargo.toml`

```toml
[package]
name = "client"
version = "0.1.0"
edition = "2021"

[dependencies]
agent_game_engine_core = { path = "../../core", features = ["client"] }
agent_game_engine_renderer = { path = "../../renderer", features = ["client"] }
agent_game_engine_macros = { workspace = true }

[features]
default = ["client"]
client = []
```

**File:** `engine/binaries/server/Cargo.toml`

```toml
[package]
name = "server"
version = "0.1.0"
edition = "2021"

[dependencies]
agent_game_engine_core = { path = "../../core", features = ["server"] }
agent_game_engine_networking = { path = "../../networking", features = ["server"] }
agent_game_engine_macros = { workspace = true }

[features]
default = ["server"]
server = []
```

---

### **3. Advanced Macro Features** (Day 2-3)

**Conditional compilation based on target:**

```rust
/// More sophisticated client_only with span preservation
#[proc_macro_attribute]
pub fn client_only(_attr: TokenStream, item: TokenStream) -> TokenStream {
    #[cfg(feature = "client")]
    {
        item
    }

    #[cfg(not(feature = "client"))]
    {
        // Generate stub or error for missing items
        let parsed = parse_macro_input!(item as syn::Item);

        match parsed {
            syn::Item::Fn(func) => {
                let name = &func.sig.ident;
                let error = format!("Function '{}' is client-only and not available on server", name);

                quote! {
                    compile_error!(#error);
                }
                .into()
            }
            syn::Item::Struct(s) => {
                let name = &s.ident;
                let error = format!("Struct '{}' is client-only and not available on server", name);

                quote! {
                    compile_error!(#error);
                }
                .into()
            }
            _ => {
                quote! {
                    compile_error!("client_only can only be used on functions, structs, or impl blocks");
                }
                .into()
            }
        }
    }
}
```

---

### **4. System Splitting** (Day 3)

**File:** `engine/core/src/systems/mod.rs`

```rust
use agent_game_engine_macros::{client_only, server_only, shared};

/// Movement system (shared - runs on both client and server)
#[shared]
pub fn movement_system(world: &mut World, dt: f32) {
    for (entity, (transform, velocity)) in world.query::<(&mut Transform, &Velocity)>() {
        transform.position += velocity.0 * dt;
    }
}

/// Rendering system (client-only)
#[client_only]
pub fn rendering_system(world: &World, renderer: &mut Renderer) {
    for (entity, (transform, mesh)) in world.query::<(&Transform, &MeshRenderer)>() {
        renderer.draw_mesh(mesh, transform);
    }
}

/// Physics simulation (server-only)
#[server_only]
pub fn physics_system(world: &mut World, physics: &mut PhysicsEngine) {
    for (entity, (transform, rigidbody)) in world.query::<(&mut Transform, &RigidBody)>() {
        // Update physics
        physics.step(entity, rigidbody, transform);
    }
}
```

---

### **5. Component Splitting** (Day 3-4)

Some components only exist on client or server:

```rust
use agent_game_engine_macros::{client_only, server_only, shared};

/// Shared component (both client and server)
#[derive(Component)]
#[shared]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

/// Client-only component
#[derive(Component)]
#[client_only]
pub struct MeshRenderer {
    pub mesh_id: u64,
    pub material_id: u64,
}

/// Client-only component
#[derive(Component)]
#[client_only]
pub struct Camera {
    pub fov: f32,
    pub aspect: f32,
}

/// Server-only component
#[derive(Component)]
#[server_only]
pub struct ServerAuthority {
    pub last_update: u64,
}

/// Server-only component
#[derive(Component)]
#[server_only]
pub struct PlayerConnection {
    pub connection_id: u64,
    pub last_ping: u64,
}
```

---

### **6. Build Scripts** (Day 4)

**File:** `scripts/build-client.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Building client..."
cargo build --bin client --features client --release

echo "Client built successfully!"
```

**File:** `scripts/build-server.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Building server..."
cargo build --bin server --features server --release

echo "Server built successfully!"
```

**File:** `scripts/build-both.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Building both client and server..."

./scripts/build-client.sh
./scripts/build-server.sh

echo "Both binaries built successfully!"
```

---

## ✅ **Acceptance Criteria**

- [ ] `#[client_only]` macro compiles out code on server
- [ ] `#[server_only]` macro compiles out code on client
- [ ] `#[shared]` macro explicit for shared code
- [ ] Feature flags work correctly
- [ ] Client binary doesn't include server code
- [ ] Server binary doesn't include client code
- [ ] Compile errors when trying to use wrong-target code
- [ ] Build scripts for both targets
- [ ] Documentation with examples

---

## 🧪 **Tests**

```rust
#[test]
#[cfg(feature = "client")]
fn test_client_only_code_present() {
    // This test only runs when building for client
    rendering_system(); // Should compile
}

#[test]
#[cfg(feature = "server")]
fn test_server_only_code_present() {
    // This test only runs when building for server
    physics_system(); // Should compile
}

#[test]
fn test_shared_code_always_present() {
    // This test runs for both client and server
    movement_system(); // Should always compile
}
```

---

## 💡 **Usage Examples**

### **Example 1: Split Systems**

```rust
use agent_game_engine_macros::{client_only, server_only};

#[client_only]
pub fn render_ui(world: &World) {
    // Only compiled in client
    // Accesses client-only components like Camera
}

#[server_only]
pub fn broadcast_state(world: &World, connections: &mut Connections) {
    // Only compiled in server
    // Accesses server-only components like PlayerConnection
}
```

### **Example 2: Conditional Impl Blocks**

```rust
#[derive(Component)]
pub struct Player {
    pub id: u64,
    pub name: String,
}

#[client_only]
impl Player {
    pub fn render(&self, renderer: &mut Renderer) {
        // Client-specific rendering logic
    }
}

#[server_only]
impl Player {
    pub fn simulate_ai(&mut self) {
        // Server-specific AI logic
    }
}
```

### **Example 3: Mixed Component**

```rust
#[derive(Component)]
pub struct Health {
    #[shared]
    pub current: f32,

    #[shared]
    pub max: f32,

    #[server_only]
    pub regeneration_rate: f32, // Server calculates regen

    #[client_only]
    pub ui_bar_id: Option<u64>, // Client displays health bar
}
```

---

## 📊 **Binary Size Impact**

With proper code splitting, expect:

| Binary | Without Split | With Split | Reduction |
|--------|--------------|------------|-----------|
| Client | 50 MB | 30 MB | 40% |
| Server | 50 MB | 25 MB | 50% |

Server binary smaller because it excludes:
- Vulkan renderer
- Mesh data structures
- UI code
- Audio mixing (may keep for server-side audio)

Client binary smaller because it excludes:
- Physics simulation (client uses prediction only)
- AI logic
- Server-side validation
- Database code

---

## 🔧 **Troubleshooting**

### **Error: "Feature 'client' not enabled"**

Make sure to build with correct features:
```bash
cargo build --bin client --features client
cargo build --bin server --features server
```

### **Error: "Function X is client-only"**

You're trying to use client-only code in server. Either:
1. Make the code `#[shared]`
2. Create a server-specific version
3. Move logic to appropriate target

---

**Dependencies:** None (first Phase 2 task)
**Next:** [phase2-network-protocol.md](phase2-network-protocol.md)
