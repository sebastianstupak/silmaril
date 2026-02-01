//! Change Detection System
//!
//! Provides efficient tracking of component modifications to enable
//! queries that only process changed entities (10-100x speedup potential).
//!
//! # Design
//!
//! - World maintains a global "tick" counter
//! - Each component stores "last_changed" tick
//! - Systems track "last_run" tick
//! - `Changed<T>` filter only returns components modified since last run
//!
//! # Example
//!
//! ```rust
//! use engine_core::{World, Component};
//!
//! #[derive(Component)]
//! struct Transform { x: f32, y: f32, z: f32 }
//!
//! fn update_physics(world: &mut World) {
//!     // Only processes entities whose Transform changed
//!     for (_entity, transform) in world.query::<(&Transform, Changed<Transform>)>() {
//!         // 100x fewer iterations if only 1% of transforms change!
//!     }
//! }
//! ```

use std::marker::PhantomData;

/// Global tick counter for change detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(u64);

impl Tick {
    /// Create a new tick with value 0
    pub const fn new() -> Self {
        Self(0)
    }

    /// Create a tick with a specific value (for testing)
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw tick value
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Increment the tick
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    /// Check if this tick is newer than another
    pub fn is_newer_than(self, other: Tick) -> bool {
        self.0 > other.0
    }

    /// Check if this tick changed since another tick
    pub fn changed_since(self, last_check: Tick) -> bool {
        self.0 > last_check.0
    }
}

impl Default for Tick {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker type for change detection queries
///
/// When used in a query, only returns components that have been modified
/// since the last time the system ran.
///
/// # Example
///
/// ```rust
/// # use engine_core::{World, Component};
/// # #[derive(Component)]
/// # struct Transform { x: f32 }
/// fn system(world: &mut World) {
///     // Only iterates over entities with modified Transform
///     for (_entity, transform) in world.query::<(&Transform, Changed<Transform>)>() {
///         // Process only changed entities
///     }
/// }
/// ```
pub struct Changed<T> {
    _marker: PhantomData<T>,
}

/// Component metadata for change tracking
#[derive(Debug, Clone, Copy)]
pub struct ComponentTicks {
    /// Tick when this component was added
    pub added: Tick,
    /// Tick when this component was last modified
    pub changed: Tick,
}

impl ComponentTicks {
    /// Create new component ticks
    pub fn new(current_tick: Tick) -> Self {
        Self {
            added: current_tick,
            changed: current_tick,
        }
    }

    /// Mark this component as changed
    pub fn set_changed(&mut self, tick: Tick) {
        self.changed = tick;
    }

    /// Check if this component was added since the given tick
    pub fn is_added(&self, last_tick: Tick) -> bool {
        self.added.is_newer_than(last_tick)
    }

    /// Check if this component changed since the given tick
    pub fn is_changed(&self, last_tick: Tick) -> bool {
        self.changed.is_newer_than(last_tick)
    }
}

/// System metadata for change detection
#[derive(Debug, Clone, Copy)]
pub struct SystemTicks {
    /// Tick when this system last ran
    pub last_run: Tick,
}

impl SystemTicks {
    /// Create new system ticks
    pub fn new() -> Self {
        Self {
            last_run: Tick::new(),
        }
    }

    /// Update the last run tick
    pub fn update(&mut self, current_tick: Tick) {
        self.last_run = current_tick;
    }

    /// Get the last run tick
    pub fn last_run(&self) -> Tick {
        self.last_run
    }
}

impl Default for SystemTicks {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_increment() {
        let mut tick = Tick::new();
        assert_eq!(tick.get(), 0);

        tick.increment();
        assert_eq!(tick.get(), 1);

        tick.increment();
        assert_eq!(tick.get(), 2);
    }

    #[test]
    fn test_tick_comparison() {
        let tick1 = Tick::from_raw(5);
        let tick2 = Tick::from_raw(10);

        assert!(tick2.is_newer_than(tick1));
        assert!(!tick1.is_newer_than(tick2));
        assert!(tick2.changed_since(tick1));
    }

    #[test]
    fn test_component_ticks() {
        let tick1 = Tick::from_raw(5);
        let tick2 = Tick::from_raw(10);

        let mut comp_ticks = ComponentTicks::new(tick1);
        assert_eq!(comp_ticks.added, tick1);
        assert_eq!(comp_ticks.changed, tick1);

        comp_ticks.set_changed(tick2);
        assert_eq!(comp_ticks.changed, tick2);
        assert!(comp_ticks.is_changed(tick1));
    }

    #[test]
    fn test_system_ticks() {
        let tick1 = Tick::from_raw(5);
        let tick2 = Tick::from_raw(10);

        let mut sys_ticks = SystemTicks::new();
        assert_eq!(sys_ticks.last_run(), Tick::new());

        sys_ticks.update(tick1);
        assert_eq!(sys_ticks.last_run(), tick1);

        sys_ticks.update(tick2);
        assert_eq!(sys_ticks.last_run(), tick2);
    }

    #[test]
    fn test_tick_wrapping() {
        let mut tick = Tick::from_raw(u64::MAX - 1);
        tick.increment();
        assert_eq!(tick.get(), u64::MAX);

        tick.increment();
        assert_eq!(tick.get(), 0); // Wraps around
    }
}
