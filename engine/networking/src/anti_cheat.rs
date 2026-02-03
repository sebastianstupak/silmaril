//! Anti-Cheat Integration Framework
//!
//! Provides server-side validation, anomaly detection, and hooks for
//! integration with anti-cheat systems for competitive multiplayer.

use std::collections::VecDeque;
use std::time::Instant;
use tracing::{debug, warn};

/// Anti-cheat validation result
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Action is valid
    Valid,
    /// Action is suspicious (log but allow)
    Suspicious {
        /// Why the action is suspicious
        reason: String,
        /// Severity level (0-10)
        severity: u8,
    },
    /// Action is invalid (reject and log)
    Invalid {
        /// Why the action is invalid
        reason: String,
    },
    /// Action indicates cheating (disconnect player)
    Cheating {
        /// Why this is considered cheating
        reason: String,
        /// Confidence level (0.0-1.0)
        confidence: f32,
    },
}

/// Player action for validation
#[derive(Debug, Clone)]
pub struct PlayerAction {
    /// Action timestamp
    pub timestamp: Instant,
    /// Action type (e.g., "move", "shoot", "jump")
    pub action_type: String,
    /// Action data (position, velocity, etc.)
    pub data: Vec<f32>,
}

/// Movement validation statistics
#[derive(Debug, Clone)]
struct MovementStats {
    max_speed: f32,
    total_distance: f32,
    direction_changes: u32,
    teleports: u32,
    last_position: Option<[f32; 3]>,
    last_update: Instant,
}

impl MovementStats {
    fn new() -> Self {
        Self {
            max_speed: 0.0,
            total_distance: 0.0,
            direction_changes: 0,
            teleports: 0,
            last_position: None,
            last_update: Instant::now(),
        }
    }
}

/// Combat validation statistics
#[derive(Debug, Clone)]
struct CombatStats {
    shots_fired: u32,
    shots_hit: u32,
    headshots: u32,
    last_shot: Option<Instant>,
}

impl CombatStats {
    fn new() -> Self {
        Self { shots_fired: 0, shots_hit: 0, headshots: 0, last_shot: None }
    }

    fn accuracy(&self) -> f32 {
        if self.shots_fired == 0 {
            0.0
        } else {
            self.shots_hit as f32 / self.shots_fired as f32
        }
    }

    fn headshot_ratio(&self) -> f32 {
        if self.shots_hit == 0 {
            0.0
        } else {
            self.headshots as f32 / self.shots_hit as f32
        }
    }
}

/// Anti-cheat validator configuration
#[derive(Debug, Clone)]
pub struct AntiCheatConfig {
    /// Maximum allowed player speed (units/sec)
    pub max_speed: f32,
    /// Maximum allowed acceleration (units/sec^2)
    pub max_acceleration: f32,
    /// Maximum teleport distance before flagging (units)
    pub max_teleport_distance: f32,
    /// Suspicious accuracy threshold (0.0-1.0)
    pub suspicious_accuracy: f32,
    /// Cheating accuracy threshold (0.0-1.0)
    pub cheating_accuracy: f32,
    /// Suspicious headshot ratio (0.0-1.0)
    pub suspicious_headshot_ratio: f32,
    /// Minimum time between shots (milliseconds)
    pub min_shot_interval: u64,
    /// Action history size for anomaly detection
    pub action_history_size: usize,
}

impl Default for AntiCheatConfig {
    fn default() -> Self {
        Self {
            max_speed: 10.0,                // 10 units/sec
            max_acceleration: 20.0,         // 20 units/sec^2
            max_teleport_distance: 5.0,     // 5 units
            suspicious_accuracy: 0.8,       // 80% accuracy
            cheating_accuracy: 0.95,        // 95% accuracy
            suspicious_headshot_ratio: 0.7, // 70% headshots
            min_shot_interval: 50,          // 50ms between shots
            action_history_size: 100,
        }
    }
}

/// Per-player anti-cheat validator
pub struct PlayerValidator {
    config: AntiCheatConfig,
    movement_stats: MovementStats,
    combat_stats: CombatStats,
    action_history: VecDeque<PlayerAction>,
    violation_count: u32,
    last_violation: Option<Instant>,
}

