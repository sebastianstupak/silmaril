//! Fault injection framework for testing error recovery and resilience.
//!
//! This module provides utilities for simulating various failure scenarios
//! to validate that the engine handles errors gracefully and recovers properly.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

/// Fault injection configuration for testing error scenarios.
#[derive(Debug, Clone)]
pub struct FaultConfig {
    /// Enable memory allocation failures
    pub fail_memory_allocation: bool,
    /// Enable file I/O failures
    pub fail_file_io: bool,
    /// Enable network failures
    pub fail_network: bool,
    /// Enable GPU operation failures
    pub fail_gpu_operations: bool,
    /// Probability of failure (0.0 = never, 1.0 = always)
    pub failure_rate: f32,
    /// Maximum number of failures before auto-disable
    pub max_failures: u32,
}

impl FaultConfig {
    /// Create a new fault configuration with all failures disabled.
    pub fn new() -> Self {
        Self {
            fail_memory_allocation: false,
            fail_file_io: false,
            fail_network: false,
            fail_gpu_operations: false,
            failure_rate: 0.0,
            max_failures: u32::MAX,
        }
    }

    /// Enable memory allocation failures.
    pub fn with_memory_failures(mut self, rate: f32) -> Self {
        self.fail_memory_allocation = true;
        self.failure_rate = rate;
        self
    }

    /// Enable file I/O failures.
    pub fn with_file_io_failures(mut self, rate: f32) -> Self {
        self.fail_file_io = true;
        self.failure_rate = rate;
        self
    }

    /// Enable network failures.
    pub fn with_network_failures(mut self, rate: f32) -> Self {
        self.fail_network = true;
        self.failure_rate = rate;
        self
    }

    /// Enable GPU operation failures.
    pub fn with_gpu_failures(mut self, rate: f32) -> Self {
        self.fail_gpu_operations = true;
        self.failure_rate = rate;
        self
    }

    /// Set maximum number of failures.
    pub fn with_max_failures(mut self, max: u32) -> Self {
        self.max_failures = max;
        self
    }
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Fault injector for simulating various failure scenarios.
pub struct FaultInjector {
    config: FaultConfig,
    failure_count: AtomicU32,
    enabled: AtomicBool,
    rng_seed: Mutex<u32>,
}

impl FaultInjector {
    /// Create a new fault injector with the given configuration.
    pub fn new(config: FaultConfig) -> Self {
        info!(
            memory_failures = config.fail_memory_allocation,
            file_io_failures = config.fail_file_io,
            network_failures = config.fail_network,
            gpu_failures = config.fail_gpu_operations,
            failure_rate = config.failure_rate,
            "Fault injector created"
        );

        Self {
            config,
            failure_count: AtomicU32::new(0),
            enabled: AtomicBool::new(true),
            rng_seed: Mutex::new(12345),
        }
    }

    /// Check if we should inject a failure based on the configuration.
    fn should_fail(&self) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        let count = self.failure_count.load(Ordering::Relaxed);
        if count >= self.config.max_failures {
            warn!(
                failure_count = count,
                max_failures = self.config.max_failures,
                "Fault injector disabled due to reaching max failures"
            );
            self.enabled.store(false, Ordering::Relaxed);
            return false;
        }

        // Simple pseudo-random number generator
        let mut seed = self.rng_seed.lock().unwrap();
        *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let random = (*seed as f32) / (u32::MAX as f32);

        random < self.config.failure_rate
    }

