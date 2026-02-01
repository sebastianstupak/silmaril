---
name: build
description: Build client and/or server binaries for agent-game-engine
trigger: /build
---

# Build Client/Server Binaries

Builds the agent-game-engine client and/or server binaries with proper feature flags.

## Instructions

1. **Determine Build Target**
   Ask user or infer from context:
   - Build client only (default for dev)
   - Build server only (for deployment)
   - Build both (for full testing)
   - Build with specific features

2. **Build Client**
   ```bash
   # Development build (fast)
   cargo build --bin client --features client

   # Release build (optimized)
   cargo build --bin client --features client --release

   # With profiling
   cargo build --bin client --features "client,profiling" --release
   ```

3. **Build Server**
   ```bash
   # Development build
   cargo build --bin server --features server

   # Release build (optimized)
   cargo build --bin server --features server --release

   # With profiling
   cargo build --bin server --features "server,profiling" --release
   ```

4. **Build Both**
   Run both builds in sequence and report on each

5. **Show Build Information**
   After successful build, display:
   - Binary location (target/debug/ or target/release/)
   - Binary size
   - Build time
   - Feature flags used
   - Target platform

6. **Handle Build Failures**
   If build fails:
   - Show the full error output
   - Identify which dependency or crate failed
   - Check for common issues:
     - Missing Vulkan SDK (check VK_SDK_PATH)
     - Missing system dependencies
     - Feature flag conflicts
     - Platform compatibility issues
   - Suggest fixes based on error patterns
   - Reference docs/platform-abstraction.md for platform-specific issues

7. **Platform-Specific Builds**
   If cross-compiling or building for specific platform:
   ```bash
   # Windows
   cargo build --target x86_64-pc-windows-msvc --features client --release

   # Linux
   cargo build --target x86_64-unknown-linux-gnu --features client --release

   # macOS (Intel)
   cargo build --target x86_64-apple-darwin --features client --release

   # macOS (ARM)
   cargo build --target aarch64-apple-darwin --features client --release
   ```

8. **Verify Dependencies**
   Before building, check for:
   - Vulkan SDK installed (for client builds)
   - Required system libraries
   - Rust toolchain version (check for minimum required version)

## Output Format

Provide a clear summary:

```
Build Summary
=============

Client Build:  SUCCESS
  Location:    target/release/client
  Size:        45.2 MB
  Features:    client, vulkan-renderer
  Time:        2m 34s

Server Build:  SUCCESS
  Location:    target/release/server
  Size:        12.8 MB
  Features:    server, networking
  Time:        1m 12s

Total Build Time: 3m 46s

Binaries ready for testing!
```

Or if failures:

```
Build Summary
=============

Client Build:  FAILED
  Error:       Vulkan SDK not found
  Solution:    Install Vulkan SDK and set VK_SDK_PATH
               See: docs/platform-abstraction.md#vulkan-setup

Server Build:  SUCCESS
  Location:    target/release/server
  Size:        12.8 MB
  Features:    server, networking
  Time:        1m 12s
```

## Build Optimizations

### Development Builds
- Fast compilation
- Debug symbols included
- No optimizations
- Larger binary size

### Release Builds
- Full optimizations (opt-level = 3)
- LTO enabled
- Smaller binary size
- Slower compilation

### Profile Builds
- Release optimizations
- Debug symbols included
- Tracy profiling enabled
- Best for performance testing

## Notes

- Always specify feature flags (client/server) to avoid compilation issues
- Check Vulkan SDK before building client
- Release builds take significantly longer but produce much faster binaries
- Binary size and build time are good indicators of compilation health
- Reference ROADMAP.md to see which binaries are implemented for current phase