impl PlayerValidator {
    /// Create a new player validator
    pub fn new(config: AntiCheatConfig) -> Self {
        Self {
            config,
            movement_stats: MovementStats::new(),
            combat_stats: CombatStats::new(),
            action_history: VecDeque::new(),
            violation_count: 0,
            last_violation: None,
        }
    }

    /// Validate a movement action
    pub fn validate_movement(
        &mut self,
        position: [f32; 3],
        velocity: [f32; 3],
    ) -> ValidationResult {
        let now = Instant::now();

        // Calculate speed
        let speed = (velocity[0].powi(2) + velocity[1].powi(2) + velocity[2].powi(2)).sqrt();

        // Check speed limit
        if speed > self.config.max_speed {
            self.record_violation();
            return ValidationResult::Invalid {
                reason: format!("Speed too high: {:.2} > {:.2}", speed, self.config.max_speed),
            };
        }

        // Check for teleportation
        if let Some(last_pos) = self.movement_stats.last_position {
            let dx = position[0] - last_pos[0];
            let dy = position[1] - last_pos[1];
            let dz = position[2] - last_pos[2];
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            let dt = now.duration_since(self.movement_stats.last_update).as_secs_f32();
            let expected_max_distance =
                self.config.max_speed * dt + self.config.max_teleport_distance;

            if distance > expected_max_distance && distance > self.config.max_teleport_distance {
                self.movement_stats.teleports += 1;
                self.record_violation();

                // Multiple teleports indicate cheating
                if self.movement_stats.teleports >= 3 {
                    return ValidationResult::Cheating {
                        reason: format!(
                            "Multiple teleports detected ({})",
                            self.movement_stats.teleports
                        ),
                        confidence: 0.9,
                    };
                }

                return ValidationResult::Suspicious {
                    reason: format!("Possible teleport: {:.2} units in {:.2}s", distance, dt),
                    severity: 7,
                };
            }

            self.movement_stats.total_distance += distance;
        }

        // Update stats
        self.movement_stats.max_speed = self.movement_stats.max_speed.max(speed);
        self.movement_stats.last_position = Some(position);
        self.movement_stats.last_update = now;

        // Record action
        self.record_action(PlayerAction {
            timestamp: now,
            action_type: "move".to_string(),
            data: vec![
                position[0],
                position[1],
                position[2],
                velocity[0],
                velocity[1],
                velocity[2],
            ],
        });

        ValidationResult::Valid
    }

    /// Validate a combat action (shooting)
    pub fn validate_shot(
        &mut self,
        hit: bool,
        headshot: bool,
        target_distance: f32,
    ) -> ValidationResult {
        let now = Instant::now();

        // Check shot interval
        if let Some(last_shot) = self.combat_stats.last_shot {
            let interval = now.duration_since(last_shot);
            if interval.as_millis() < self.config.min_shot_interval as u128 {
                self.record_violation();
                return ValidationResult::Invalid {
                    reason: format!(
                        "Shot interval too short: {}ms < {}ms",
                        interval.as_millis(),
                        self.config.min_shot_interval
                    ),
                };
            }
        }

        // Update stats
        self.combat_stats.shots_fired += 1;
        if hit {
            self.combat_stats.shots_hit += 1;
            if headshot {
                self.combat_stats.headshots += 1;
            }
        }
        self.combat_stats.last_shot = Some(now);

        // Record action
        self.record_action(PlayerAction {
            timestamp: now,
            action_type: "shoot".to_string(),
            data: vec![hit as u8 as f32, headshot as u8 as f32, target_distance],
        });

        // Check accuracy after minimum sample size
        if self.combat_stats.shots_fired >= 20 {
            let accuracy = self.combat_stats.accuracy();
            let headshot_ratio = self.combat_stats.headshot_ratio();

            // Check for aimbot indicators
            if accuracy >= self.config.cheating_accuracy {
                self.record_violation();
                return ValidationResult::Cheating {
                    reason: format!("Suspicious accuracy: {:.1}%", accuracy * 100.0),
                    confidence: 0.85,
                };
            }

            if headshot_ratio >= self.config.suspicious_headshot_ratio
                && accuracy >= self.config.suspicious_accuracy
            {
                return ValidationResult::Suspicious {
                    reason: format!(
                        "High accuracy with high headshot ratio: {:.1}% acc, {:.1}% HS",
                        accuracy * 100.0,
                        headshot_ratio * 100.0
                    ),
                    severity: 8,
                };
            }
        }

        ValidationResult::Valid
    }