    /// Try to inject a memory allocation failure.
    pub fn maybe_fail_memory_allocation(&self) -> Result<(), &'static str> {
        if self.config.fail_memory_allocation && self.should_fail() {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
            error!("Injecting memory allocation failure");
            return Err("Simulated memory allocation failure");
        }
        Ok(())
    }

    /// Try to inject a file I/O failure.
    pub fn maybe_fail_file_io(&self, operation: &str) -> Result<(), std::io::Error> {
        if self.config.fail_file_io && self.should_fail() {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
            error!(operation = operation, "Injecting file I/O failure");
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Simulated {} failure", operation),
            ));
        }
        Ok(())
    }

    /// Try to inject a network failure.
    pub fn maybe_fail_network(&self, operation: &str) -> Result<(), String> {
        if self.config.fail_network && self.should_fail() {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
            error!(operation = operation, "Injecting network failure");
            return Err(format!("Simulated network {} failure", operation));
        }
        Ok(())
    }

    /// Try to inject a GPU operation failure.
    pub fn maybe_fail_gpu_operation(&self, operation: &str) -> Result<(), String> {
        if self.config.fail_gpu_operations && self.should_fail() {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
            error!(operation = operation, "Injecting GPU operation failure");
            return Err(format!("Simulated GPU {} failure", operation));
        }
        Ok(())
    }

    /// Get the total number of failures injected.
    pub fn failure_count(&self) -> u32 {
        self.failure_count.load(Ordering::Relaxed)
    }

    /// Reset the failure counter.
    pub fn reset(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.enabled.store(true, Ordering::Relaxed);
        info!("Fault injector reset");
    }

    /// Disable the fault injector.
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
        info!("Fault injector disabled");
    }

    /// Enable the fault injector.
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
        info!("Fault injector enabled");
    }
}

/// Mock renderer that can simulate GPU failures.
pub struct MockRenderer {
    pub device_lost: AtomicBool,
    pub oom_errors: AtomicU32,
    pub fault_injector: Option<Arc<FaultInjector>>,
}

impl MockRenderer {
    /// Create a new mock renderer.
    pub fn new() -> Self {
        Self {
            device_lost: AtomicBool::new(false),
            oom_errors: AtomicU32::new(0),
            fault_injector: None,
        }
    }

    /// Create a mock renderer with fault injection.
    pub fn with_fault_injector(fault_injector: Arc<FaultInjector>) -> Self {
        Self {
            device_lost: AtomicBool::new(false),
            oom_errors: AtomicU32::new(0),
            fault_injector: Some(fault_injector),
        }
    }

    /// Simulate device lost error.
    pub fn simulate_device_lost(&self) {
        self.device_lost.store(true, Ordering::Relaxed);
        warn!("Simulated device lost");
    }

    /// Check if device is lost.
    pub fn is_device_lost(&self) -> bool {
        self.device_lost.load(Ordering::Relaxed)
    }

    /// Reset device state.
    pub fn reset_device(&self) {
        self.device_lost.store(false, Ordering::Relaxed);
        info!("Mock device reset");
    }

    /// Simulate rendering a frame.
    pub fn render_frame(&self) -> Result<(), String> {
        if let Some(ref injector) = self.fault_injector {
            injector.maybe_fail_gpu_operation("render")?;
        }

        if self.is_device_lost() {
            return Err("Device lost".to_string());
        }

        Ok(())
    }

    /// Simulate allocating GPU memory.
    pub fn allocate_memory(&self, size: usize) -> Result<(), String> {
        if let Some(ref injector) = self.fault_injector {
            injector.maybe_fail_gpu_operation("memory allocation")?;
        }

        // Simulate OOM after 100MB
        if size > 100_000_000 {
            self.oom_errors.fetch_add(1, Ordering::Relaxed);
            return Err("Out of GPU memory".to_string());
        }

        Ok(())
    }

    /// Get OOM error count.
    pub fn oom_error_count(&self) -> u32 {
        self.oom_errors.load(Ordering::Relaxed)
    }
}

impl Default for MockRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock network connection that can simulate network failures.
pub struct MockNetworkConnection {
    pub connected: AtomicBool,
    pub packet_loss_count: AtomicU32,
    pub fault_injector: Option<Arc<FaultInjector>>,
}

