//! Physics configuration module
//!
//! Provides runtime configuration for physics behavior.

use engine_math::Vec3;
use serde::{Deserialize, Serialize};

/// Physics execution mode (runtime decision)
///
/// This determines how physics simulation is executed across client/server.
/// The same PhysicsWorld code runs everywhere - behavior changes based on mode.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PhysicsMode {
    /// Server simulates authoritatively, clients interpolate received state
    ///
    /// Best for: MMOs, authoritative gameplay
    /// Network: Server sends full state updates
    /// Performance: Server pays full cost, clients minimal
    ServerAuthoritative,

    /// Client predicts locally, server reconciles
    ///
    /// Best for: Fast-paced games (FPS, racing)
    /// Network: Client sends inputs, server sends corrections
    /// Performance: Both simulate, bandwidth reduced
    ClientPrediction {
        /// Threshold for triggering reconciliation (distance in meters)
        reconciliation_threshold: f32,

        /// Number of frames to keep for rollback
        history_frames: u32,
    },

    /// Both client and server simulate identically (lockstep)
    ///
    /// Best for: RTS, MOBA, fighting games
    /// Network: Only inputs sent, deterministic simulation
    /// Performance: Both simulate, minimal bandwidth
    /// **Requires**: Deterministic math (fixed-point or careful f32)
    Deterministic {
        /// Use fixed-point math (slower but deterministic)
        use_fixed_point: bool,
    },

    /// Local simulation only (singleplayer, editor)
    ///
    /// Best for: Offline games, testing
    /// Network: None
    /// Performance: Full simulation locally
    LocalOnly,

    /// Physics disabled (UI-only games, no gameplay physics)
    Disabled,
}

/// Physics world configuration
///
/// Controls runtime behavior of physics simulation.
/// Can be loaded from config files or set programmatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    /// Execution mode
    pub mode: PhysicsMode,

    /// Gravity vector (m/s²)
    /// Default: Vec3::new(0.0, -9.81, 0.0)
    pub gravity: Vec3,

    /// Physics timestep in Hz (60 = 60 updates/second)
    /// Default: 60
    pub timestep_hz: u32,

    /// Maximum sub-steps per frame (prevents spiral of death)
    /// Default: 4
    pub max_substeps: u32,

    /// Enable Continuous Collision Detection (CCD)
    /// Prevents fast-moving objects from tunneling
    /// Default: true
    pub enable_ccd: bool,

    /// Number of solver iterations (higher = more stable, slower)
    /// Default: 8
    pub solver_iterations: u32,

    /// Enable parallel physics (uses rayon)
    /// Only useful for 500+ bodies
    /// Default: true
    pub enable_parallel: bool,

    /// Enable SIMD optimizations
    /// Requires CPU support (detected at runtime)
    /// Default: true
    pub enable_simd: bool,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            mode: PhysicsMode::LocalOnly, // Safe default
            gravity: Vec3::new(0.0, -9.81, 0.0),
            timestep_hz: 60,
            max_substeps: 4,
            enable_ccd: true,
            solver_iterations: 8,
            enable_parallel: true,
            enable_simd: true,
        }
    }
}

impl PhysicsConfig {
    /// Create configuration for server-authoritative mode
    pub fn server_authoritative() -> Self {
        Self { mode: PhysicsMode::ServerAuthoritative, ..Default::default() }
    }

    /// Create configuration for client-side prediction
    pub fn client_prediction(reconciliation_threshold: f32) -> Self {
        Self {
            mode: PhysicsMode::ClientPrediction {
                reconciliation_threshold,
                history_frames: 60, // 1 second at 60Hz
            },
            ..Default::default()
        }
    }

    /// Create configuration for deterministic lockstep
    pub fn deterministic(use_fixed_point: bool) -> Self {
        Self {
            mode: PhysicsMode::Deterministic { use_fixed_point },
            // Deterministic mode needs stricter settings
            enable_parallel: false, // Parallelism breaks determinism
            enable_simd: false,     // SIMD may vary across CPUs
            solver_iterations: 10,  // More iterations for stability
            ..Default::default()
        }
    }

    /// Get timestep in seconds
    pub fn timestep(&self) -> f32 {
        1.0 / self.timestep_hz as f32
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.timestep_hz == 0 {
            return Err("timestep_hz must be > 0");
        }

        if self.max_substeps == 0 {
            return Err("max_substeps must be > 0");
        }

        if self.solver_iterations == 0 {
            return Err("solver_iterations must be > 0");
        }

        if let PhysicsMode::ClientPrediction { reconciliation_threshold, history_frames } =
            self.mode
        {
            if reconciliation_threshold <= 0.0 {
                return Err("reconciliation_threshold must be > 0");
            }
            if history_frames == 0 {
                return Err("history_frames must be > 0");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_physics_config_defaults() {
        let config = PhysicsConfig::default();
        assert_eq!(config.mode, PhysicsMode::LocalOnly);
        assert_eq!(config.timestep_hz, 60);
        assert_eq!(config.timestep(), 1.0 / 60.0);
        assert!(config.enable_ccd);
        assert!(config.enable_parallel);
        assert!(config.enable_simd);
    }

    #[test]
    fn test_server_authoritative_config() {
        let config = PhysicsConfig::server_authoritative();
        assert!(matches!(config.mode, PhysicsMode::ServerAuthoritative));
        assert_eq!(config.gravity.y, -9.81);
    }

    #[test]
    fn test_client_prediction_config() {
        let config = PhysicsConfig::client_prediction(0.1);
        match config.mode {
            PhysicsMode::ClientPrediction { reconciliation_threshold, history_frames } => {
                assert_eq!(reconciliation_threshold, 0.1);
                assert_eq!(history_frames, 60);
            }
            _ => panic!("Expected ClientPrediction mode"),
        }
    }

    #[test]
    fn test_deterministic_disables_parallelism() {
        let config = PhysicsConfig::deterministic(false);
        assert!(matches!(config.mode, PhysicsMode::Deterministic { use_fixed_point: false }));
        assert!(!config.enable_parallel); // Must be disabled for determinism
        assert!(!config.enable_simd); // Must be disabled for determinism
        assert_eq!(config.solver_iterations, 10); // Higher for stability
    }

    #[test]
    fn test_config_validation() {
        let mut config = PhysicsConfig::default();
        assert!(config.validate().is_ok());

        config.timestep_hz = 0;
        assert!(config.validate().is_err());

        config = PhysicsConfig::default();
        config.max_substeps = 0;
        assert!(config.validate().is_err());

        config = PhysicsConfig::default();
        config.solver_iterations = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_timestep_calculation() {
        let mut config = PhysicsConfig::default();

        config.timestep_hz = 60;
        assert!((config.timestep() - 1.0 / 60.0).abs() < 0.0001);

        config.timestep_hz = 120;
        assert!((config.timestep() - 1.0 / 120.0).abs() < 0.0001);

        config.timestep_hz = 30;
        assert!((config.timestep() - 1.0 / 30.0).abs() < 0.0001);
    }

    #[test]
    fn test_config_serialization() {
        let config = PhysicsConfig::server_authoritative();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: PhysicsConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(config.mode, deserialized.mode);
        assert_eq!(config.timestep_hz, deserialized.timestep_hz);
    }
}
