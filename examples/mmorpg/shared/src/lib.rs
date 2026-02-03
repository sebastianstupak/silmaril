//! MMORPG Shared Library
//!
//! Shared code between client and server including protocol definitions,
//! entity structures, and common game logic.

use engine_core::ecs::Entity;
use serde::{Deserialize, Serialize};

/// Network protocol messages
pub mod protocol {
    use super::*;

    /// Client -> Server messages
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum ClientMessage {
        /// Join the game
        Join { player_name: String },

        /// Move player to position
        Move { x: f32, y: f32 },

        /// Disconnect from game
        Disconnect,
    }

    /// Server -> Client messages
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub enum ServerMessage {
        /// Welcome message with assigned player entity
        Welcome { player_entity: Entity, player_name: String },

        /// Full state snapshot
        StateSnapshot { players: Vec<PlayerState> },

        /// Player joined
        PlayerJoined { entity: Entity, name: String, x: f32, y: f32 },

        /// Player moved
        PlayerMoved { entity: Entity, x: f32, y: f32 },

        /// Player left
        PlayerLeft { entity: Entity },
    }
}

/// Shared entity definitions
pub mod entities {
    use super::*;

    /// Player entity state
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Player {
        pub entity: Entity,
        pub name: String,
        pub position: Position,
        pub velocity: Velocity,
    }

    impl Player {
        pub fn new(entity: Entity, name: String, x: f32, y: f32) -> Self {
            Self {
                entity,
                name,
                position: Position { x, y },
                velocity: Velocity { x: 0.0, y: 0.0 },
            }
        }
    }
}

/// Shared component definitions
pub mod components {
    use super::*;

    /// Position component (2D for simplicity)
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct Position {
        pub x: f32,
        pub y: f32,
    }

    impl Position {
        pub fn distance_to(&self, other: &Position) -> f32 {
            let dx = self.x - other.x;
            let dy = self.y - other.y;
            (dx * dx + dy * dy).sqrt()
        }
    }

    /// Velocity component
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct Velocity {
        pub x: f32,
        pub y: f32,
    }
}

/// Game constants and configuration
pub mod config {
    /// Server tick rate (Hz)
    pub const TICK_RATE: u64 = 60;

    /// Tick duration (ms)
    pub const TICK_DURATION_MS: u64 = 1000 / TICK_RATE;

    /// World bounds
    pub const WORLD_WIDTH: f32 = 1000.0;
    pub const WORLD_HEIGHT: f32 = 1000.0;

    /// Movement speed (units per second)
    pub const MOVE_SPEED: f32 = 100.0;

    /// Maximum players per server
    pub const MAX_PLAYERS: usize = 100;

    /// Default spawn position
    pub const SPAWN_X: f32 = WORLD_WIDTH / 2.0;
    pub const SPAWN_Y: f32 = WORLD_HEIGHT / 2.0;
}

/// Player state for network sync
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub entity: Entity,
    pub name: String,
    pub x: f32,
    pub y: f32,
}

// Re-export commonly used types
pub use components::{Position, Velocity};
pub use entities::Player;
pub use protocol::{ClientMessage, ServerMessage};
