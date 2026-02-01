// MOBA Shared Library
//
// Shared code between client and server including hero definitions,
// ability systems, game rules, and network protocol.

/// Network protocol messages
pub mod protocol {
    // TODO: Define message types
    // - PlayerInput (movement, abilities, items)
    // - EntityUpdate (positions, health, etc.)
    // - AbilityUsage
    // - CombatEvent
    // - MinimapUpdate
    // - etc.
}

/// Hero definitions and abilities
pub mod heroes {
    // TODO: Define hero types
    // - Stats (health, mana, damage, etc.)
    // - Abilities (Q, W, E, R)
    // - Passive effects
    // - Scaling factors
}

/// Game entities
pub mod entities {
    // TODO: Define entity types
    // - Hero
    // - Minion
    // - Tower
    // - Jungle camp
    // - Projectile
    // - etc.
}

/// Component definitions
pub mod components {
    // TODO: Define components
    // - Position
    // - Health/Mana
    // - Abilities
    // - Inventory
    // - Buffs/Debuffs
    // - Team
    // - etc.
}

/// Combat and ability systems
pub mod combat {
    // TODO: Damage calculation
    // TODO: Ability effect resolution
    // TODO: Status effect handling
    // TODO: Cooldown management
}

/// Game constants and configuration
pub mod config {
    // TODO: Define constants
    // - Tick rate
    // - Map dimensions
    // - Minion spawn times
    // - Tower stats
    // - Game rules
    // - etc.
}

/// Matchmaking data structures
pub mod matchmaking {
    // TODO: Player rating (MMR/ELO)
    // TODO: Queue data
    // TODO: Team composition rules
}
