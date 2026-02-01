# Workflow: Start New Phase Task

> Step-by-step guide for beginning work on a phase task from the roadmap

---

## Prerequisites

- [ ] Read [ROADMAP.md](../../ROADMAP.md)
- [ ] Read [CLAUDE.md](../../CLAUDE.md)
- [ ] Understand current phase status

---

## Step 1: Identify Task to Work On

**Check roadmap:**
```bash
cat ROADMAP.md
```

**Find tasks by phase:**
- Phase 0: Documentation, CI/CD, dev tools
- Phase 1: ECS, rendering, networking basics
- Phase 2: Advanced networking, procedural macros
- Phase 3: Physics, audio, advanced features
- Phase 4: Polish, profiling, hot-reload
- Phase 5: Examples, documentation, benchmarks

**Check task status:**
- ⚪ Not Started - Ready to begin
- 🔵 In Progress - Currently being worked on
- ✅ Complete - Finished
- ⏸️ Blocked - Waiting on dependencies

---

## Step 2: Read Task File

**Location:** `docs/tasks/phase{N}-{task-name}.md`

**Example:**
```bash
cat docs/tasks/phase1-ecs-core.md
```

**Understand:**
- [ ] Objective - What needs to be built
- [ ] Detailed tasks - Step-by-step implementation
- [ ] Acceptance criteria - How to know when done
- [ ] Performance targets - What metrics to meet
- [ ] Dependencies - What must be completed first

---

## Step 3: Check Dependencies

**Read task file footer:**
```markdown
**Dependencies:** [List of tasks that must be complete]
**Next:** [What this enables]
```

**Verify dependencies are complete:**
```bash
# Check status of dependency tasks
grep "Status:" docs/tasks/phase0-repo-setup.md
```

**If dependencies incomplete:**
- Work on dependencies first
- OR wait for them to complete
- OR discuss with team about parallel work

---

## Step 4: Create Work Branch

**Branch naming convention:**
```bash
git checkout -b feat/phase{N}-{task-name}

# Examples:
git checkout -b feat/phase1-ecs-core
git checkout -b feat/phase2-tcp-connection
git checkout -b fix/phase1-vulkan-crash
```

**Verify branch:**
```bash
git branch --show-current
```

---

## Step 5: Create Task Checklist

**Create markdown file for tracking:**
```bash
touch .claude/tasks/phase{N}-{task-name}-progress.md
```

**Template:**
```markdown
# Phase {N}: {Task Name} - Progress Tracker

**Started:** YYYY-MM-DD
**Target Completion:** YYYY-MM-DD
**Status:** In Progress

---

## Task Breakdown

### Task 1: {First major task}
- [ ] Subtask 1.1
- [ ] Subtask 1.2
- [ ] Subtask 1.3

### Task 2: {Second major task}
- [ ] Subtask 2.1
- [ ] Subtask 2.2
- [ ] Subtask 2.3

### Task 3: {Third major task}
- [ ] Subtask 3.1
- [ ] Subtask 3.2

---

## Acceptance Criteria

- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Benchmarks meet targets
- [ ] Documentation complete
- [ ] Code review complete
- [ ] No clippy warnings
- [ ] Formatted with rustfmt

---

## Performance Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| {Metric 1} | {Target} | - | ⏳ |
| {Metric 2} | {Target} | - | ⏳ |

---

## Blockers

None currently.

---

## Notes

{Add notes as you work}

---

## Daily Log

### YYYY-MM-DD
- Started task
- Implemented {feature}
- Encountered {issue}

### YYYY-MM-DD
- Fixed {issue}
- Added tests
- Benchmarked performance
```

---

## Step 6: Set Up Development Environment

**Install dependencies:**
```bash
# Rust toolchain
rustup update

# Platform-specific tools
# Windows: Vulkan SDK
# Linux: sudo apt install vulkan-tools libvulkan-dev
# macOS: brew install molten-vk

# Development tools
cargo install cargo-watch
cargo install cargo-flamegraph
cargo install criterion
```

**Verify setup:**
```bash
cargo --version
rustc --version
cargo clippy --version
cargo fmt --version
```

---

