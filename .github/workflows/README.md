# GitHub Actions Workflows

This directory contains CI/CD workflows for the agent-game-engine project.

## Workflows Overview

### 1. **architecture.yml** - Architecture Validation
**Purpose:** Enforce architectural rules and patterns defined in CLAUDE.md

**Runs on:** Every push and PR to main/develop

**What it validates:**
- **Dependency Architecture** (cargo-deny)
  - Security vulnerabilities (deny)
  - License compliance (Apache-2.0 compatible)
  - Banned dependencies (anyhow, openssl)
  - Multiple version conflicts (warn)

- **Compile-time Checks** (build.rs)
  - Platform abstraction enforcement
  - Error handling patterns
  - Cross-platform compilation

- **Runtime Architecture Tests**
  - Platform abstraction correctness
  - Architecture constraint validation
  - Integration tests for architectural patterns

- **Architecture-focused Lints** (clippy)
  - No print/debug statements (`print_stdout`, `print_stderr`, `dbg_macro`)
  - No unimplemented TODOs in production code
  - Missing error/panic documentation

- **Code Formatting** (rustfmt)
  - Consistent code style across codebase

**Matrix:**
- Linux (x86_64)
- Windows (x86_64)
- macOS (x86_64 and ARM64)

**Fast-fail strategy:** Yes for build/dependency checks, No for tests

### 2. **ci.yml** - Continuous Integration
**Purpose:** General testing and quality checks

**Runs on:** Every push and PR to main/develop

**What it validates:**
- Format check (rustfmt)
- General clippy lints
- Cross-platform tests (all 145+ tests)
- Documentation tests
- Code coverage (via tarpaulin + codecov)
- Documentation build (no warnings)
- Security audit (RustSec advisory-db)

**Matrix:** Same as architecture.yml

**Relationship to architecture.yml:**
- Runs in parallel for speed
- Architecture checks are mandatory gates
- CI checks focus on correctness, not architecture

### 3. **bench.yml** - Performance Benchmarks
**Purpose:** Track performance regressions

**Runs on:** Push to main, manual trigger

**What it does:**
- Runs all `cargo bench` benchmarks
- Stores results for comparison
- Alerts on significant regressions

### 4. **docker.yml** - Container Builds
**Purpose:** Build and publish Docker images

**Runs on:** Push to main, version tags

**What it does:**
- Builds client and server Docker images
- Publishes to container registry
- Validates multi-platform images

### 5. **release.yml** - Release Automation
**Purpose:** Automated releases

**Runs on:** Version tags (v*.*.*)

**What it does:**
- Builds release binaries for all platforms
- Generates release notes
- Publishes to GitHub Releases
- Optionally publishes to crates.io

## Workflow Dependencies

```
architecture.yml (mandatory) ─┐
                              ├─> Merge allowed
ci.yml (mandatory) ───────────┘

bench.yml (optional, informational)
docker.yml (on main/tags only)
release.yml (on tags only)
```

## Architecture Validation Details

### Cargo Deny Checks

The `architecture.yml` workflow runs cargo-deny with our custom configuration (`deny.toml`):

1. **Advisories Check**
   - Scans for known security vulnerabilities
   - Uses RustSec advisory database
   - Fails on any vulnerability (severity: deny)

2. **Bans Check**
   - Enforces banned dependencies:
     - `anyhow` (use custom error types via `define_error!`)
     - `openssl` / `openssl-sys` (use rustls)
   - Allows specific duplicate crates (windows-sys, syn, bitflags)

3. **Licenses Check**
   - Ensures all dependencies use compatible licenses
   - Allowed: MIT, Apache-2.0, BSD-*, ISC, Zlib, etc.
   - Special exception for `ring` (OpenSSL license)

4. **Sources Check**
   - Validates all crates come from crates.io
   - No unknown registries or git sources

### Build.rs Checks

When implemented (Phase 1.4), build scripts will validate:

- No `#[cfg(target_os = "...")]` in business logic
- Platform abstractions properly used
- No direct platform API calls outside abstraction layer

### Architecture Tests

Runtime tests that validate:

- Platform abstraction traits work correctly
- Error types follow defined patterns
- No forbidden dependencies in compiled code
- Cross-platform compatibility

### Clippy Architecture Lints

Specific lints enforced for architecture:

```rust
// ❌ Forbidden
println!("Debug output");        // clippy::print_stdout
eprintln!("Error");              // clippy::print_stderr
dbg!(value);                      // clippy::dbg_macro
todo!("Implement later");         // clippy::todo (production)
unimplemented!();                 // clippy::unimplemented

// ⚠️  Warnings
fn may_error() -> Result<()> { } // clippy::missing_errors_doc
fn may_panic() { }                // clippy::missing_panics_doc
```

