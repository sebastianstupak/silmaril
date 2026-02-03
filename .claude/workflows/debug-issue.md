# Workflow: Debug Issue

> Step-by-step debugging workflow for troubleshooting problems

---

## Prerequisites

- [ ] Issue reproduction steps
- [ ] Expected vs actual behavior
- [ ] Error messages or logs
- [ ] Platform/environment details

---

## Step 1: Reproduce the Issue

**Create minimal reproduction:**
```bash
# Start with failing test case
cargo test {failing_test_name}

# Or run the application
cargo run --bin {client|server}
```

**Document reproduction steps:**
```markdown
## Reproduction Steps

1. Start server: `cargo run --bin server`
2. Start client: `cargo run --bin client`
3. Perform action: {specific action}
4. Observe error: {error description}

## Expected Behavior
{What should happen}

## Actual Behavior
{What actually happens}

## Environment
- OS: Windows 11 / Ubuntu 22.04 / macOS 14
- Rust: 1.75.0
- GPU: NVIDIA RTX 3080 / AMD RX 6800
- Vulkan: 1.3.268
```

---

## Step 2: Enable Verbose Logging

**Set log level:**
```bash
# Trace everything
RUST_LOG=trace cargo run

# Trace specific module
RUST_LOG=silmaril_ecs=trace cargo run

# Multiple modules
RUST_LOG=silmaril_ecs=trace,silmaril_renderer=debug cargo run

# Trace with file locations
RUST_LOG=trace RUST_LOG_STYLE=always cargo run
```

**Review logs:**
```bash
# Redirect to file
RUST_LOG=trace cargo run 2>&1 | tee debug.log

# Filter for errors
RUST_LOG=trace cargo run 2>&1 | grep -i error

# Filter for specific component
RUST_LOG=trace cargo run 2>&1 | grep "entity_id=123"
```

---

## Step 3: Enable Validation Layers (for Vulkan issues)

**Windows:**
```bash
set VK_LAYER_PATH=C:\VulkanSDK\Bin
set VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
cargo run
```

**Linux:**
```bash
export VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
cargo run
```

**macOS:**
```bash
export VK_LAYER_PATH=/usr/local/share/vulkan/explicit_layer.d
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
cargo run
```

**Check validation output:**
```bash
# Validation errors will appear in logs
RUST_LOG=debug cargo run 2>&1 | grep -i validation
```

---

## Step 4: Add Debug Instrumentation

**Add tracing spans:**
```rust
use tracing::{debug, error, info, span, warn, Level};

fn problematic_function(entity: Entity) -> Result<(), Error> {
    let _span = span!(Level::DEBUG, "problematic_function", ?entity).entered();

    debug!("Starting problematic operation");

    // Your code here
    let result = do_something(entity);

    match result {
        Ok(value) => {
            debug!(?value, "Operation succeeded");
            Ok(())
        }
        Err(e) => {
            error!(?e, "Operation failed");
            Err(e)
        }
    }
}
```

**Add debug prints (temporary):**
```rust
// Only in debug builds
#[cfg(debug_assertions)]
{
    eprintln!("DEBUG: entity={:?}, position={:?}", entity, position);
}
```

---

## Step 5: Use Debugger

**Build with debug symbols:**
```bash
cargo build
```

**Launch in debugger:**

**GDB (Linux):**
```bash
rust-gdb target/debug/client

(gdb) break main
(gdb) run
(gdb) next
(gdb) print variable_name
(gdb) backtrace
```

**LLDB (macOS):**
```bash
rust-lldb target/debug/client

(lldb) breakpoint set --name main
(lldb) run
(lldb) next
(lldb) print variable_name
(lldb) bt
```

**Visual Studio (Windows):**
```bash
# Install rust-analyzer and CodeLLDB extension in VS Code
# Set breakpoint in editor
# Press F5 to debug
```

**Common debugger commands:**
```bash
# Set breakpoint
(gdb) break src/main.rs:42
(gdb) break my_function

# Conditional breakpoint
(gdb) break src/main.rs:42 if entity.id == 123

# Run until breakpoint
(gdb) run

# Step over (next line)
(gdb) next

# Step into (function call)
(gdb) step

# Continue execution
(gdb) continue

# Print variable
(gdb) print my_var

# Print all locals
(gdb) info locals

# Backtrace
(gdb) backtrace
(gdb) bt

# Move up/down stack frames
(gdb) up
(gdb) down
```

---

## Step 6: Check Memory Issues