impl MockNetworkConnection {
    /// Create a new mock network connection.
    pub fn new() -> Self {
        Self {
            connected: AtomicBool::new(false),
            packet_loss_count: AtomicU32::new(0),
            fault_injector: None,
        }
    }

    /// Create a mock connection with fault injection.
    pub fn with_fault_injector(fault_injector: Arc<FaultInjector>) -> Self {
        Self {
            connected: AtomicBool::new(false),
            packet_loss_count: AtomicU32::new(0),
            fault_injector: Some(fault_injector),
        }
    }

    /// Connect to server.
    pub fn connect(&self) -> Result<(), String> {
        if let Some(ref injector) = self.fault_injector {
            injector.maybe_fail_network("connect")?;
        }

        self.connected.store(true, Ordering::Relaxed);
        info!("Mock connection established");
        Ok(())
    }

    /// Disconnect from server.
    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::Relaxed);
        info!("Mock connection closed");
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Send a packet.
    pub fn send(&self, _data: &[u8]) -> Result<(), String> {
        if !self.is_connected() {
            return Err("Not connected".to_string());
        }

        if let Some(ref injector) = self.fault_injector {
            injector.maybe_fail_network("send")?;
        }

        Ok(())
    }

    /// Receive a packet (simulated).
    pub fn receive(&self) -> Result<Vec<u8>, String> {
        if !self.is_connected() {
            return Err("Not connected".to_string());
        }

        if let Some(ref injector) = self.fault_injector {
            if injector.maybe_fail_network("receive").is_err() {
                self.packet_loss_count.fetch_add(1, Ordering::Relaxed);
                return Err("Packet lost".to_string());
            }
        }

        Ok(vec![0u8; 64])
    }

    /// Get packet loss count.
    pub fn packet_loss_count(&self) -> u32 {
        self.packet_loss_count.load(Ordering::Relaxed)
    }
}

impl Default for MockNetworkConnection {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock file system that can simulate I/O failures.
pub struct MockFileSystem {
    pub disk_full: AtomicBool,
    pub read_errors: AtomicU32,
    pub write_errors: AtomicU32,
    pub fault_injector: Option<Arc<FaultInjector>>,
}

impl MockFileSystem {
    /// Create a new mock file system.
    pub fn new() -> Self {
        Self {
            disk_full: AtomicBool::new(false),
            read_errors: AtomicU32::new(0),
            write_errors: AtomicU32::new(0),
            fault_injector: None,
        }
    }

    /// Create a mock file system with fault injection.
    pub fn with_fault_injector(fault_injector: Arc<FaultInjector>) -> Self {
        Self {
            disk_full: AtomicBool::new(false),
            read_errors: AtomicU32::new(0),
            write_errors: AtomicU32::new(0),
            fault_injector: Some(fault_injector),
        }
    }

    /// Simulate disk full condition.
    pub fn set_disk_full(&self, full: bool) {
        self.disk_full.store(full, Ordering::Relaxed);
        if full {
            warn!("Simulated disk full");
        }
    }

    /// Read a file.
    pub fn read_file(&self, _path: &str) -> Result<Vec<u8>, std::io::Error> {
        if let Some(ref injector) = self.fault_injector {
            if let Err(e) = injector.maybe_fail_file_io("read") {
                self.read_errors.fetch_add(1, Ordering::Relaxed);
                return Err(e);
            }
        }

        Ok(vec![0u8; 1024])
    }

    /// Write a file.
    pub fn write_file(&self, _path: &str, _data: &[u8]) -> Result<(), std::io::Error> {
        if self.disk_full.load(Ordering::Relaxed) {
            self.write_errors.fetch_add(1, Ordering::Relaxed);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Disk full",
            ));
        }

