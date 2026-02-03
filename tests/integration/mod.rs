//! Integration test setup and utilities
//!
//! This module provides infrastructure for integration tests that test
//! multiple components working together. Integration tests are located
//! in the `tests/` directory and have access to the public API only.

use std::path::PathBuf;
use std::sync::Once;

// Error recovery and resilience tests
pub mod error_recovery_test;

static INIT: Once = Once::new();

/// Initialize test environment (logging, tracing, etc.)
pub fn init_test_environment() {
    INIT.call_once(|| {
        // Initialize tracing for tests if RUST_LOG is set
        if std::env::var("RUST_LOG").is_ok() {
            tracing_subscriber::fmt()
                .with_test_writer()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .init();
        }
    });
}

/// Test configuration for integration tests
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    pub enable_logging: bool,
    pub temp_dir: Option<PathBuf>,
    pub timeout_ms: u64,
}

impl IntegrationTestConfig {
    pub fn new() -> Self {
        Self {
            enable_logging: false,
            temp_dir: None,
            timeout_ms: 5000,
        }
    }

    pub fn with_logging(mut self) -> Self {
        self.enable_logging = true;
        self
    }

    pub fn with_temp_dir(mut self, path: PathBuf) -> Self {
        self.temp_dir = Some(path);
        self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a temporary directory for test files
pub fn create_temp_test_dir(test_name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("silmaril-tests");
    path.push(test_name);

    if path.exists() {
        std::fs::remove_dir_all(&path).ok();
    }

    std::fs::create_dir_all(&path).expect("Failed to create temp test directory");
    path
}

/// Cleanup temporary test directory
pub fn cleanup_temp_test_dir(path: &PathBuf) {
    if path.exists() {
        std::fs::remove_dir_all(path).ok();
    }
}

/// RAII guard for temporary test directories
pub struct TempTestDir {
    path: PathBuf,
}

impl TempTestDir {
    pub fn new(test_name: &str) -> Self {
        Self {
            path: create_temp_test_dir(test_name),
        }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Drop for TempTestDir {
    fn drop(&mut self) {
        cleanup_temp_test_dir(&self.path);
    }
}

// =============================================================================
// ECS Integration Test Helpers
// =============================================================================

/// Helper for testing ECS world operations across multiple frames
pub struct MultiFrameTest {
    frame_count: usize,
    current_frame: usize,
}

impl MultiFrameTest {
    pub fn new(frame_count: usize) -> Self {
        Self {
            frame_count,
            current_frame: 0,
        }
    }

    pub fn run<F>(&mut self, mut frame_fn: F)
    where
        F: FnMut(usize),
    {
        for frame in 0..self.frame_count {
            self.current_frame = frame;
            frame_fn(frame);
        }
    }

    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    pub fn is_first_frame(&self) -> bool {
        self.current_frame == 0
    }

    pub fn is_last_frame(&self) -> bool {
        self.current_frame == self.frame_count - 1
    }
}

// =============================================================================
// Performance Integration Test Helpers
// =============================================================================

/// Measures performance of an operation across multiple iterations
pub struct PerformanceMeasurement {
    samples: Vec<std::time::Duration>,
    name: String,
}

impl PerformanceMeasurement {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            samples: Vec::new(),
            name: name.into(),
        }
    }

    pub fn measure<F, R>(&mut self, mut operation: F) -> R
    where
        F: FnMut() -> R,
    {
        let start = std::time::Instant::now();
        let result = operation();
        let elapsed = start.elapsed();
        self.samples.push(elapsed);
        result
    }

    pub fn average(&self) -> std::time::Duration {
        if self.samples.is_empty() {
            return std::time::Duration::ZERO;
        }
        let sum: std::time::Duration = self.samples.iter().sum();
        sum / self.samples.len() as u32
    }

    pub fn min(&self) -> std::time::Duration {
        self.samples
            .iter()
            .min()
            .copied()
            .unwrap_or(std::time::Duration::ZERO)
    }

    pub fn max(&self) -> std::time::Duration {
        self.samples
            .iter()
            .max()
            .copied()
            .unwrap_or(std::time::Duration::ZERO)
    }

    pub fn print_stats(&self) {
        println!("\n=== Performance: {} ===", self.name);
        println!("Samples: {}", self.samples.len());
        println!("Average: {:?}", self.average());
        println!("Min:     {:?}", self.min());
        println!("Max:     {:?}", self.max());
    }
}

// =============================================================================
// Networking Integration Test Helpers
// =============================================================================

/// Mock network client for testing
#[cfg(feature = "networking")]
pub struct MockNetworkClient {
    pub connected: bool,
    pub received_messages: Vec<Vec<u8>>,
}

#[cfg(feature = "networking")]
impl MockNetworkClient {
    pub fn new() -> Self {
        Self {
            connected: false,
            received_messages: Vec::new(),
        }
    }

    pub fn connect(&mut self) {
        self.connected = true;
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    pub fn send_message(&mut self, _data: Vec<u8>) {
        // Simulate sending
    }

    pub fn receive_message(&mut self, data: Vec<u8>) {
        self.received_messages.push(data);
    }

    pub fn clear_messages(&mut self) {
        self.received_messages.clear();
    }
}

#[cfg(feature = "networking")]
impl Default for MockNetworkClient {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Renderer Integration Test Helpers
// =============================================================================

/// Configuration for headless rendering tests
#[cfg(feature = "renderer")]
pub struct HeadlessRenderConfig {
    pub width: u32,
    pub height: u32,
    pub enable_validation: bool,
}

#[cfg(feature = "renderer")]
impl HeadlessRenderConfig {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            enable_validation: true,
        }
    }

    pub fn small() -> Self {
        Self::new(256, 256)
    }

    pub fn medium() -> Self {
        Self::new(512, 512)
    }

    pub fn large() -> Self {
        Self::new(1024, 1024)
    }
}

#[cfg(feature = "renderer")]
impl Default for HeadlessRenderConfig {
    fn default() -> Self {
        Self::medium()
    }
}

// =============================================================================
// Physics Integration Test Helpers
// =============================================================================

/// Physics simulation test helper
#[cfg(feature = "physics")]
pub struct PhysicsTestSimulation {
    pub timestep: f32,
    pub current_time: f32,
    pub max_time: f32,
}

#[cfg(feature = "physics")]
impl PhysicsTestSimulation {
    pub fn new(timestep: f32, max_time: f32) -> Self {
        Self {
            timestep,
            current_time: 0.0,
            max_time,
        }
    }

    pub fn step(&mut self) -> bool {
        self.current_time += self.timestep;
        self.current_time < self.max_time
    }

    pub fn run<F>(&mut self, mut step_fn: F)
    where
        F: FnMut(f32),
    {
        while self.step() {
            step_fn(self.timestep);
        }
    }

    pub fn steps_remaining(&self) -> usize {
        ((self.max_time - self.current_time) / self.timestep) as usize
    }
}

// =============================================================================
// Serialization Integration Test Helpers
// =============================================================================

/// Helper for testing serialization round-trips
pub struct SerializationTester;

impl SerializationTester {
    /// Test binary serialization round-trip
    #[cfg(feature = "serialization")]
    pub fn test_binary_roundtrip<T>(value: &T) -> bool
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq,
    {
        let bytes = bincode::serialize(value).unwrap();
        let decoded: T = bincode::deserialize(&bytes).unwrap();
        value == &decoded
    }

    /// Test JSON serialization round-trip (useful for debugging)
    #[cfg(feature = "serialization")]
    pub fn test_json_roundtrip<T>(value: &T) -> bool
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq,
    {
        let json = serde_json::to_string(value).unwrap();
        let decoded: T = serde_json::from_str(&json).unwrap();
        value == &decoded
    }
}

// =============================================================================
// Multi-threaded Integration Test Helpers
// =============================================================================

/// Helper for testing thread-safe operations
pub struct ThreadSafetyTest {
    thread_count: usize,
}

impl ThreadSafetyTest {
    pub fn new(thread_count: usize) -> Self {
        Self { thread_count }
    }

    pub fn run<F>(&self, test_fn: F)
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        use std::sync::Arc;
        let test_fn = Arc::new(test_fn);
        let mut handles = vec![];

        for thread_id in 0..self.thread_count {
            let test_fn = Arc::clone(&test_fn);
            let handle = std::thread::spawn(move || {
                test_fn(thread_id);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temp_dir_creation() {
        let temp_dir = TempTestDir::new("test_temp_dir");
        assert!(temp_dir.path().exists());
    }

    #[test]
    fn test_multi_frame_test() {
        let mut test = MultiFrameTest::new(10);
        let mut frame_counter = 0;

        test.run(|frame| {
            assert_eq!(frame, frame_counter);
            frame_counter += 1;
        });

        assert_eq!(frame_counter, 10);
    }

    #[test]
    fn test_performance_measurement() {
        let mut perf = PerformanceMeasurement::new("test_operation");

        for _ in 0..10 {
            perf.measure(|| {
                std::thread::sleep(std::time::Duration::from_micros(100));
            });
        }

        assert_eq!(perf.samples.len(), 10);
        assert!(perf.average() > std::time::Duration::ZERO);
    }

    #[test]
    fn test_thread_safety_test() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let test = ThreadSafetyTest::new(4);
        test.run(move |_| {
            for _ in 0..100 {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        assert_eq!(counter.load(Ordering::SeqCst), 400);
    }

    #[cfg(feature = "networking")]
    #[test]
    fn test_mock_network_client() {
        let mut client = MockNetworkClient::new();
        assert!(!client.connected);

        client.connect();
        assert!(client.connected);

        client.receive_message(vec![1, 2, 3]);
        assert_eq!(client.received_messages.len(), 1);

        client.disconnect();
        assert!(!client.connected);
    }
}
