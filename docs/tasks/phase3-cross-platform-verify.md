# Phase 3.6: Cross-Platform Verification

**Status:** ⚪ Not Started
**Estimated Time:** 2-3 days
**Priority:** Critical (deployment readiness)

---

## 🎯 **Objective**

Verify all engine components work correctly across Windows, Linux, and macOS. Ensure platform-specific code is properly isolated, performance targets are met on all platforms, and CI/CD pipeline validates cross-platform builds.

**Features:**
- Platform-specific testing suite
- Performance validation per platform
- CI/CD cross-platform builds
- Platform abstraction verification
- Deployment package generation
- Documentation for platform differences

---

## 📋 **Detailed Tasks**

### **1. Platform Test Suite** (Day 1)

**File:** `tests/platform/mod.rs`

```rust
//! Cross-platform test suite
//!
//! Tests that verify engine functionality across Windows, Linux, and macOS.

#[cfg(test)]
mod tests {
    use engine_core::*;
    use engine_ecs::*;
    use engine_platform::*;

    /// Test platform detection
    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();

        #[cfg(target_os = "windows")]
        assert_eq!(platform, Platform::Windows);

        #[cfg(target_os = "linux")]
        assert_eq!(platform, Platform::Linux);

        #[cfg(target_os = "macos")]
        assert_eq!(platform, Platform::MacOS);

        println!("Running on platform: {:?}", platform);
    }

    /// Test filesystem paths
    #[test]
    fn test_filesystem_paths() {
        use std::path::PathBuf;

        // Test data directory
        let data_dir = Platform::data_dir();
        assert!(data_dir.exists() || data_dir.parent().unwrap().exists());
        println!("Data directory: {:?}", data_dir);

        // Test config directory
        let config_dir = Platform::config_dir();
        assert!(config_dir.exists() || config_dir.parent().unwrap().exists());
        println!("Config directory: {:?}", config_dir);

        // Test cache directory
        let cache_dir = Platform::cache_dir();
        assert!(cache_dir.exists() || cache_dir.parent().unwrap().exists());
        println!("Cache directory: {:?}", cache_dir);
    }

    /// Test threading
    #[test]
    fn test_threading() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};

        let counter = Arc::new(AtomicU32::new(0));
        let mut handles = vec![];

        // Spawn 4 threads
        for _ in 0..4 {
            let counter = Arc::clone(&counter);
            let handle = std::thread::spawn(move || {
                for _ in 0..1000 {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 4000);
        println!("Threading test passed: 4 threads, 4000 increments");
    }

    /// Test SIMD (if available)
    #[test]
    fn test_simd() {
        use glam::Vec3;

        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);

        let dot = a.dot(b);
        assert_eq!(dot, 1.0 * 4.0 + 2.0 * 5.0 + 3.0 * 6.0);

        let cross = a.cross(b);
        println!("SIMD test passed: dot={}, cross={:?}", dot, cross);
    }

    /// Test ECS performance
    #[test]
    fn test_ecs_performance() {
        use std::time::Instant;

        let mut world = World::new();

        // Spawn 10,000 entities
        let start = Instant::now();
        for i in 0..10000 {
            let entity = world.spawn();
            world.add_component(entity, Transform::default());
            world.add_component(entity, Health { current: 100, max: 100 });
        }
        let spawn_time = start.elapsed();

        println!("Spawned 10,000 entities in {:?}", spawn_time);
        assert!(spawn_time.as_millis() < 100, "Entity spawn too slow!");

        // Query entities
        let start = Instant::now();
        let mut count = 0;
        for (entity, (transform, health)) in world.query::<(&Transform, &Health)>().iter() {
            count += 1;
        }
        let query_time = start.elapsed();

        println!("Queried {} entities in {:?}", count, query_time);
        assert_eq!(count, 10000);
        assert!(query_time.as_millis() < 10, "Entity query too slow!");
    }

    /// Test memory allocation
    #[test]
    fn test_memory_allocation() {
        use std::time::Instant;

        // Allocate large buffer
        let size = 100 * 1024 * 1024; // 100MB
        let start = Instant::now();
        let mut buffer = vec![0u8; size];
        let alloc_time = start.elapsed();

        println!("Allocated {}MB in {:?}", size / 1024 / 1024, alloc_time);

        // Write to buffer
        let start = Instant::now();
        for i in 0..buffer.len() {
            buffer[i] = (i % 256) as u8;
        }
        let write_time = start.elapsed();

        println!("Wrote to buffer in {:?}", write_time);

        // Verify
        assert_eq!(buffer[0], 0);
        assert_eq!(buffer[255], 255);
    }

    #[derive(Debug, Clone, Copy)]
    struct Transform {
        position: glam::Vec3,
        rotation: glam::Quat,
        scale: glam::Vec3,
    }

    impl Default for Transform {
        fn default() -> Self {
            Self {
                position: glam::Vec3::ZERO,
                rotation: glam::Quat::IDENTITY,
                scale: glam::Vec3::ONE,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct Health {
        current: u32,
        max: u32,
    }
}
```