        if let Some(ref injector) = self.fault_injector {
            if let Err(e) = injector.maybe_fail_file_io("write") {
                self.write_errors.fetch_add(1, Ordering::Relaxed);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Get read error count.
    pub fn read_error_count(&self) -> u32 {
        self.read_errors.load(Ordering::Relaxed)
    }

    /// Get write error count.
    pub fn write_error_count(&self) -> u32 {
        self.write_errors.load(Ordering::Relaxed)
    }
}

impl Default for MockFileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fault_config_builder() {
        let config = FaultConfig::new()
            .with_memory_failures(0.5)
            .with_max_failures(10);

        assert!(config.fail_memory_allocation);
        assert_eq!(config.failure_rate, 0.5);
        assert_eq!(config.max_failures, 10);
    }

    #[test]
    fn test_fault_injector_always_fails() {
        let config = FaultConfig::new().with_memory_failures(1.0).with_max_failures(1);
        let injector = FaultInjector::new(config);

        let result = injector.maybe_fail_memory_allocation();
        assert!(result.is_err());
        assert_eq!(injector.failure_count(), 1);
    }

    #[test]
    fn test_fault_injector_never_fails() {
        let config = FaultConfig::new().with_memory_failures(0.0);
        let injector = FaultInjector::new(config);

        for _ in 0..100 {
            let result = injector.maybe_fail_memory_allocation();
            assert!(result.is_ok());
        }

        assert_eq!(injector.failure_count(), 0);
    }

    #[test]
    fn test_fault_injector_max_failures() {
        let config = FaultConfig::new().with_memory_failures(1.0).with_max_failures(5);
        let injector = FaultInjector::new(config);

        // First 5 should fail
        for _ in 0..5 {
            let result = injector.maybe_fail_memory_allocation();
            assert!(result.is_err());
        }

        // After max failures, should not fail anymore
        for _ in 0..10 {
            let result = injector.maybe_fail_memory_allocation();
            assert!(result.is_ok());
        }

        assert_eq!(injector.failure_count(), 5);
    }

    #[test]
    fn test_mock_renderer() {
        let renderer = MockRenderer::new();

        assert!(renderer.render_frame().is_ok());

        renderer.simulate_device_lost();
        assert!(renderer.render_frame().is_err());

        renderer.reset_device();
        assert!(renderer.render_frame().is_ok());
    }

    #[test]
    fn test_mock_renderer_oom() {
        let renderer = MockRenderer::new();

        // Small allocation should succeed
        assert!(renderer.allocate_memory(1000).is_ok());

        // Large allocation should fail
        assert!(renderer.allocate_memory(200_000_000).is_err());
        assert_eq!(renderer.oom_error_count(), 1);
    }

    #[test]
    fn test_mock_network() {
        let network = MockNetworkConnection::new();

        assert!(!network.is_connected());
        assert!(network.send(&[]).is_err());

        assert!(network.connect().is_ok());
        assert!(network.is_connected());
        assert!(network.send(&[1, 2, 3]).is_ok());

        network.disconnect();
        assert!(!network.is_connected());
    }

    #[test]
    fn test_mock_file_system() {
        let fs = MockFileSystem::new();

        assert!(fs.read_file("test.txt").is_ok());
        assert!(fs.write_file("test.txt", &[1, 2, 3]).is_ok());

        fs.set_disk_full(true);
        assert!(fs.write_file("test.txt", &[1, 2, 3]).is_err());
        assert_eq!(fs.write_error_count(), 1);
    }

    #[test]
    fn test_fault_injector_reset() {
        let config = FaultConfig::new().with_memory_failures(1.0);
        let injector = FaultInjector::new(config);

        assert!(injector.maybe_fail_memory_allocation().is_err());
        assert_eq!(injector.failure_count(), 1);

        injector.reset();
        assert_eq!(injector.failure_count(), 0);
    }

    #[test]
    fn test_fault_injector_enable_disable() {
        let config = FaultConfig::new().with_memory_failures(1.0);
        let injector = FaultInjector::new(config);

        injector.disable();
        assert!(injector.maybe_fail_memory_allocation().is_ok());

        injector.enable();
        assert!(injector.maybe_fail_memory_allocation().is_err());
    }
}