    /// Get current combat statistics
    pub fn get_combat_stats(&self) -> (u32, u32, u32, f32, f32) {
        (
            self.combat_stats.shots_fired,
            self.combat_stats.shots_hit,
            self.combat_stats.headshots,
            self.combat_stats.accuracy(),
            self.combat_stats.headshot_ratio(),
        )
    }

    /// Get current movement statistics
    pub fn get_movement_stats(&self) -> (f32, f32, u32, u32) {
        (
            self.movement_stats.max_speed,
            self.movement_stats.total_distance,
            self.movement_stats.direction_changes,
            self.movement_stats.teleports,
        )
    }

    /// Get violation count
    pub fn get_violation_count(&self) -> u32 {
        self.violation_count
    }

    /// Reset statistics (e.g., new match)
    pub fn reset(&mut self) {
        self.movement_stats = MovementStats::new();
        self.combat_stats = CombatStats::new();
        self.action_history.clear();
        self.violation_count = 0;
        self.last_violation = None;
    }

    fn record_action(&mut self, action: PlayerAction) {
        self.action_history.push_back(action);
        if self.action_history.len() > self.config.action_history_size {
            self.action_history.pop_front();
        }
    }

    fn record_violation(&mut self) {
        self.violation_count += 1;
        self.last_violation = Some(Instant::now());
    }
}

/// Anti-cheat manager for all players
pub struct AntiCheatManager {
    config: AntiCheatConfig,
    validators: std::collections::HashMap<u64, PlayerValidator>,
}

impl AntiCheatManager {
    /// Create a new anti-cheat manager
    pub fn new(config: AntiCheatConfig) -> Self {
        debug!("Creating anti-cheat manager");
        Self { config, validators: std::collections::HashMap::new() }
    }

    /// Register a player for validation
    pub fn register_player(&mut self, player_id: u64) {
        debug!(player_id, "Registering player for anti-cheat validation");
        self.validators.insert(player_id, PlayerValidator::new(self.config.clone()));
    }

    /// Unregister a player
    pub fn unregister_player(&mut self, player_id: u64) {
        self.validators.remove(&player_id);
    }

    /// Validate player movement
    pub fn validate_movement(
        &mut self,
        player_id: u64,
        position: [f32; 3],
        velocity: [f32; 3],
    ) -> ValidationResult {
        if let Some(validator) = self.validators.get_mut(&player_id) {
            let result = validator.validate_movement(position, velocity);

            // Log violations
            match &result {
                ValidationResult::Suspicious { reason, severity } => {
                    warn!(player_id, %reason, severity, "Suspicious movement detected");
                }
                ValidationResult::Invalid { reason } => {
                    warn!(player_id, %reason, "Invalid movement detected");
                }
                ValidationResult::Cheating { reason, confidence } => {
                    warn!(player_id, %reason, confidence, "Cheating detected - movement");
                }
                _ => {}
            }

            result
        } else {
            ValidationResult::Invalid { reason: "Player not registered for validation".to_string() }
        }
    }

    /// Validate player shot
    pub fn validate_shot(
        &mut self,
        player_id: u64,
        hit: bool,
        headshot: bool,
        target_distance: f32,
    ) -> ValidationResult {
        if let Some(validator) = self.validators.get_mut(&player_id) {
            let result = validator.validate_shot(hit, headshot, target_distance);

            // Log violations
            match &result {
                ValidationResult::Suspicious { reason, severity } => {
                    warn!(player_id, %reason, severity, "Suspicious combat detected");
                }
                ValidationResult::Invalid { reason } => {
                    warn!(player_id, %reason, "Invalid combat action");
                }
                ValidationResult::Cheating { reason, confidence } => {
                    warn!(player_id, %reason, confidence, "Cheating detected - aimbot");
                }
                _ => {}
            }

            result
        } else {
            ValidationResult::Invalid { reason: "Player not registered for validation".to_string() }
        }
    }

    /// Get player combat statistics
    pub fn get_player_combat_stats(&self, player_id: u64) -> Option<(u32, u32, u32, f32, f32)> {
        self.validators.get(&player_id).map(|v| v.get_combat_stats())
    }

