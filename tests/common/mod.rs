//! Common test utilities and helpers
//!
//! This module provides shared testing infrastructure used across unit tests,
//! integration tests, and benchmarks. It includes mock components, test helpers,
//! custom assertions, and test data builders.

use std::sync::atomic::{AtomicU32, Ordering};

// =============================================================================
// Mock Components
// =============================================================================

/// Mock position component for testing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MockPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl MockPosition {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn distance_to(&self, other: &Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl Default for MockPosition {
    fn default() -> Self {
        Self::zero()
    }
}

/// Mock velocity component for testing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MockVelocity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl MockVelocity {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

impl Default for MockVelocity {
    fn default() -> Self {
        Self::zero()
    }
}

/// Mock health component for testing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MockHealth {
    pub current: i32,
    pub max: i32,
}

impl MockHealth {
    pub fn new(current: i32, max: i32) -> Self {
        Self { current, max }
    }

    pub fn full(max: i32) -> Self {
        Self::new(max, max)
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0
    }

    pub fn is_full(&self) -> bool {
        self.current == self.max
    }

    pub fn damage(&mut self, amount: i32) {
        self.current = (self.current - amount).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.current = (self.current + amount).min(self.max);
    }
}

impl Default for MockHealth {
    fn default() -> Self {
        Self::full(100)
    }
}

/// Mock name component for testing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockName {
    pub value: String,
}

impl MockName {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
        }
    }
}

impl Default for MockName {
    fn default() -> Self {
        Self::new("Entity")
    }
}

/// Marker component for testing (zero-sized type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MockPlayer;

/// Marker component for testing (zero-sized type)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MockEnemy;

// =============================================================================
// Test Helpers
// =============================================================================

/// Test entity builder for creating entities with components
pub struct TestEntityBuilder {
    position: Option<MockPosition>,
    velocity: Option<MockVelocity>,
    health: Option<MockHealth>,
    name: Option<MockName>,
    is_player: bool,
    is_enemy: bool,
}

impl TestEntityBuilder {
    pub fn new() -> Self {
        Self {
            position: None,
            velocity: None,
            health: None,
            name: None,
            is_player: false,
            is_enemy: false,
        }
    }

    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = Some(MockPosition::new(x, y, z));
        self
    }

    pub fn with_velocity(mut self, x: f32, y: f32, z: f32) -> Self {
        self.velocity = Some(MockVelocity::new(x, y, z));
        self
    }

    pub fn with_health(mut self, current: i32, max: i32) -> Self {
        self.health = Some(MockHealth::new(current, max));
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(MockName::new(name));
        self
    }

    pub fn as_player(mut self) -> Self {
        self.is_player = true;
        self
    }

    pub fn as_enemy(mut self) -> Self {
        self.is_enemy = true;
        self
    }

    pub fn position(&self) -> Option<&MockPosition> {
        self.position.as_ref()
    }

    pub fn velocity(&self) -> Option<&MockVelocity> {
        self.velocity.as_ref()
    }

    pub fn health(&self) -> Option<&MockHealth> {
        self.health.as_ref()
    }

    pub fn name(&self) -> Option<&MockName> {
        self.name.as_ref()
    }

    pub fn is_player(&self) -> bool {
        self.is_player
    }

    pub fn is_enemy(&self) -> bool {
        self.is_enemy
    }
}

impl Default for TestEntityBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates unique test IDs for entities
pub struct TestIdGenerator {
    next_id: AtomicU32,
}

impl TestIdGenerator {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU32::new(1),
        }
    }

    pub fn next(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    pub fn reset(&self) {
        self.next_id.store(1, Ordering::SeqCst);
    }
}

impl Default for TestIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Custom Assertions
// =============================================================================

/// Asserts that two floating point values are approximately equal
#[macro_export]
macro_rules! assert_approx_eq {
    ($a:expr, $b:expr) => {
        assert_approx_eq!($a, $b, 0.0001)
    };
    ($a:expr, $b:expr, $epsilon:expr) => {
        let diff = ($a - $b).abs();
        assert!(
            diff < $epsilon,
            "assertion failed: `(left ≈ right)`\n  left: `{:?}`,\n right: `{:?}`,\n  diff: `{:?}`,\n epsilon: `{:?}`",
            $a,
            $b,
            diff,
            $epsilon
        );
    };
}

/// Asserts that two positions are approximately equal
#[macro_export]
macro_rules! assert_position_eq {
    ($a:expr, $b:expr) => {
        assert_position_eq!($a, $b, 0.0001)
    };
    ($a:expr, $b:expr, $epsilon:expr) => {
        assert_approx_eq!($a.x, $b.x, $epsilon);
        assert_approx_eq!($a.y, $b.y, $epsilon);
        assert_approx_eq!($a.z, $b.z, $epsilon);
    };
}

/// Asserts that two velocities are approximately equal
#[macro_export]
macro_rules! assert_velocity_eq {
    ($a:expr, $b:expr) => {
        assert_velocity_eq!($a, $b, 0.0001)
    };
    ($a:expr, $b:expr, $epsilon:expr) => {
        assert_approx_eq!($a.x, $b.x, $epsilon);
        assert_approx_eq!($a.y, $b.y, $epsilon);
        assert_approx_eq!($a.z, $b.z, $epsilon);
    };
}

// =============================================================================
// Test Data Generators
// =============================================================================