## Step 7: Read Related Documentation

**Architecture docs:**
```bash
cat docs/architecture.md
cat docs/platform-abstraction.md
cat docs/error-handling.md
cat docs/testing-strategy.md
```

**Coding standards:**
```bash
cat docs/rules/coding-standards.md
```

**Related task files:**
```bash
# Find related tasks
ls docs/tasks/phase{N}-*.md
```

---

## Step 8: Create Initial File Structure

**Based on task requirements:**
```bash
# Example for ECS core task
mkdir -p engine/core/src/ecs
touch engine/core/src/ecs/entity.rs
touch engine/core/src/ecs/component.rs
touch engine/core/src/ecs/storage.rs
touch engine/core/src/ecs/world.rs
touch engine/core/src/ecs/mod.rs
```

**Create test structure:**
```bash
mkdir -p engine/core/tests
touch engine/core/tests/ecs_integration.rs
```

**Create benchmark structure:**
```bash
mkdir -p engine/core/benches
touch engine/core/benches/ecs_benchmark.rs
```

---

## Step 9: Write Tests First (TDD)

**Start with test file:**
```rust
// engine/core/tests/ecs_integration.rs

#[test]
fn test_entity_spawn() {
    let mut world = World::new();
    let entity = world.spawn();
    assert!(world.is_alive(entity));
}

#[test]
fn test_component_add_get() {
    let mut world = World::new();
    let entity = world.spawn();

    world.add(entity, Transform::default());

    let transform = world.get::<Transform>(entity).unwrap();
    assert_eq!(transform.position, Vec3::ZERO);
}
```

**Run tests (they will fail):**
```bash
cargo test --package agent-game-engine-core
```

---

## Step 10: Implement Feature

**Follow task file steps:**
```rust
// Implement according to task specification
// Example from phase1-ecs-core.md

pub struct Entity {
    id: u32,
    generation: u32,
}

pub struct EntityAllocator {
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl EntityAllocator {
    pub fn allocate(&mut self) -> Entity {
        // Implementation...
    }
}
```

**Build incrementally:**
```bash
# Build after each major change
cargo build --package agent-game-engine-core

# Run tests frequently
cargo test --package agent-game-engine-core
```

---

## Step 11: Validate Implementation

**Run all checks:**
```bash
# Format
cargo fmt

# Lint
cargo clippy --workspace -- -D warnings

# Tests
cargo test --workspace

# Benchmarks
cargo bench --package agent-game-engine-core

# Docs
cargo doc --no-deps --open
```

---

## Step 12: Measure Performance

**Run benchmarks:**
```bash
cargo bench --package agent-game-engine-core
```

**Check against targets:**
```bash
# Results in target/criterion/
cat target/criterion/{benchmark_name}/report/index.html
```

**Update progress tracker:**
```markdown
## Performance Targets

| Metric | Target | Current | Status |
|--------|--------|---------|--------|
| Allocate entity | < 0.1μs | 0.08μs | ✅ |
| Insert component | < 0.2μs | 0.15μs | ✅ |
```

---

## Step 13: Write Documentation

**Add rustdoc:**
```rust
/// Entity handle - opaque, copyable, hashable.
///
/// Entities are lightweight handles to game objects. They use
/// generational indices to safely handle deleted entities.
///
/// # Examples
///
/// ```
/// use agent_game_engine::ecs::*;
///
/// let mut world = World::new();
/// let entity = world.spawn();
/// assert!(world.is_alive(entity));
/// ```
pub struct Entity {
    // ...
}
```

**Build and review docs:**
```bash
cargo doc --package agent-game-engine-core --no-deps --open
```

---

## Step 14: Update Task Status

**Update task file:**
```bash
# Edit docs/tasks/phase{N}-{task-name}.md
# Change status from ⚪ Not Started to 🔵 In Progress
```

**Update progress tracker:**
```markdown
## Task Breakdown

### Task 1: Entity Allocator
- [x] Subtask 1.1
- [x] Subtask 1.2
- [ ] Subtask 1.3  <- Currently working on this
```

---

## Step 15: Commit Progress

**Commit frequently:**
```bash
git add .
git commit -m "feat(ecs): implement entity allocator