---

### **2. Performance Validation** (Day 1-2)

**File:** `tests/platform/performance.rs`

```rust
//! Platform-specific performance validation

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PerformanceBenchmark {
    pub name: String,
    pub duration: Duration,
    pub passed: bool,
    pub target: Duration,
}

pub struct PlatformBenchmarkSuite {
    benchmarks: Vec<PerformanceBenchmark>,
}

impl PlatformBenchmarkSuite {
    pub fn new() -> Self {
        Self {
            benchmarks: Vec::new(),
        }
    }

    /// Run benchmark
    pub fn benchmark<F>(
        &mut self,
        name: &str,
        target_ms: u64,
        iterations: u32,
        f: F,
    ) where
        F: Fn(),
    {
        let target = Duration::from_millis(target_ms);

        let start = Instant::now();
        for _ in 0..iterations {
            f();
        }
        let total_duration = start.elapsed();
        let avg_duration = total_duration / iterations;

        let passed = avg_duration <= target;

        self.benchmarks.push(PerformanceBenchmark {
            name: name.to_string(),
            duration: avg_duration,
            passed,
            target,
        });

        let status = if passed { "PASS" } else { "FAIL" };
        println!(
            "[{}] {}: {:.3}ms (target: {}ms)",
            status,
            name,
            avg_duration.as_secs_f64() * 1000.0,
            target_ms
        );
    }

    /// Print summary
    pub fn print_summary(&self) {
        println!("\n=== Platform Performance Summary ===");
        println!("Platform: {:?}", std::env::consts::OS);
        println!("Architecture: {}", std::env::consts::ARCH);
        println!();

        let total = self.benchmarks.len();
        let passed = self.benchmarks.iter().filter(|b| b.passed).count();
        let failed = total - passed;

        println!("Total benchmarks: {}", total);
        println!("Passed: {}", passed);
        println!("Failed: {}", failed);
        println!();

        if failed > 0 {
            println!("Failed benchmarks:");
            for benchmark in &self.benchmarks {
                if !benchmark.passed {
                    println!(
                        "  - {}: {:.3}ms (target: {:.3}ms)",
                        benchmark.name,
                        benchmark.duration.as_secs_f64() * 1000.0,
                        benchmark.target.as_secs_f64() * 1000.0
                    );
                }
            }
        }

        println!("====================================");
    }

    /// Check if all benchmarks passed
    pub fn all_passed(&self) -> bool {
        self.benchmarks.iter().all(|b| b.passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_ecs::World;

    #[test]
    fn test_platform_performance() {
        let mut suite = PlatformBenchmarkSuite::new();

        // ECS spawn benchmark
        suite.benchmark("ECS Entity Spawn (1000)", 10, 100, || {
            let mut world = World::new();
            for _ in 0..1000 {
                world.spawn();
            }
        });

        // Vector math benchmark
        suite.benchmark("Vector Math (10000 ops)", 1, 100, || {
            use glam::Vec3;
            let mut result = Vec3::ZERO;
            for i in 0..10000 {
                let v = Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0);
                result += v;
                result = result.normalize();
            }
        });

        // Memory allocation benchmark
        suite.benchmark("Memory Allocation (10MB)", 5, 100, || {
            let buffer = vec![0u8; 10 * 1024 * 1024];
            drop(buffer);
        });

        // HashMap operations
        suite.benchmark("HashMap Operations (1000)", 1, 100, || {
            use std::collections::HashMap;
            let mut map = HashMap::new();
            for i in 0..1000 {
                map.insert(i, i * 2);
            }
            for i in 0..1000 {
                let _ = map.get(&i);
            }
        });

        suite.print_summary();

        assert!(suite.all_passed(), "Some benchmarks failed!");
    }
}
```

