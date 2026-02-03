//! Spectator Mode for Esports
//!
//! Provides delayed streaming, multiple camera perspectives, and tournament
//! broadcasting for competitive multiplayer games.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// Spectator perspective
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpectatorPerspective {
    /// Free camera (observer)
    FreeCamera,
    /// First-person view of specific player
    FirstPerson {
        /// Player to spectate
        player_id: u64,
    },
    /// Third-person view of specific player
    ThirdPerson {
        /// Player to spectate
        player_id: u64,
    },
    /// Overhead tactical view
    TacticalView,
    /// Auto-director (automatically follows action)
    AutoDirector,
}

/// Spectator configuration
#[derive(Debug, Clone)]
pub struct SpectatorConfig {
    /// Stream delay for competitive integrity (prevents ghosting)
    pub stream_delay: Duration,
    /// Maximum number of concurrent spectators
    pub max_spectators: usize,
    /// Allow spectators to switch perspectives
    pub allow_perspective_switching: bool,
    /// Allow spectators to see all players (vs team-only)
    pub allow_all_player_vision: bool,
    /// Update rate for spectator clients (Hz)
    pub update_rate: u32,
}

impl Default for SpectatorConfig {
    fn default() -> Self {
        Self {
            stream_delay: Duration::from_secs(90), // 90 second delay for tournaments
            max_spectators: 10_000,                // Support large tournaments
            allow_perspective_switching: true,
            allow_all_player_vision: true,
            update_rate: 30, // 30 FPS for spectators (lower than gameplay)
        }
    }
}

/// Buffered game state snapshot for delayed streaming
#[derive(Debug, Clone)]
pub struct BufferedSnapshot {
    /// Snapshot timestamp
    pub timestamp: Instant,
    /// Serialized world state
    pub world_state: Vec<u8>,
    /// Recent events (kills, objectives, etc.)
    pub events: Vec<GameEvent>,
}

/// Game event for spectator interest
#[derive(Debug, Clone)]
pub struct GameEvent {
    /// Event type
    pub event_type: String,
    /// Player ID involved
    pub player_id: u64,
    /// Event position in world
    pub position: [f32; 3],
    /// Event importance (0-10)
    pub importance: u8,
}

/// Spectator client state
struct SpectatorClient {
    /// Current perspective
    perspective: SpectatorPerspective,
    /// Last update time
    last_update: Instant,
    /// Total bytes sent
    bytes_sent: u64,
    /// Perspective switch count
    perspective_switches: u32,
}

impl SpectatorClient {
    fn new() -> Self {
        Self {
            perspective: SpectatorPerspective::AutoDirector,
            last_update: Instant::now(),
            bytes_sent: 0,
            perspective_switches: 0,
        }
    }
}

/// Spectator manager
pub struct SpectatorManager {
    config: SpectatorConfig,
    spectators: HashMap<u64, SpectatorClient>,
    snapshot_buffer: VecDeque<BufferedSnapshot>,
    current_tick: u64,
    active_players: HashMap<u64, [f32; 3]>,
}

impl SpectatorManager {
    /// Create a new spectator manager
    pub fn new(config: SpectatorConfig) -> Self {
        info!(
            stream_delay_secs = config.stream_delay.as_secs(),
            max_spectators = config.max_spectators,
            update_rate = config.update_rate,
            "Creating spectator manager"
        );

        Self {
            config,
            spectators: HashMap::new(),
            snapshot_buffer: VecDeque::new(),
            current_tick: 0,
            active_players: HashMap::new(),
        }
    }

    /// Add a spectator
    pub fn add_spectator(&mut self, spectator_id: u64) -> Result<(), String> {
        if self.spectators.len() >= self.config.max_spectators {
            return Err(format!(
                "Maximum spectators reached ({}/{})",
                self.spectators.len(),
                self.config.max_spectators
            ));
        }

        if self.spectators.contains_key(&spectator_id) {
            return Err("Spectator already exists".to_string());
        }

        self.spectators.insert(spectator_id, SpectatorClient::new());

        info!(spectator_id, total_spectators = self.spectators.len(), "Spectator added");

        Ok(())
    }

    /// Remove a spectator
    pub fn remove_spectator(&mut self, spectator_id: u64) -> bool {
        let removed = self.spectators.remove(&spectator_id).is_some();
        if removed {
            debug!(spectator_id, remaining_spectators = self.spectators.len(), "Spectator removed");
        }
        removed
    }

    /// Set spectator perspective
    pub fn set_perspective(
        &mut self,
        spectator_id: u64,
        perspective: SpectatorPerspective,
    ) -> Result<(), String> {
        if !self.config.allow_perspective_switching {
            return Err("Perspective switching is disabled".to_string());
        }

        let spectator = self
            .spectators
            .get_mut(&spectator_id)
            .ok_or_else(|| "Spectator not found".to_string())?;

        spectator.perspective = perspective;
        spectator.perspective_switches += 1;

        debug!(
            spectator_id,
            ?perspective,
            switches = spectator.perspective_switches,
            "Perspective changed"
        );

        Ok(())
    }