/// Generates random test positions
pub fn random_position(range: f32) -> MockPosition {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    std::time::SystemTime::now().hash(&mut hasher);
    let hash = hasher.finish();

    let x = ((hash % 1000) as f32 / 1000.0 - 0.5) * range * 2.0;
    let y = (((hash >> 16) % 1000) as f32 / 1000.0 - 0.5) * range * 2.0;
    let z = (((hash >> 32) % 1000) as f32 / 1000.0 - 0.5) * range * 2.0;

    MockPosition::new(x, y, z)
}

/// Generates random test velocities
pub fn random_velocity(max_speed: f32) -> MockVelocity {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};

    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    std::time::SystemTime::now().hash(&mut hasher);
    let hash = hasher.finish();

    let x = ((hash % 1000) as f32 / 1000.0 - 0.5) * max_speed * 2.0;
    let y = (((hash >> 16) % 1000) as f32 / 1000.0 - 0.5) * max_speed * 2.0;
    let z = (((hash >> 32) % 1000) as f32 / 1000.0 - 0.5) * max_speed * 2.0;

    MockVelocity::new(x, y, z)
}

/// Creates a batch of test entities with sequential IDs
pub fn create_test_entities(count: usize) -> Vec<TestEntityBuilder> {
    (0..count)
        .map(|i| {
            TestEntityBuilder::new()
                .with_position(i as f32, i as f32, i as f32)
                .with_name(format!("Entity_{}", i))
        })
        .collect()
}

// =============================================================================
// Test World Helpers
// =============================================================================

/// Test world configuration for consistent test environments
#[derive(Debug, Clone)]
pub struct TestWorldConfig {
    pub entity_capacity: usize,
    pub enable_parallel: bool,
}

impl TestWorldConfig {
    pub fn new() -> Self {
        Self {
            entity_capacity: 1000,
            enable_parallel: true,
        }
    }

    pub fn small() -> Self {
        Self {
            entity_capacity: 100,
            enable_parallel: false,
        }
    }

    pub fn large() -> Self {
        Self {
            entity_capacity: 100_000,
            enable_parallel: true,
        }
    }
}

impl Default for TestWorldConfig {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Performance Testing Helpers
// =============================================================================

/// Simple timer for measuring test performance
pub struct TestTimer {
    start: std::time::Instant,
    name: String,
}

impl TestTimer {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: std::time::Instant::now(),
            name: name.into(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed().as_secs_f64() * 1000.0
    }

    pub fn elapsed_us(&self) -> f64 {
        self.elapsed().as_secs_f64() * 1_000_000.0
    }
}

impl Drop for TestTimer {
    fn drop(&mut self) {
        println!("[{}] took {:.2}ms", self.name, self.elapsed_ms());
    }
}

// =============================================================================
// Test Utilities
// =============================================================================

/// Runs a test function multiple times to check for race conditions
pub fn stress_test<F>(iterations: usize, mut test_fn: F)
where
    F: FnMut(usize),
{
    for i in 0..iterations {
        test_fn(i);
    }
}

/// Runs a test function in parallel threads
pub fn parallel_stress_test<F>(threads: usize, iterations_per_thread: usize, test_fn: F)
where
    F: Fn(usize) + Send + Sync + 'static,
{
    use std::sync::Arc;
    let test_fn = Arc::new(test_fn);
    let mut handles = vec![];

    for thread_id in 0..threads {
        let test_fn = Arc::clone(&test_fn);
        let handle = std::thread::spawn(move || {
            for i in 0..iterations_per_thread {
                test_fn(thread_id * iterations_per_thread + i);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_position() {
        let pos1 = MockPosition::new(1.0, 2.0, 3.0);
        let pos2 = MockPosition::new(4.0, 5.0, 6.0);

        let distance = pos1.distance_to(&pos2);
        assert_approx_eq!(distance, 5.196152);
    }

    #[test]
    fn test_mock_velocity() {
        let vel = MockVelocity::new(3.0, 4.0, 0.0);
        assert_approx_eq!(vel.magnitude(), 5.0);
    }

    #[test]
    fn test_mock_health() {
        let mut health = MockHealth::full(100);
        assert!(health.is_full());
        assert!(health.is_alive());

        health.damage(30);
        assert_eq!(health.current, 70);
        assert!(!health.is_full());
        assert!(health.is_alive());

        health.heal(20);
        assert_eq!(health.current, 90);

        health.damage(200);
        assert_eq!(health.current, 0);
        assert!(!health.is_alive());
    }

    #[test]
    fn test_entity_builder() {
        let entity = TestEntityBuilder::new()
            .with_position(1.0, 2.0, 3.0)
            .with_velocity(0.5, 0.0, -0.5)
            .with_health(50, 100)
            .with_name("TestEntity")
            .as_player();

        assert!(entity.position().is_some());
        assert!(entity.velocity().is_some());
        assert!(entity.health().is_some());
        assert!(entity.name().is_some());
        assert!(entity.is_player());
        assert!(!entity.is_enemy());
    }

    #[test]
    fn test_id_generator() {
        let gen = TestIdGenerator::new();
        let id1 = gen.next();
        let id2 = gen.next();
        let id3 = gen.next();

        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);

        gen.reset();
        assert_eq!(gen.next(), 1);
    }

    #[test]
    fn test_create_test_entities() {
        let entities = create_test_entities(5);
        assert_eq!(entities.len(), 5);

        for (i, entity) in entities.iter().enumerate() {
            let pos = entity.position().unwrap();
            assert_approx_eq!(pos.x, i as f32);
            assert_approx_eq!(pos.y, i as f32);
            assert_approx_eq!(pos.z, i as f32);
        }
    }

    #[test]
    fn test_stress_test_runs_iterations() {
        let mut counter = 0;
        stress_test(100, |_| {
            counter += 1;
        });
        assert_eq!(counter, 100);
    }
}