---

### **3. CI/CD Configuration** (Day 2)

**File:** `.github/workflows/cross-platform.yml`

```yaml
name: Cross-Platform Build & Test

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

jobs:
  test-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal
          override: true

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Build
        run: cargo build --release --all-features

      - name: Run tests
        run: cargo test --release --all-features

      - name: Run platform tests
        run: cargo test --release --test platform

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: windows-build
          path: target/release/*.exe

  test-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libvulkan-dev vulkan-tools

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal
          override: true

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Build
        run: cargo build --release --all-features

      - name: Run tests
        run: cargo test --release --all-features

      - name: Run platform tests
        run: cargo test --release --test platform

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: linux-build
          path: target/release/agent-game-engine

  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install dependencies
        run: |
          brew install vulkan-loader vulkan-headers molten-vk

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          profile: minimal
          override: true

      - name: Cache cargo registry
        uses: actions/cache@v3
        with:
          path: ~/.cargo/registry
          key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo index
        uses: actions/cache@v3
        with:
          path: ~/.cargo/git
          key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

      - name: Cache cargo build
        uses: actions/cache@v3
        with:
          path: target
          key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

      - name: Build
        run: cargo build --release --all-features

      - name: Run tests
        run: cargo test --release --all-features

      - name: Run platform tests
        run: cargo test --release --test platform

      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: macos-build
          path: target/release/agent-game-engine

  create-release:
    needs: [test-windows, test-linux, test-macos]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v3

      - name: Create release archives
        run: |
          cd windows-build && zip -r ../agent-game-engine-windows.zip . && cd ..
          cd linux-build && tar czf ../agent-game-engine-linux.tar.gz . && cd ..
          cd macos-build && tar czf ../agent-game-engine-macos.tar.gz . && cd ..

      - name: Upload release artifacts
        uses: actions/upload-artifact@v3
        with:
          name: release-packages
          path: |
            agent-game-engine-windows.zip
            agent-game-engine-linux.tar.gz
            agent-game-engine-macos.tar.gz
```

---

### **4. Platform Abstraction Verification** (Day 2-3)

**File:** `tests/platform/abstraction.rs`

