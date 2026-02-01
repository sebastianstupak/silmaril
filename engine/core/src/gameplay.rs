//! Gameplay-related components

use crate::ecs::Component;
use serde::{Deserialize, Serialize};

/// Health component - current and max health
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Health {
    /// Current health points
    pub current: f32,
    /// Maximum health points
    pub max: f32,
}

impl Component for Health {}

impl Health {
    /// Create a new health component
    pub const fn new(current: f32, max: f32) -> Self {
        Self { current, max }
    }

    /// Check if entity is alive
    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    /// Check if entity is at full health
    pub fn is_full(&self) -> bool {
        self.current >= self.max
    }

    /// Damage the entity
    pub fn damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    /// Heal the entity
    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_new() {
        let h = Health::new(75.0, 100.0);
        assert_eq!(h.current, 75.0);
        assert_eq!(h.max, 100.0);
    }

    #[test]
    fn test_health_is_alive() {
        let alive = Health::new(50.0, 100.0);
        let dead = Health::new(0.0, 100.0);

        assert!(alive.is_alive());
        assert!(!dead.is_alive());
    }

    #[test]
    fn test_health_damage() {
        let mut h = Health::new(100.0, 100.0);
        h.damage(30.0);
        assert_eq!(h.current, 70.0);

        h.damage(100.0); // Overkill
        assert_eq!(h.current, 0.0);
    }

    #[test]
    fn test_health_heal() {
        let mut h = Health::new(50.0, 100.0);
        h.heal(30.0);
        assert_eq!(h.current, 80.0);

        h.heal(100.0); // Overheal capped at max
        assert_eq!(h.current, 100.0);
    }

    #[test]
    fn test_health_is_full() {
        let full = Health::new(100.0, 100.0);
        let not_full = Health::new(50.0, 100.0);

        assert!(full.is_full());
        assert!(!not_full.is_full());
    }
}