    /// Get player movement statistics
    pub fn get_player_movement_stats(&self, player_id: u64) -> Option<(f32, f32, u32, u32)> {
        self.validators.get(&player_id).map(|v| v.get_movement_stats())
    }

    /// Get player violation count
    pub fn get_player_violations(&self, player_id: u64) -> Option<u32> {
        self.validators.get(&player_id).map(|v| v.get_violation_count())
    }

    /// Reset player statistics
    pub fn reset_player(&mut self, player_id: u64) {
        if let Some(validator) = self.validators.get_mut(&player_id) {
            validator.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_movement() {
        let config = AntiCheatConfig::default();
        let mut validator = PlayerValidator::new(config);

        let result = validator.validate_movement([0.0, 0.0, 0.0], [5.0, 0.0, 0.0]);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_excessive_speed() {
        let config = AntiCheatConfig { max_speed: 10.0, ..Default::default() };
        let mut validator = PlayerValidator::new(config);

        let result = validator.validate_movement([0.0, 0.0, 0.0], [20.0, 0.0, 0.0]);
        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }

    #[test]
    fn test_teleport_detection() {
        let config = AntiCheatConfig::default();
        let mut validator = PlayerValidator::new(config);

        // Normal movement
        validator.validate_movement([0.0, 0.0, 0.0], [5.0, 0.0, 0.0]);

        // Wait a bit
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Teleport
        let result = validator.validate_movement([100.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        assert!(matches!(
            result,
            ValidationResult::Suspicious { .. } | ValidationResult::Cheating { .. }
        ));
    }

    #[test]
    fn test_valid_shooting() {
        let config = AntiCheatConfig::default();
        let mut validator = PlayerValidator::new(config);

        let result = validator.validate_shot(true, false, 10.0);
        assert_eq!(result, ValidationResult::Valid);
    }

    #[test]
    fn test_rapid_fire_detection() {
        let config = AntiCheatConfig { min_shot_interval: 100, ..Default::default() };
        let mut validator = PlayerValidator::new(config);

        // First shot OK
        assert_eq!(validator.validate_shot(true, false, 10.0), ValidationResult::Valid);

        // Immediate second shot should fail
        let result = validator.validate_shot(true, false, 10.0);
        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }

    #[test]
    fn test_aimbot_detection() {
        let config = AntiCheatConfig { cheating_accuracy: 0.95, ..Default::default() };
        let mut validator = PlayerValidator::new(config);

        // Simulate suspiciously high accuracy
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(101));
            validator.validate_shot(true, false, 10.0); // 100% accuracy
        }

        // Should detect cheating
        std::thread::sleep(std::time::Duration::from_millis(101));
        let result = validator.validate_shot(true, true, 10.0);
        assert!(matches!(result, ValidationResult::Cheating { .. }));
    }

    #[test]
    fn test_combat_stats() {
        let config = AntiCheatConfig::default();
        let mut validator = PlayerValidator::new(config);

        // Simulate some combat
        std::thread::sleep(std::time::Duration::from_millis(101));
        validator.validate_shot(true, false, 10.0);
        std::thread::sleep(std::time::Duration::from_millis(101));
        validator.validate_shot(false, false, 15.0);
        std::thread::sleep(std::time::Duration::from_millis(101));
        validator.validate_shot(true, true, 8.0);

        let (fired, hit, headshots, accuracy, hs_ratio) = validator.get_combat_stats();
        assert_eq!(fired, 3);
        assert_eq!(hit, 2);
        assert_eq!(headshots, 1);
        assert!((accuracy - 0.666).abs() < 0.01);
        assert!((hs_ratio - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_manager_registration() {
        let config = AntiCheatConfig::default();
        let mut manager = AntiCheatManager::new(config);

        manager.register_player(1);
        manager.register_player(2);

        let result = manager.validate_movement(1, [0.0, 0.0, 0.0], [5.0, 0.0, 0.0]);
        assert_eq!(result, ValidationResult::Valid);

        manager.unregister_player(1);

        let result = manager.validate_movement(1, [0.0, 0.0, 0.0], [5.0, 0.0, 0.0]);
        assert!(matches!(result, ValidationResult::Invalid { .. }));
    }
}