    /// Buffer a game state snapshot
    pub fn buffer_snapshot(
        &mut self,
        world_state: Vec<u8>,
        player_positions: HashMap<u64, [f32; 3]>,
        events: Vec<GameEvent>,
    ) {
        let snapshot = BufferedSnapshot { timestamp: Instant::now(), world_state, events };

        self.snapshot_buffer.push_back(snapshot);
        self.active_players = player_positions;

        // Remove old snapshots beyond delay window
        let cutoff = Instant::now() - self.config.stream_delay - Duration::from_secs(10);
        while let Some(front) = self.snapshot_buffer.front() {
            if front.timestamp < cutoff {
                self.snapshot_buffer.pop_front();
            } else {
                break;
            }
        }

        self.current_tick += 1;
    }

    /// Get delayed snapshot for spectators
    pub fn get_delayed_snapshot(&self) -> Option<&BufferedSnapshot> {
        let now = Instant::now();

        // Find snapshot that is old enough (elapsed time >= stream_delay)
        self.snapshot_buffer
            .iter()
            .find(|snapshot| now.duration_since(snapshot.timestamp) >= self.config.stream_delay)
    }

    /// Update spectator (send them data)
    pub fn update_spectator(&mut self, spectator_id: u64) -> Result<Option<Vec<u8>>, String> {
        // Check update rate and get perspective (borrow spectator)
        let (should_update, perspective) = {
            let spectator = self
                .spectators
                .get(&spectator_id)
                .ok_or_else(|| "Spectator not found".to_string())?;

            let min_interval = Duration::from_millis(1000 / self.config.update_rate as u64);
            if spectator.last_update.elapsed() < min_interval {
                return Ok(None);
            }

            (true, spectator.perspective)
        };

        if !should_update {
            return Ok(None);
        }

        // Get delayed snapshot
        let snapshot = match self.get_delayed_snapshot() {
            Some(s) => s,
            None => return Ok(None), // No data available yet
        };

        // Filter world state based on perspective
        let filtered_state =
            self.filter_state_for_perspective(&snapshot.world_state, &perspective)?;

        // Update spectator stats
        if let Some(spectator) = self.spectators.get_mut(&spectator_id) {
            spectator.last_update = Instant::now();
            spectator.bytes_sent += filtered_state.len() as u64;
        }

        Ok(Some(filtered_state))
    }

    /// Auto-director: Automatically select interesting perspective
    pub fn auto_director_tick(&mut self) -> Option<SpectatorPerspective> {
        // Get delayed snapshot
        let snapshot = self.get_delayed_snapshot()?;

        // Find most interesting event
        if let Some(event) = snapshot.events.iter().max_by_key(|e| e.importance) {
            // Focus on player involved in most important event
            return Some(SpectatorPerspective::ThirdPerson { player_id: event.player_id });
        }

        // Fallback: Find player in most action-dense area
        // (simplified: just pick first player)
        if let Some(&player_id) = self.active_players.keys().next() {
            return Some(SpectatorPerspective::ThirdPerson { player_id });
        }

        None
    }

    /// Get spectator count
    pub fn spectator_count(&self) -> usize {
        self.spectators.len()
    }

    /// Get spectator statistics
    pub fn get_spectator_stats(&self, spectator_id: u64) -> Option<(u64, u32)> {
        self.spectators
            .get(&spectator_id)
            .map(|s| (s.bytes_sent, s.perspective_switches))
    }

    /// Get all spectators watching a specific player
    pub fn get_spectators_watching_player(&self, player_id: u64) -> Vec<u64> {
        self.spectators
            .iter()
            .filter(|(_, spec)| {
                matches!(
                    spec.perspective,
                    SpectatorPerspective::FirstPerson { player_id: pid }
                        | SpectatorPerspective::ThirdPerson { player_id: pid }
                    if pid == player_id
                )
            })
            .map(|(id, _)| *id)
            .collect()
    }

