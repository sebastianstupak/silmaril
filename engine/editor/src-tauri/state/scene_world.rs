//! Shared ECS world state for the editor scene.
//!
//! Wraps `engine_core::World` in an `Arc<RwLock<_>>` so it can be safely
//! shared between the Tauri command thread and the Vulkan render thread.

use std::sync::{Arc, RwLock};

use engine_core::World;

/// Tauri managed state holding the live ECS world for the current scene.
pub struct SceneWorldState(pub Arc<RwLock<World>>);

impl SceneWorldState {
    pub fn new() -> Self {
        let mut world = World::new();
        // Pre-register components used by the editor so `world.add()` never panics.
        world.register::<engine_core::Transform>();
        Self(Arc::new(RwLock::new(world)))
    }
}