## Performance Optimizations

### Parallel Execution
- Cargo deny checks run in parallel (advisories, bans, licenses, sources)
- Build checks run per-platform in parallel
- Tests run per-platform in parallel

### Caching Strategy
- Cargo dependencies cached via `Swatinem/rust-cache@v2`
- Separate cache keys for:
  - Build checks per platform
  - Test runs per platform
  - Clippy runs
  - Deny checks per check type

### Fail-fast
- Dependency checks fail fast (invalid config stops all jobs)
- Build checks fail fast (compilation errors stop tests)
- Tests do NOT fail fast (run all platform tests for visibility)

## Required Status Checks

For PR merging, these checks must pass:

**From architecture.yml:**
- [ ] Dependency Checks (all 4 cargo-deny checks)
- [ ] Compile-time Architecture Checks (all 4 platforms)
- [ ] Runtime Architecture Tests (all 4 platforms)
- [ ] Clippy Architecture Lints
- [ ] Format Check

**From ci.yml:**
- [ ] Format Check (duplicate, but faster feedback)
- [ ] Clippy General Lints
- [ ] Tests (all 4 platforms)
- [ ] Documentation Build
- [ ] Security Audit

## Local Development

Before pushing, run these commands to catch issues early:

```bash
# Architecture validation (local)
cargo deny check                           # Dependency architecture
cargo build --all-targets --all-features   # Compile-time checks
cargo test architecture_                   # Runtime architecture tests
cargo clippy -- -D warnings \
    -D clippy::print_stdout \
    -D clippy::print_stderr \
    -D clippy::dbg_macro
cargo fmt --check                          # Format check

# Full CI simulation (all checks)
cargo test --workspace --all-features      # All tests
cargo test --doc                           # Doc tests
cargo doc --no-deps --workspace            # Documentation build
```

## Troubleshooting

### Architecture Validation Failures

**cargo-deny fails:**
```bash
# Locally reproduce
cargo install cargo-deny
cargo deny check

# Fix dependency issues
# Edit Cargo.toml to remove banned dependencies
# Update deny.toml if needed (rare)
```

**Build check fails on specific platform:**
```bash
# Install cross-compilation toolchain
rustup target add x86_64-pc-windows-msvc    # Windows
rustup target add x86_64-unknown-linux-gnu  # Linux
rustup target add x86_64-apple-darwin       # macOS x64
rustup target add aarch64-apple-darwin      # macOS ARM

# Try cross-compilation locally
cargo build --target <target-triple>
```

**Architecture tests fail:**
```bash
# Run with verbose output
cargo test architecture_ -- --nocapture

# Run specific test
cargo test --test architecture_platform -- --nocapture
```

**Clippy architecture lints fail:**
```bash
# See all violations
cargo clippy --workspace --all-features --all-targets -- \
    -D clippy::print_stdout \
    -D clippy::print_stderr \
    -D clippy::dbg_macro

# Fix: Replace prints with tracing
use tracing::info;
info!("Message");  // Instead of println!
```

## Adding New Architecture Checks

### 1. Adding a new cargo-deny rule

Edit `deny.toml`:

```toml
[[bans.deny]]
name = "new-banned-crate"
wrappers = ["engine-*"]  # Ban in engine crates only
```

### 2. Adding a new build.rs check

Create/edit `engine/*/build.rs`:

```rust
fn main() {
    // Emit warnings for violations
    println!("cargo:warning=Architecture violation detected");
}
```

### 3. Adding a new architecture test

Create test in `engine/*/tests/architecture_*.rs`:

```rust
#[test]
fn test_architecture_constraint() {
    // Validate runtime constraint
}
```

### 4. Adding a new clippy lint

Edit `architecture.yml`:

```yaml
- name: Run clippy with architecture-focused lints
  run: |
    cargo clippy --workspace --all-features --all-targets -- \
      -D warnings \
      -D clippy::new_lint_name  # Add here
```

## References

- [CLAUDE.md](../../CLAUDE.md) - Project architecture rules
- [docs/rules/coding-standards.md](../../docs/rules/coding-standards.md) - Coding standards
- [docs/platform-abstraction.md](../../docs/platform-abstraction.md) - Platform abstraction guide
- [docs/error-handling.md](../../docs/error-handling.md) - Error handling architecture
- [deny.toml](../../deny.toml) - Cargo-deny configuration