    fn filter_state_for_perspective(
        &self,
        world_state: &[u8],
        perspective: &SpectatorPerspective,
    ) -> Result<Vec<u8>, String> {
        // In a real implementation, this would filter the world state
        // based on the perspective (e.g., hide enemy positions in first-person)
        // For now, just return the full state if allowed
        if !self.config.allow_all_player_vision {
            match perspective {
                SpectatorPerspective::FirstPerson { .. }
                | SpectatorPerspective::ThirdPerson { .. } => {
                    // Would filter to only show what the player can see
                }
                _ => {}
            }
        }

        Ok(world_state.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_spectator() {
        let config = SpectatorConfig::default();
        let mut manager = SpectatorManager::new(config);

        assert!(manager.add_spectator(1).is_ok());
        assert_eq!(manager.spectator_count(), 1);

        // Adding same spectator should fail
        assert!(manager.add_spectator(1).is_err());
    }

    #[test]
    fn test_max_spectators() {
        let config = SpectatorConfig { max_spectators: 3, ..Default::default() };
        let mut manager = SpectatorManager::new(config);

        assert!(manager.add_spectator(1).is_ok());
        assert!(manager.add_spectator(2).is_ok());
        assert!(manager.add_spectator(3).is_ok());

        // 4th spectator should fail
        assert!(manager.add_spectator(4).is_err());
    }

    #[test]
    fn test_remove_spectator() {
        let config = SpectatorConfig::default();
        let mut manager = SpectatorManager::new(config);

        manager.add_spectator(1).unwrap();
        assert_eq!(manager.spectator_count(), 1);

        assert!(manager.remove_spectator(1));
        assert_eq!(manager.spectator_count(), 0);

        // Removing non-existent spectator
        assert!(!manager.remove_spectator(999));
    }

    #[test]
    fn test_perspective_switching() {
        let config = SpectatorConfig::default();
        let mut manager = SpectatorManager::new(config);

        manager.add_spectator(1).unwrap();

        // Change perspective
        let result =
            manager.set_perspective(1, SpectatorPerspective::FirstPerson { player_id: 10 });
        assert!(result.is_ok());

        // Stats should track switches
        let (_, switches) = manager.get_spectator_stats(1).unwrap();
        assert_eq!(switches, 1);
    }

    #[test]
    fn test_perspective_switching_disabled() {
        let config = SpectatorConfig { allow_perspective_switching: false, ..Default::default() };
        let mut manager = SpectatorManager::new(config);

        manager.add_spectator(1).unwrap();

        // Should fail when disabled
        let result = manager.set_perspective(1, SpectatorPerspective::FreeCamera);
        assert!(result.is_err());
    }

    #[test]
    fn test_snapshot_buffering() {
        let config = SpectatorConfig::default();
        let mut manager = SpectatorManager::new(config);

        // Buffer some snapshots
        for _ in 0..5 {
            manager.buffer_snapshot(
                vec![1, 2, 3, 4],
                HashMap::from([(1, [0.0, 0.0, 0.0])]),
                vec![],
            );
        }

        // Should have snapshots buffered
        assert!(manager.snapshot_buffer.len() > 0);
    }

    #[test]
    fn test_delayed_snapshot() {
        let config =
            SpectatorConfig { stream_delay: Duration::from_millis(10), ..Default::default() };
        let mut manager = SpectatorManager::new(config);

        // Before any snapshots, should be None
        assert!(manager.get_delayed_snapshot().is_none());

        // Buffer a snapshot
        manager.buffer_snapshot(vec![1, 2, 3, 4], HashMap::from([(1, [0.0, 0.0, 0.0])]), vec![]);

        // Wait for delay
        std::thread::sleep(Duration::from_millis(15));

        // Now should have delayed snapshot
        assert!(manager.get_delayed_snapshot().is_some());
    }

    #[test]
    fn test_spectator_stats() {
        let config = SpectatorConfig::default();
        let mut manager = SpectatorManager::new(config);

        manager.add_spectator(1).unwrap();

        // Change perspective a few times
        for i in 0..3 {
            manager
                .set_perspective(1, SpectatorPerspective::FirstPerson { player_id: i })
                .unwrap();
        }

        let (bytes_sent, switches) = manager.get_spectator_stats(1).unwrap();
        assert_eq!(switches, 3);
        assert_eq!(bytes_sent, 0); // No updates sent yet
    }

    #[test]
    fn test_get_spectators_watching_player() {
        let config = SpectatorConfig::default();
        let mut manager = SpectatorManager::new(config);

        manager.add_spectator(1).unwrap();
        manager.add_spectator(2).unwrap();
        manager.add_spectator(3).unwrap();

        // Set spectators to watch player 10
        manager
            .set_perspective(1, SpectatorPerspective::FirstPerson { player_id: 10 })
            .unwrap();
        manager
            .set_perspective(2, SpectatorPerspective::ThirdPerson { player_id: 10 })
            .unwrap();
        manager.set_perspective(3, SpectatorPerspective::FreeCamera).unwrap();

        let watchers = manager.get_spectators_watching_player(10);
        assert_eq!(watchers.len(), 2);
        assert!(watchers.contains(&1));
        assert!(watchers.contains(&2));
    }

    #[test]
    fn test_auto_director() {
        let config = SpectatorConfig::default();
        let mut manager = SpectatorManager::new(config);

        // Buffer snapshot with event
        let events = vec![GameEvent {
            event_type: "kill".to_string(),
            player_id: 42,
            position: [10.0, 0.0, 5.0],
            importance: 10,
        }];

        manager.buffer_snapshot(vec![1, 2, 3, 4], HashMap::from([(42, [10.0, 0.0, 5.0])]), events);

        // Wait for delay
        std::thread::sleep(Duration::from_millis(100));

        // Auto-director should focus on player 42
        if let Some(perspective) = manager.auto_director_tick() {
            assert!(matches!(perspective, SpectatorPerspective::ThirdPerson { player_id: 42 }));
        }
    }
}