**Run with sanitizers:**

**AddressSanitizer (memory safety):**
```bash
RUSTFLAGS="-Z sanitizer=address" cargo +nightly run
```

**LeakSanitizer (memory leaks):**
```bash
RUSTFLAGS="-Z sanitizer=leak" cargo +nightly run
```

**ThreadSanitizer (data races):**
```bash
RUSTFLAGS="-Z sanitizer=thread" cargo +nightly run
```

**Valgrind (Linux):**
```bash
cargo build
valgrind --leak-check=full ./target/debug/client
```

**heaptrack (Linux):**
```bash
cargo build
heaptrack ./target/debug/client
heaptrack_gui heaptrack.client.{pid}.gz
```

---

## Step 7: Profile Performance Issues

**Tracy profiler:**
```bash
# Build with profiling
cargo build --features profiling

# Run application
./target/debug/client

# Open Tracy profiler GUI
# Connect to running application
# Analyze frame times, zones, memory
```

**Flamegraph:**
```bash
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin client

# Open flamegraph.svg in browser
```

**Criterion benchmarks:**
```bash
# Run benchmarks
cargo bench

# View results
cat target/criterion/*/report/index.html

# Compare with baseline
cargo bench -- --save-baseline main
git checkout feature-branch
cargo bench -- --baseline main
```

**perf (Linux):**
```bash
# Record
perf record -g ./target/release/client

# Report
perf report
```

---

## Step 8: Analyze Crash Dumps

**Generate core dump (Linux):**
```bash
ulimit -c unlimited
cargo run
# After crash:
gdb target/debug/client core
(gdb) bt
```

**Windows crash dumps:**
```bash
# Use Windows Error Reporting
# Or Visual Studio Just-In-Time debugger
# Or Procdump:
procdump -ma -i client.exe
```

**Analyze backtrace:**
```bash
# Set panic handler to print backtrace
RUST_BACKTRACE=1 cargo run

# Full backtrace
RUST_BACKTRACE=full cargo run
```

---

## Step 9: Test Hypotheses

**Isolate the problem:**
```rust
#[test]
fn test_isolated_problem() {
    // Minimal test case for the issue
    let mut world = World::new();
    let entity = world.spawn();

    // Reproduce issue
    world.add(entity, ProblematicComponent::default());

    // This should fail if bug exists
    assert!(world.is_alive(entity));
}
```

**Binary search:**
```bash
# Use git bisect to find regression
git bisect start
git bisect bad  # Current commit is bad
git bisect good {known-good-commit}

# Git will checkout commits
# Test each one:
cargo test {failing_test}
git bisect good  # or git bisect bad

# When done:
git bisect reset
```

---

## Step 10: Check Platform-Specific Issues

**Test on different platforms:**
```bash
# Windows
cargo test --target x86_64-pc-windows-msvc

# Linux
cargo test --target x86_64-unknown-linux-gnu

# macOS
cargo test --target x86_64-apple-darwin
```

**Check for platform-specific code:**
```bash
# Find all platform-specific code
grep -r "#\[cfg(target_os" engine/

# Common issues:
# - Path separators (use std::path::Path)
# - Line endings (use \n, normalize on read)
# - Case-sensitive filesystems
# - Different default behaviors
```

---

## Step 11: Review Error Handling

**Check error propagation:**
```rust
// Bad - swallows error
if let Err(e) = do_something() {
    // Silent failure
}

// Good - logs and propagates
do_something().map_err(|e| {
    error!(?e, "Failed to do something");
    e
})?;
```

**Add context to errors:**
```rust
use anyhow::Context;

fn load_asset(path: &Path) -> Result<Asset> {
    std::fs::read(path)
        .context(format!("Failed to read asset file: {:?}", path))?;

    // ...
}
```

---

## Step 12: Check Resource Cleanup

**Verify resources are freed:**
```rust
#[test]
fn test_no_resource_leak() {
    let initial_memory = get_memory_usage();

    {
        let mut world = World::new();

        // Perform operations
        for _ in 0..1000 {
            let entity = world.spawn();
            world.add(entity, Transform::default());
        }

        // Resources should be cleaned up when world drops
    }

    let final_memory = get_memory_usage();
    assert!(final_memory <= initial_memory + threshold);
}
```

**Use RAII:**
```rust
// Bad - manual cleanup (easy to forget)
let handle = create_resource();
// ... use handle ...
destroy_resource(handle);  // Might not run if panic!

// Good - automatic cleanup
struct Resource(Handle);

impl Drop for Resource {
    fn drop(&mut self) {
        destroy_resource(self.0);
    }
}
```

