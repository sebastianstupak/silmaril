# Contributing to Agent Game Engine

> **Welcome! We're excited to have you contribute to the Agent Game Engine project.**

This guide will help you understand our development workflow, coding standards, and contribution process.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Commit Conventions](#commit-conventions)
- [Pull Request Process](#pull-request-process)
- [Testing Requirements](#testing-requirements)
- [Documentation](#documentation)
- [Code Review](#code-review)
- [Community Guidelines](#community-guidelines)

---

## Getting Started

### Prerequisites

Before contributing, ensure you have:

- **Rust** (latest stable version via rustup)
- **Git** (version control)
- **Vulkan SDK** (for renderer development)
- **Platform-specific tools:**
  - **Windows:** Visual Studio Build Tools
  - **Linux:** GCC/Clang, development headers
  - **macOS:** Xcode Command Line Tools

### Setting Up Your Development Environment

1. **Fork and clone the repository:**
   ```bash
   git clone https://github.com/your-username/agent-game-engine.git
   cd agent-game-engine
   ```

2. **Install development dependencies:**
   ```bash
   # Install rustfmt and clippy
   rustup component add rustfmt clippy

   # Install cargo-watch for development
   cargo install cargo-watch

   # Install cargo-nextest for faster testing
   cargo install cargo-nextest
   ```

3. **Build the project:**
   ```bash
   cargo build
   ```

4. **Run tests:**
   ```bash
   cargo test
   ```

5. **Set up git hooks (optional but recommended):**
   ```bash
   # Format check on commit
   echo "cargo fmt --check" > .git/hooks/pre-commit
   chmod +x .git/hooks/pre-commit
   ```

---

## Development Workflow

### 1. Create a Feature Branch

Always work on a feature branch, never directly on `main`:

```bash
# Update your main branch
git checkout main
git pull origin main

# Create a new feature branch
git checkout -b feat/your-feature-name
```

**Branch naming convention:**
```
<type>/<short-description>

Examples:
feat/pbr-rendering
fix/vulkan-memory-leak
docs/networking-guide
refactor/ecs-queries
perf/batch-rendering
test/integration-tests
```

### 2. Make Your Changes

- Write clean, well-documented code
- Follow our [coding standards](./rules/coding-standards.md)
- Add tests for new functionality
- Update documentation as needed
- Keep commits atomic and well-described

### 3. Test Your Changes

Before committing, ensure:

```bash
# Format code
cargo fmt

# Check for warnings
cargo clippy -- -D warnings

# Run all tests
cargo test

# Run specific test suite
cargo test --package agent-game-engine-core

# Run integration tests
cargo test --test '*'

# Build documentation
cargo doc --no-deps
```

### 4. Commit Your Changes

Follow our [commit conventions](./contributing/commit-conventions.md):

```bash
# Stage specific files
git add src/renderer/pbr.rs tests/pbr_tests.rs

# Commit with conventional format
git commit -m "feat(renderer): add PBR material system"
```

See the detailed [commit conventions guide](./contributing/commit-conventions.md) for more information.

### 5. Push and Create Pull Request

```bash
# Push your branch
git push origin feat/your-feature-name

# Create a pull request on GitHub
# Use the PR template and fill in all sections
```

---

## Coding Standards

We maintain strict coding standards to ensure code quality, safety, and performance. Please read and follow the complete [coding standards document](./rules/coding-standards.md).

### Key Requirements

#### 1. No Printing - Use `tracing` Only

```rust
// ❌ FORBIDDEN
println!("Player joined");
eprintln!("Error: {}", e);
dbg!(value);

// ✅ CORRECT
use tracing::{info, error, debug};
info!(player_id = %id, "Player joined");
error!(error = ?e, "Operation failed");
```

#### 2. Custom Error Types Always

```rust
// ❌ FORBIDDEN
fn load() -> Result<Data, Box<dyn Error>> { }
fn init() -> anyhow::Result<()> { }

// ✅ CORRECT
define_error! {
    pub enum LoadError {
        NotFound { path: String } = ErrorCode::NotFound, ErrorSeverity::Error,
    }
}
fn load() -> Result<Data, LoadError> { }
```

#### 3. No Unsafe (Except FFI)

Unsafe code is only allowed for:
- Vulkan FFI calls
- Platform-specific APIs (Win32, etc.)
- Performance-critical code with thorough safety documentation

All unsafe code requires:
- Safety comments explaining invariants
- Comprehensive testing
- Code review approval

#### 4. Platform Abstraction

Never use platform-specific code directly in business logic:

```rust
// ❌ FORBIDDEN
#[cfg(windows)]
fn update() { /* Windows-specific code */ }

// ✅ CORRECT
fn update(platform: &dyn Platform) {
    platform.update();
}
```

#### 5. Format and Lint

Before every commit:

```bash
cargo fmt        # Format code
cargo clippy -- -D warnings  # No warnings allowed
```

**CI will block merge if formatting or linting fails.**

### Documentation Requirements

All public APIs must have rustdoc documentation:

```rust
/// Spawns a new entity in the world.
///
/// # Examples
///
/// ```
/// use agent_game_engine::*;
///
/// let mut world = World::new();
/// let entity = world.spawn();
/// world.add(entity, Transform::default());
/// ```
///
/// # Errors
///
/// Returns [`WorldError::EntityLimitReached`] if max entities exceeded.
pub fn spawn(&mut self) -> Result<Entity, WorldError> {
    // ...
}
```

---

## Commit Conventions

We follow [Conventional Commits](https://www.conventionalcommits.org/) for all commit messages.

### Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Quick Reference

**Types:**
- `feat` - New features
- `fix` - Bug fixes
- `docs` - Documentation only
- `style` - Code style (formatting, no logic change)
- `refactor` - Code refactoring (no feature/fix)
- `perf` - Performance improvements
- `test` - Adding or updating tests
- `chore` - Build process, tooling, dependencies
- `ci` - CI/CD changes
- `build` - Build system changes

**Examples:**
```
feat(renderer): add cascaded shadow maps
fix(ecs): prevent entity despawn during iteration
docs(api): add examples to World::spawn
refactor(networking): extract packet encoding logic
perf(query): optimize component iteration with SIMD
test(serialization): add property tests for encoding
chore(deps): update ash to 0.38.0
```

For detailed guidelines, examples, and best practices, see the complete [commit conventions guide](./contributing/commit-conventions.md).

---

## Pull Request Process

### Before Creating a PR

1. **Ensure your branch is up to date:**
   ```bash
   git checkout main
   git pull origin main
   git checkout feat/your-feature
   git rebase main
   ```

2. **Run the full test suite:**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test --all
   cargo doc --no-deps
   ```

3. **Review your changes:**
   ```bash
   git diff main..HEAD
   ```

### Creating the PR

1. **Push your branch:**
   ```bash
   git push origin feat/your-feature
   ```

2. **Create PR on GitHub** with:
   - **Title:** Brief description (like a commit message)
   - **Description:** Detailed explanation of changes
   - **Testing:** How you tested the changes
   - **Screenshots:** For visual changes
   - **Breaking Changes:** If any APIs changed
   - **Related Issues:** Reference relevant issues

### PR Template

```markdown
## Summary

Brief description of what this PR does.

## Motivation

Why is this change needed? What problem does it solve?

## Changes

- List of key changes
- Organized by component/area
- Include file paths if helpful

## Testing

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] All tests pass locally

## Screenshots (if applicable)

[Add screenshots for visual changes]

## Breaking Changes

[Describe any breaking API changes and migration path]

## Checklist

- [ ] Code follows coding standards
- [ ] Tests added for new functionality
- [ ] Documentation updated
- [ ] Commit messages follow conventions
- [ ] All CI checks pass
- [ ] No compiler warnings

## Related Issues

Closes #123
Relates to #456
```

### PR Review Process

1. **Automated checks** must pass:
   - Formatting (cargo fmt)
   - Linting (cargo clippy)
   - Tests (cargo test)
   - Documentation build (cargo doc)
   - Platform builds (Windows, Linux, macOS)

2. **Code review** by at least one maintainer:
   - Code quality and correctness
   - Test coverage
   - Documentation completeness
   - Performance considerations
   - Security implications

3. **Address feedback:**
   - Make requested changes
   - Push new commits (don't force push during review)
   - Respond to review comments
   - Request re-review when ready

4. **Merge:**
   - Maintainer will merge using squash or rebase
   - Branch will be automatically deleted
   - Referenced issues will be closed

### After Merge

- **Delete your local branch:**
  ```bash
  git checkout main
  git pull origin main
  git branch -d feat/your-feature
  ```

- **Update your fork:**
  ```bash
  git push origin main
  ```

---

## Testing Requirements

All code must be thoroughly tested. See our [testing strategy](./testing-strategy.md) for details.

### Test Types

1. **Unit Tests** - Test individual functions/modules
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_entity_spawn() {
           let mut world = World::new();
           let entity = world.spawn();
           assert!(entity.is_valid());
       }
   }
   ```

2. **Integration Tests** - Test component interactions
   ```rust
   // tests/integration_test.rs
   use agent_game_engine::*;

   #[test]
   fn test_full_game_loop() {
       // Test complete workflows
   }
   ```

3. **Property Tests** - Test invariants with random inputs
   ```rust
   use proptest::prelude::*;

   proptest! {
       #[test]
       fn test_serialization_roundtrip(data: ComponentData) {
           let bytes = serialize(&data)?;
           let decoded = deserialize(&bytes)?;
           assert_eq!(data, decoded);
       }
   }
   ```

### Test Coverage

- **New features** must have >90% code coverage
- **Bug fixes** must include regression tests
- **Refactorings** must maintain existing test coverage
- **Critical paths** (networking, serialization) require property tests

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_entity_spawn

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Benchmark Requirements

All performance-critical code must include benchmarks to prevent regressions.

#### When to Add Benchmarks

Add benchmarks for:
- **ECS operations**: Entity spawn, component queries, iteration
- **Physics systems**: Integration, collision detection, SIMD operations
- **Rendering**: Command buffer creation, sync operations, pipeline setup
- **Math operations**: Vector math, transforms, SIMD optimizations
- **Serialization**: Encoding/decoding world state, component data
- **Hot paths**: Any code that runs frequently (>1000 times/frame)

#### Benchmark Structure

```rust
// benches/my_feature_benches.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use agent_game_engine::*;

fn bench_my_feature(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_feature");

    // Test with different sizes
    for size in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &size| {
                // Setup
                let world = setup_world(size);

                b.iter(|| {
                    // Benchmark code
                    let result = my_feature(&world);
                    black_box(result); // Prevent optimization
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_my_feature);
criterion_main!(benches);
```

#### Performance Targets

All benchmarks must meet industry-standard targets:

| Operation | Target | Acceptable | Critical |
|-----------|--------|------------|----------|
| Entity spawn | < 50ns | < 100ns | < 200ns |
| Component query (1K entities) | < 1ms | < 2ms | < 5ms |
| Physics tick (10K entities) | < 8ms | < 12ms | < 16ms |
| Vulkan fence reset | < 10µs | < 20µs | < 50µs |

See [docs/performance-targets.md](performance-targets.md) for complete targets.

#### Running Benchmarks

```bash
# Run all benchmarks
just bench-all

# Run specific suite
just bench-ecs

# Compare with baseline
just bench-baseline

# Run with profiling
just bench-profile
```

#### Regression Detection

CI/CD automatically detects performance regressions on every PR:

- **Threshold**: 20% regression triggers CI failure
- **Comparison**: Against `main` branch baseline
- **Platforms**: Linux, Windows, macOS
- **Reporting**: Automated PR comments with results

If your PR triggers a regression:

1. **Review the benchmark results** in the PR comment
2. **Profile the code** with `just bench-profile`
3. **Optimize** the hot paths identified
4. **Re-run benchmarks** with `just bench-baseline`
5. **Document** any intentional performance trade-offs

#### Baseline Updates

Update baselines only when:

- ✅ Performance **improves** significantly (>10%)
- ✅ Architecture changes make old baseline irrelevant
- ✅ Quarterly baseline refresh for long-term tracking

Do NOT update baselines when:

- ❌ Performance **regresses** (fix the regression instead)
- ❌ Results vary due to system load (re-run in clean environment)
- ❌ Without maintainer approval and documentation

To update baseline:

```bash
# Create new baseline
./scripts/update_benchmark_baseline.sh main

# Review changes
git diff benchmarks/baselines/

# Commit with justification
git add benchmarks/baselines/
git commit -m "chore: Update benchmark baseline after ECS optimization

Performance improvements:
- Entity spawn: 47ns → 38ns (-19%)
- Component query: 0.8ms → 0.6ms (-25%)

Baseline updated to reflect optimizations in PR #123."
```

#### Benchmark Best Practices

1. **Use `black_box`**: Prevent compiler from optimizing away code
   ```rust
   b.iter(|| {
       let result = expensive_operation();
       black_box(result); // Essential!
   });
   ```

2. **Minimize setup time**: Only benchmark the operation, not setup
   ```rust
   // Setup outside the benchmark loop
   let world = setup_world(10000);

   b.iter(|| {
       // Only benchmark this part
       query_entities(&world);
   });
   ```

3. **Use realistic data**: Benchmark with production-like workloads
   ```rust
   // Good: Realistic entity count
   let world = create_world_with_entities(10000);

   // Bad: Unrealistically small
   let world = create_world_with_entities(10);
   ```

4. **Test multiple scales**: Verify performance across different sizes
   ```rust
   for size in [100, 1000, 10000, 100000] {
       group.bench_with_input(BenchmarkId::from_parameter(size), ...);
   }
   ```

5. **Document targets**: Explain why performance target was chosen
   ```rust
   /// Benchmark: Entity spawn
   ///
   /// Target: < 50ns (industry standard for ECS engines)
   /// - Unity DOTS: ~60ns
   /// - Bevy: ~45ns
   /// - Our target: < 50ns to be competitive
   ```

---

## Documentation

### Code Documentation

All public APIs require rustdoc comments:

```rust
/// Short one-line description.
///
/// Longer description with more details. Explain what this does,
/// when to use it, and any important considerations.
///
/// # Examples
///
/// ```
/// use agent_game_engine::*;
///
/// let mut world = World::new();
/// let entity = world.spawn();
/// ```
///
/// # Errors
///
/// Returns [`Error`] if operation fails.
///
/// # Panics
///
/// Panics if the entity is invalid.
///
/// # Safety
///
/// This function is unsafe because...
pub fn function() -> Result<(), Error> {
    // ...
}
```

### Markdown Documentation

Update relevant documentation in `docs/`:

- **Architecture changes** → `docs/architecture.md`
- **New features** → Feature-specific docs
- **API changes** → Update guides and examples
- **Performance** → `docs/performance-targets.md`

### Examples

Add examples for significant features:

```rust
// examples/pbr_rendering.rs
use agent_game_engine::*;

fn main() {
    // Demonstrate feature usage
}
```

---

## Code Review

### As a Reviewer

When reviewing PRs:

1. **Functionality:**
   - Does it solve the stated problem?
   - Are there edge cases not handled?
   - Is the approach sound?

2. **Code Quality:**
   - Follows coding standards?
   - Well-structured and readable?
   - Appropriate abstractions?

3. **Testing:**
   - Adequate test coverage?
   - Tests are meaningful?
   - Edge cases tested?

4. **Documentation:**
   - Public APIs documented?
   - Complex logic explained?
   - Examples provided?

5. **Performance:**
   - No obvious performance issues?
   - Allocations minimized?
   - Benchmarks if needed?

6. **Security:**
   - Input validation?
   - No unsafe patterns?
   - Server validation for networked code?

### Review Feedback

- **Be constructive** - Explain why, suggest alternatives
- **Be specific** - Reference line numbers, provide examples
- **Distinguish blocking vs. non-blocking** - Mark critical issues clearly
- **Acknowledge good work** - Positive feedback is valuable

### Review Labels

- `needs-changes` - Blocking issues to address
- `needs-discussion` - Design discussion needed
- `approved` - Ready to merge
- `waiting-for-author` - Author needs to respond/update

---

## Community Guidelines

### Code of Conduct

- **Be respectful** - Treat all contributors with respect
- **Be constructive** - Provide actionable feedback
- **Be collaborative** - Help others succeed
- **Be inclusive** - Welcome all skill levels and backgrounds

### Communication

- **Issues** - Bug reports, feature requests, questions
- **Pull Requests** - Code contributions
- **Discussions** - Design discussions, RFCs, general questions

### Attribution

We value all contributions:

- **Significant contributions** - Added to CONTRIBUTORS file
- **Co-authoring** - Use git co-author for pair programming
- **AI assistance** - Acknowledge Claude or other AI tools when significant

Example commit with AI co-authoring:
```
feat(renderer): implement deferred rendering

Designed architecture with help from Claude Sonnet 4.5.

Co-authored-by: Claude Sonnet 4.5 <noreply@anthropic.com>
```

---

## Getting Help

- **Documentation:** Check `docs/` directory
- **Examples:** See `examples/` directory
- **Issues:** Search existing issues or create new one
- **Discussions:** Ask questions in GitHub Discussions

---

## Quick Reference

### Pre-Commit Checklist

- [ ] `cargo fmt` - Code formatted
- [ ] `cargo clippy -- -D warnings` - No warnings
- [ ] `cargo test` - All tests pass
- [ ] `cargo doc` - Docs build
- [ ] No `println!` / `dbg!` / `eprintln!`
- [ ] No `unsafe` (unless FFI with documentation)
- [ ] Public APIs documented
- [ ] Tests added for new code
- [ ] Commit message follows conventions

### Useful Commands

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt --check

# Lint code
cargo clippy -- -D warnings

# Run tests
cargo test

# Run specific test
cargo test test_name

# Build docs
cargo doc --no-deps --open

# Watch for changes and run tests
cargo watch -x test

# Build release
cargo build --release

# Run benchmarks
cargo bench
```

---

## Resources

### Project Documentation

- [Architecture Overview](./architecture.md)
- [Coding Standards](./rules/coding-standards.md)
- [Commit Conventions](./contributing/commit-conventions.md)
- [Testing Strategy](./testing-strategy.md)
- [Development Workflow](./development-workflow.md)
- [Error Handling](./error-handling.md)
- [Platform Abstraction](./platform-abstraction.md)
- [Performance Targets](./performance-targets.md)

### External Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [How to Write a Git Commit Message](https://cbea.ms/git-commit/)

---

**Thank you for contributing to Agent Game Engine!**

We appreciate your time and effort in making this project better.

---

**Last Updated:** 2026-02-01