```rust
//! Verify platform abstraction layer

#[cfg(test)]
mod tests {
    use engine_platform::*;

    #[test]
    fn test_window_creation() {
        // Test window creation (headless for CI)
        let window_config = WindowConfig {
            width: 800,
            height: 600,
            title: "Platform Test".to_string(),
            resizable: true,
            headless: true, // Headless for CI
        };

        let result = Window::create(window_config);
        assert!(result.is_ok(), "Failed to create window");

        println!("Window creation test passed");
    }

    #[test]
    fn test_vulkan_available() {
        // Check if Vulkan is available
        let vulkan_available = VulkanContext::is_available();

        println!("Vulkan available: {}", vulkan_available);

        // On CI, Vulkan might not be available (headless)
        // Just log the result, don't assert
    }

    #[test]
    fn test_file_io() {
        use std::fs;
        use std::io::Write;

        // Test file I/O
        let test_file = Platform::temp_dir().join("test_file.txt");

        // Write
        let mut file = fs::File::create(&test_file).unwrap();
        file.write_all(b"Hello, platform!").unwrap();
        drop(file);

        // Read
        let contents = fs::read_to_string(&test_file).unwrap();
        assert_eq!(contents, "Hello, platform!");

        // Cleanup
        fs::remove_file(&test_file).unwrap();

        println!("File I/O test passed");
    }

    #[test]
    fn test_time() {
        use std::time::{Duration, Instant};

        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(50)); // Allow some slack

        println!("Time test passed: {:?}", elapsed);
    }

    #[test]
    fn test_env_vars() {
        // Test environment variables
        std::env::set_var("TEST_VAR", "test_value");
        let value = std::env::var("TEST_VAR").unwrap();
        assert_eq!(value, "test_value");

        std::env::remove_var("TEST_VAR");

        println!("Environment variable test passed");
    }
}
```

---

### **5. Documentation** (Day 3)

**File:** `docs/cross-platform.md`

```markdown
# Cross-Platform Support

The Agent Game Engine supports Windows, Linux, and macOS.

## Platform Requirements

### Windows
- Windows 10/11 (64-bit)
- Vulkan 1.2+ drivers
- Visual Studio 2019+ (for building)

### Linux
- Ubuntu 20.04+ (or equivalent)
- Vulkan 1.2+ drivers
- GCC 9+ or Clang 10+

### macOS
- macOS 11.0+ (Big Sur or later)
- MoltenVK (Vulkan on Metal)
- Xcode 12+

## Building

### Windows
```bash
cargo build --release
```

### Linux
```bash
# Install dependencies
sudo apt-get install libvulkan-dev vulkan-tools

# Build
cargo build --release
```

### macOS
```bash
# Install dependencies
brew install vulkan-loader vulkan-headers molten-vk

# Build
cargo build --release
```

## Platform Differences

### File Paths
- Windows: `C:\Users\...\AppData\Local\AgentGameEngine`
- Linux: `~/.local/share/agent-game-engine`
- macOS: `~/Library/Application Support/com.agent.game-engine`

### Performance
- Windows: Best performance on NVIDIA/AMD GPUs
- Linux: Good performance, may require driver updates
- macOS: Performance limited by MoltenVK (Metal translation layer)

### Known Issues
- macOS: MoltenVK doesn't support all Vulkan extensions
- Linux: Some older GPUs may not support Vulkan 1.2

## Testing

Run platform-specific tests:
```bash
cargo test --test platform
cargo test --test performance
```

## CI/CD

The project uses GitHub Actions for cross-platform CI/CD:
- Builds on Windows, Linux, and macOS
- Runs all tests on each platform
- Generates release artifacts

See `.github/workflows/cross-platform.yml` for details.
```

---

## ✅ **Acceptance Criteria**

- [ ] Platform test suite passes on Windows
- [ ] Platform test suite passes on Linux
- [ ] Platform test suite passes on macOS
- [ ] Performance targets met on all platforms
- [ ] CI/CD builds all platforms
- [ ] Platform abstraction verified
- [ ] No platform-specific code outside abstraction layer
- [ ] Documentation complete
- [ ] Release packages generated
- [ ] All tests pass on all platforms

---

## 🎯 **Performance Targets**

All performance targets from previous phases should be met on all platforms:

| Platform | ECS Performance | Physics Performance | Network Performance |
|----------|----------------|---------------------|---------------------|
| Windows | 100% | 100% | 100% |
| Linux | ≥ 95% | ≥ 95% | ≥ 95% |
| macOS | ≥ 90% | ≥ 90% | ≥ 90% |

Relative to Windows baseline.

---

**Dependencies:** All previous Phase 0-3 tasks
**Next:** Production deployment