- Generational entity handles
- Free list for recycling IDs
- Tests passing
- Benchmarks meet targets

Part of phase1-ecs-core task."

git push origin feat/phase1-ecs-core
```

---

## Step 16: Handle Blockers

**If you encounter a blocker:**

1. **Document it:**
```markdown
## Blockers

### Blocker 1: Vulkan SDK not available on CI
- **Impact:** Cannot run integration tests
- **Workaround:** Mock Vulkan for tests
- **Resolution:** Install SDK in CI environment
- **Owner:** DevOps team
```

2. **Notify team:**
- Create GitHub issue
- Update task status
- Discuss in team chat

3. **Work on unblocked tasks:**
- Find parallel work
- Write additional tests
- Improve documentation

---

## Step 17: Review Acceptance Criteria

**Before marking complete:**
```markdown
## Acceptance Criteria

- [x] Entity allocator implemented and tested
- [x] Sparse-set storage implemented and tested
- [x] World container implemented and tested
- [x] All unit tests pass (>20 tests)
- [x] Benchmarks meet targets
- [x] Zero unsafe code (except where necessary)
- [x] 100% rustdoc coverage for public APIs
- [x] Code formatted (rustfmt)
- [x] No clippy warnings
```

**If all checked:**
- Update task file status to ✅ Complete
- Create pull request
- Request code review

---

## Step 18: Create Pull Request

**Follow PR workflow:**
```bash
# See pr-workflow.md for complete guide
git push origin feat/phase1-ecs-core

gh pr create \
  --title "feat: Phase 1 - ECS Core Implementation" \
  --body "Implements core ECS foundation as specified in phase1-ecs-core.md

## Summary
- Entity allocator with generational indices
- Sparse-set component storage
- World container with type-safe API

## Testing
- 25+ unit tests
- Integration tests passing
- Benchmarks meet all targets

## Performance
- Spawn 10k entities: 0.8ms (target: <1ms)
- Query 10k components: 0.4ms (target: <0.5ms)

Closes #123"
```

---

## Common Issues and Solutions

### Issue: Tests failing on CI but passing locally
```bash
# Run tests exactly as CI does
./scripts/ci-local.sh

# Check for platform-specific issues
cargo test --target x86_64-pc-windows-msvc
cargo test --target x86_64-unknown-linux-gnu
```

---

### Issue: Performance targets not met
```bash
# Profile the code
cargo build --features profiling
./target/debug/client
# Open Tracy

# Or use flamegraph
cargo flamegraph --bench my_bench

# Optimize hot paths
# - Reduce allocations
# - Use better data structures
# - Add benchmarks for iterations
```

---

### Issue: Unclear requirements
```bash
# Check related tasks
ls docs/tasks/

# Read architecture docs
cat docs/architecture.md

# Ask for clarification
# - Create GitHub issue
# - Tag task owner
# - Discuss in team chat
```

---

## Validation Checklist

- [ ] Task file read and understood
- [ ] Dependencies verified
- [ ] Work branch created
- [ ] Progress tracker created
- [ ] Development environment set up
- [ ] Related docs read
- [ ] File structure created
- [ ] Tests written first (TDD)
- [ ] Feature implemented
- [ ] All tests passing
- [ ] Performance targets met
- [ ] Documentation complete
- [ ] Task status updated
- [ ] Code committed
- [ ] PR created

---

## Daily Workflow

**Start of day:**
1. Pull latest changes
2. Review progress tracker
3. Check for blockers
4. Plan today's work

**During day:**
1. Implement feature incrementally
2. Run tests frequently
3. Commit progress regularly
4. Update progress tracker

**End of day:**
1. Commit all work
2. Push to remote
3. Update progress tracker
4. Note blockers/questions

---

## References

- [ROADMAP.md](../../ROADMAP.md) - All phase tasks
- [CLAUDE.md](../../CLAUDE.md) - AI agent guide
- [docs/development-workflow.md](../../docs/development-workflow.md) - Dev workflow
- [.claude/workflows/pr-workflow.md](pr-workflow.md) - PR creation

---

**Last Updated:** 2026-02-01