---

## Step 13: Document Findings

**Create issue report:**
```markdown
# Bug Report: {Title}

## Summary
Brief description of the issue.

## Environment
- OS: {OS and version}
- Rust: {version}
- Commit: {git commit hash}
- GPU: {GPU model}

## Reproduction
1. Step 1
2. Step 2
3. Step 3

## Expected Behavior
{What should happen}

## Actual Behavior
{What actually happens}

## Logs
```
{Paste relevant logs}
```

## Analysis
- Root cause: {description}
- Affected components: {list}
- Workaround: {if any}

## Fix
{Description of fix or PR link}
```

---

## Step 14: Implement Fix

**Write failing test:**
```rust
#[test]
fn test_issue_123_entity_leak() {
    let mut world = World::new();
    let entity = world.spawn();

    world.despawn(entity);

    // This should pass after fix
    assert!(!world.is_alive(entity));
}
```

**Implement fix:**
```rust
// Fix the bug
pub fn despawn(&mut self, entity: Entity) -> bool {
    // ... fixed implementation
}
```

**Verify fix:**
```bash
cargo test test_issue_123_entity_leak
```

---

## Step 15: Verify Fix Doesn't Break Anything

**Run full test suite:**
```bash
cargo test --workspace --all-features
```

**Run affected benchmarks:**
```bash
cargo bench --bench {related_benchmark}
```

**Test on all platforms:**
```bash
./scripts/test-all-platforms.sh
```

---

## Common Issue Categories

### Memory Issues
- Memory leaks (Valgrind, heaptrack)
- Use-after-free (AddressSanitizer)
- Buffer overflows (AddressSanitizer)
- Double-free (AddressSanitizer)

**Tools:** Valgrind, AddressSanitizer, heaptrack

---

### Concurrency Issues
- Data races (ThreadSanitizer)
- Deadlocks (debug logs, tracing)
- Race conditions (careful code review)

**Tools:** ThreadSanitizer, tracing, debugging

---

### Performance Issues
- Slow functions (flamegraph, Tracy)
- Memory bloat (heaptrack)
- High CPU usage (perf, Tracy)

**Tools:** Tracy, flamegraph, perf, Criterion

---

### Rendering Issues
- Validation errors (Vulkan validation layers)
- Resource leaks (Tracy, memory profiler)
- Incorrect rendering (RenderDoc, frame capture)

**Tools:** Vulkan validation, RenderDoc, Tracy

---

### Network Issues
- Packet loss (Wireshark)
- Desync (state dumps, logging)
- High latency (network profiling)

**Tools:** Wireshark, logging, tracing

---

## Debugging Checklist

- [ ] Issue reproduced consistently
- [ ] Verbose logging enabled
- [ ] Validation layers enabled (if Vulkan)
- [ ] Debug instrumentation added
- [ ] Debugger used to inspect state
- [ ] Memory issues checked (sanitizers)
- [ ] Performance profiled (if slow)
- [ ] Crash dumps analyzed
- [ ] Hypothesis tested
- [ ] Platform-specific issues checked
- [ ] Error handling reviewed
- [ ] Resource cleanup verified
- [ ] Findings documented
- [ ] Fix implemented
- [ ] Full test suite passing

---

## Advanced Debugging Techniques

### Core Dumps
```bash
# Enable core dumps
ulimit -c unlimited

# After crash
gdb target/debug/client core

(gdb) bt
(gdb) info locals
(gdb) print variable_name
```

### RenderDoc (for Vulkan)
```bash
# Install RenderDoc
# Run application through RenderDoc
# Capture frame
# Analyze draw calls, resources, shaders
```

### Network Debugging
```bash
# Capture packets
wireshark

# Filter for game traffic
tcp.port == 8080 or udp.port == 8081

# Analyze packet contents
```

### Time-Travel Debugging
```bash
# Use rr (Linux only)
rr record ./target/debug/client
rr replay

# Debug with reverse execution
(gdb) reverse-next
(gdb) reverse-continue
```

---

## References

- [docs/development-workflow.md](../../docs/development-workflow.md) - Dev workflow
- [docs/testing-strategy.md](../../docs/testing-strategy.md) - Testing guide
- [docs/performance-targets.md](../../docs/performance-targets.md) - Performance targets

---

**Last Updated:** 2026-02-01
