# Commit Conventions

> **Git commit message guidelines for the Agent Game Engine project**
>
> These conventions ensure a clean, searchable commit history and enable automated tooling.

---

## Overview

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. All commits must use this format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

---

## 1. Conventional Commits Format

### Basic Structure

```
type(scope): subject
```

### Full Structure (with optional body and footer)

```
type(scope): subject

Optional body that explains the motivation for the change.
This is the place to explain WHY, not WHAT.

Optional footer with issue references or breaking changes.
Closes #123
BREAKING CHANGE: Brief description of breaking change
```

---

## 2. Commit Types

### Primary Types

| Type | Purpose | When to Use |
|------|---------|-------------|
| `feat` | New features | Adding new functionality that users/developers can use |
| `fix` | Bug fixes | Fixing incorrect behavior or errors |
| `docs` | Documentation only | Changes that only affect documentation (code comments, markdown files) |
| `style` | Code style/formatting | Formatting changes, missing semicolons, whitespace (no logic change) |
| `refactor` | Code refactoring | Restructuring code without changing behavior (no new features or fixes) |
| `perf` | Performance improvements | Changes that improve performance characteristics |
| `test` | Tests | Adding missing tests or correcting existing tests |
| `chore` | Build/tooling | Changes to build process, dependencies, or development tools |
| `ci` | CI/CD changes | Changes to continuous integration and deployment pipelines |
| `build` | Build system | Changes that affect the build system or external dependencies |

### Type Usage Guidelines

**feat** - New Features
```
feat(ecs): add support for component queries with filters
feat(renderer): implement PBR material system
feat(networking): add UDP packet compression
```

**fix** - Bug Fixes
```
fix(ecs): prevent entity despawn during iteration
fix(renderer): resolve Vulkan validation layer warnings
fix(networking): correct byte order in packet serialization
```

**docs** - Documentation
```
docs(architecture): update ECS design documentation
docs(api): add examples to World::spawn documentation
docs(readme): clarify installation requirements
```

**style** - Code Style (no logic change)
```
style(core): run cargo fmt on entity module
style(renderer): fix clippy warnings in shader loader
style(all): enforce 100 char line limit
```

**refactor** - Code Refactoring
```
refactor(ecs): extract query logic into separate module
refactor(renderer): simplify pipeline creation API
refactor(networking): move serialization to separate crate
```

**perf** - Performance Improvements
```
perf(ecs): optimize component lookup with hash map cache
perf(renderer): reduce draw call overhead with batching
perf(networking): use object pooling for packet allocations
```

**test** - Tests
```
test(ecs): add integration tests for system execution
test(renderer): add property tests for vertex buffer validation
test(networking): add stress tests for connection handling
```

**chore** - Build Process/Tooling/Dependencies
```
chore(deps): update ash to 0.38.0
chore(tooling): add pre-commit hooks for formatting
chore(workspace): organize Cargo.toml dependencies
```

**ci** - CI/CD Changes
```
ci(github): add automated benchmark workflow
ci(coverage): integrate code coverage reporting
ci(deploy): automate documentation deployment
```

**build** - Build System Changes
```
build(cargo): enable LTO for release builds
build(features): add new 'profiling' feature flag
build(workspace): configure shared dependencies
```

---

## 3. Scope Examples

The scope indicates which part of the codebase is affected. Use lowercase.

### Common Scopes

**Core Systems:**
- `core` - Core engine functionality
- `ecs` - Entity Component System
- `world` - World management
- `entity` - Entity operations
- `component` - Component system
- `system` - System execution
- `query` - Query system

**Rendering:**
- `renderer` - General rendering
- `vulkan` - Vulkan-specific code
- `shader` - Shader management
- `pipeline` - Pipeline creation
- `mesh` - Mesh handling
- `texture` - Texture loading
- `material` - Material system
- `lighting` - Lighting system
- `pbr` - Physically-based rendering

**Networking:**
- `networking` - General networking
- `network` - Network protocol
- `server` - Server-side code
- `client` - Client-side code
- `replication` - State replication
- `prediction` - Client prediction
- `sync` - Synchronization

**Other:**
- `physics` - Physics system
- `audio` - Audio system
- `platform` - Platform abstraction
- `serialization` - Serialization/deserialization
- `testing` - Testing infrastructure
- `ci` - CI/CD infrastructure
- `docs` - Documentation
- `examples` - Example applications

### Scope Usage

```
feat(ecs): add parallel system execution
fix(vulkan): resolve memory leak in texture upload
refactor(networking): simplify packet encoding
perf(query): optimize component iteration
test(serialization): add roundtrip tests
docs(architecture): document networking design
chore(deps): update dependencies
```

### Multiple Scopes

For changes affecting multiple areas, use the most specific applicable scope or the higher-level scope:

```
feat(renderer): add support for multiple render passes
// affects vulkan, pipeline, shader - use 'renderer'

fix(networking): resolve desync in state replication
// affects server, client, sync - use 'networking'
```

---

## 4. Subject Guidelines

The subject is a brief description of the change.

### Rules

1. **Use imperative mood** - "add feature" not "added feature" or "adds feature"
2. **Start with lowercase** - "add PBR materials" not "Add PBR materials"
3. **No period at the end** - "fix crash" not "fix crash."
4. **Maximum 72 characters** - Keep it concise
5. **Be specific** - Describe what the commit does, not what issue it fixes

### Good Examples

```
feat(ecs): add support for optional components in queries
fix(renderer): prevent crash when swapchain recreation fails
refactor(networking): extract replication logic to separate module
perf(ecs): reduce allocation overhead in query iteration
docs(api): add examples for entity spawning
test(serialization): add property tests for component encoding
```

### Bad Examples

```
feat(ecs): Added support for optional components    // ❌ Past tense
fix(renderer): Fix crash.                           // ❌ Capitalized, has period
refactor(networking): Stuff                         // ❌ Not specific
perf(ecs): This makes queries faster by using a cache instead of linear search which was slow  // ❌ Too long
docs(api): Fixed #123                               // ❌ Doesn't describe the change
test(serialization): Tests                          // ❌ Not specific
```

---

## 5. Body Format (Optional)

The body provides additional context about the change. It explains WHY the change was made, not WHAT was changed (the diff shows that).

### Guidelines

- Separate from subject with a blank line
- Wrap at 72 characters per line
- Explain the motivation for the change
- Explain what problem it solves
- Reference issues if applicable
- Explain alternatives considered
- Note any limitations or side effects

### Example

```
feat(ecs): add parallel system execution

Systems can now execute in parallel when they don't have conflicting
component access. This significantly improves performance for worlds
with many systems and entities.

The parallel executor uses rayon for work-stealing parallelism and
automatically detects dependencies based on component read/write access.
Systems with conflicts are automatically serialized.

Performance testing shows 2.5x improvement on 8-core systems with
typical game workloads.

Relates to #234
```

### When to Include a Body

**Required for:**
- Breaking changes
- Complex features
- Non-obvious bug fixes
- Performance optimizations (include benchmark results)
- Refactorings that change architecture

**Optional for:**
- Simple bug fixes
- Documentation updates
- Trivial changes

---

## 6. Footer Format (Optional)

The footer contains metadata about the commit.

### Breaking Changes

Use `BREAKING CHANGE:` to indicate incompatible API changes:

```
feat(ecs): redesign query API for better performance

BREAKING CHANGE: Query::iter() now returns QueryIter instead of
std::slice::Iter. Update all query iteration to use the new API.

Migration:
- Old: world.query::<&Transform>().iter()
- New: world.query::<&Transform>().iter()
```

### Issue References

Reference issues using these keywords:

- `Closes #123` - Closes an issue
- `Fixes #123` - Fixes a bug issue
- `Resolves #123` - Resolves an issue
- `Relates to #123` - Related but doesn't close
- `Ref #123` - Reference for context

### Multiple References

```
feat(renderer): add deferred rendering pipeline

Implements a deferred rendering pipeline with support for many lights.
Includes G-buffer generation and light accumulation passes.

Closes #156
Relates to #134, #145
```

### Co-Author Attribution

Use `Co-authored-by:` to attribute multiple authors:

```
feat(physics): integrate rapier physics engine

Co-authored-by: Jane Developer <jane@example.com>
Co-authored-by: Claude Sonnet 4.5 <noreply@anthropic.com>
```

---

## 7. Examples of Good Commits

### Feature Addition

```
feat(renderer): add cascaded shadow maps

Implements cascaded shadow mapping for directional lights with 4
cascades. Automatically adjusts cascade splits based on camera
frustum for optimal shadow quality distribution.

Performance impact: ~2ms per directional light on GTX 1060.

Closes #178
```

### Bug Fix

```
fix(networking): prevent packet loss during congestion

The send buffer was not properly handling the case where the
underlying socket returned EWOULDBLOCK. This caused packets to be
silently dropped during network congestion.

Now we queue unsent packets and retry on the next tick.

Fixes #203
```

### Refactoring

```
refactor(ecs): extract component storage into trait

Component storage is now abstracted behind the ComponentStorage trait,
allowing for different storage strategies (dense, sparse, hierarchical).
This prepares the codebase for future optimizations without breaking
the public API.

No behavior changes.

Relates to #167
```

### Performance Improvement

```
perf(query): use SIMD for component filtering

Replaced scalar component type matching with SIMD operations using
std::simd. This speeds up query iteration by ~40% in benchmarks.

Benchmark results (entity_iteration/1000 entities):
- Before: 45 μs
- After:  27 μs

Closes #189
```

### Documentation

```
docs(architecture): document networking state replication

Added comprehensive documentation of the state replication system,
including:
- Entity interest management
- Delta compression
- Priority-based updates
- Client prediction integration

Closes #145
```

### Test Addition

```
test(ecs): add stress tests for concurrent entity operations

Added property-based tests using proptest to verify correctness of
entity operations under concurrent access. Tests spawn/despawn/modify
operations with random interleavings.

Found and fixed 2 race conditions during development.
```

### Chore

```
chore(deps): update ash to 0.38.0 and gpu-allocator to 0.27.0

Both dependencies had patch releases with bug fixes.
Updated all Vulkan-related code to use new APIs where needed.

Tested on Windows, Linux, and macOS.
```

---

## 8. Examples of Bad Commits

### Vague

```
fix(core): fix bug          // ❌ What bug? How was it fixed?
feat(renderer): add stuff   // ❌ What stuff?
chore: updates              // ❌ What was updated?
```

### Wrong Type

```
feat(ecs): fix crash in query system    // ❌ Should be 'fix'
fix(renderer): add shadow mapping       // ❌ Should be 'feat'
docs(api): refactor documentation       // ❌ Should be 'refactor' if restructuring
```

### Poor Subject

```
feat(ECS): Add Component System         // ❌ Capitalized type and subject
fix(renderer): Fixed the Vulkan crash.  // ❌ Past tense, capitalized, period
refactor(networking): This refactors the packet serialization to use bincode instead of serde_json which is much faster  // ❌ Too long
```

### Missing Context

```
fix(ecs): change query behavior
// ❌ Should explain why it was changed and what the old behavior was
```

### Mixed Changes

```
feat(renderer): add PBR materials and fix shadow bugs and update docs
// ❌ Should be 3 separate commits
```

---

## 9. How to Write Atomic Commits

### What is an Atomic Commit?

An atomic commit is a commit that:
- Contains one logical change
- Can be reverted independently
- Doesn't break the build
- Has all related changes (tests, docs, code)

### Guidelines

**DO:**
- One feature per commit
- One bug fix per commit
- Include tests with the feature/fix
- Include documentation updates with API changes
- Ensure the code compiles after each commit
- Ensure tests pass after each commit

**DON'T:**
- Mix multiple features in one commit
- Mix features and bug fixes
- Commit broken code (even temporarily)
- Commit formatting changes with logic changes
- Make commits too granular (every line change)

### Example Workflow

**Good: Separate Commits**
```bash
# Commit 1: Add feature
git add src/renderer/pbr.rs tests/pbr_tests.rs
git commit -m "feat(renderer): add PBR material system"

# Commit 2: Update docs
git add docs/rendering.md
git commit -m "docs(renderer): document PBR material usage"

# Commit 3: Add examples
git add examples/pbr_demo.rs
git commit -m "docs(examples): add PBR material demo"
```

**Bad: Mixed Commits**
```bash
# ❌ Everything in one commit
git add .
git commit -m "feat(renderer): add PBR and fix shadow bugs and update docs"
```

### Splitting Changes

If you've made multiple changes, use `git add -p` to stage parts:

```bash
# Review and stage changes interactively
git add -p src/renderer/material.rs

# Commit the staged changes
git commit -m "feat(renderer): add metallic-roughness workflow"

# Stage and commit the remaining changes
git add -p src/renderer/material.rs
git commit -m "refactor(renderer): simplify material parameter handling"
```

### Amending Commits

If you forgot something in the last commit:

```bash
# Make the change
vim src/renderer/pbr.rs

# Add to the previous commit
git add src/renderer/pbr.rs
git commit --amend --no-edit
```

**Warning:** Only amend commits that haven't been pushed or shared!

---

## 10. Co-Authoring (Including Claude)

### When to Co-Author

Use co-authoring when:
- Pair programming
- Significant contributions from multiple people
- AI-assisted development (Claude, Copilot, etc.)
- Code review with substantial changes
- Merging someone else's work manually

### Format

Add co-authors in the commit footer:

```
feat(renderer): implement deferred rendering pipeline

Implements G-buffer generation, light accumulation, and post-processing.
Supports up to 1024 dynamic lights with efficient culling.

Co-authored-by: Alice Developer <alice@example.com>
Co-authored-by: Bob Engineer <bob@example.com>
Co-authored-by: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### AI Co-Authoring Guidelines

**When Claude contributes significantly:**
- Generated substantial code (>30% of the change)
- Designed the architecture or approach
- Debugged complex issues
- Wrote tests or documentation

**Example:**
```
fix(ecs): resolve deadlock in parallel system execution

Claude identified the issue: systems were acquiring locks in
inconsistent order. Solution uses global lock ordering based on
component type IDs.

Co-authored-by: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**When NOT to co-author with Claude:**
- Simple code completion
- Minor suggestions
- Answering questions without code changes
- Reviewing existing code

### Multiple Co-Authors

List all contributors:

```
feat(networking): implement delta compression

Delta compression reduces bandwidth by 80% for typical game state.
Uses XOR-based diffing with run-length encoding.

Benchmarking by Alice, implementation by Bob, optimization by Charlie.

Co-authored-by: Alice Benchmark <alice@example.com>
Co-authored-by: Bob Developer <bob@example.com>
Co-authored-by: Charlie Optimizer <charlie@example.com>
Co-authored-by: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### Format Requirements

- Use exact format: `Co-authored-by: Name <email>`
- Place in footer after issue references
- One co-author per line
- Alphabetical order (optional but recommended)
- Claude's email: `noreply@anthropic.com`

---

## Commit Message Checklist

Before committing, verify:

- [ ] Type is correct (feat, fix, docs, etc.)
- [ ] Scope is appropriate and lowercase
- [ ] Subject uses imperative mood
- [ ] Subject is lowercase and has no period
- [ ] Subject is under 72 characters
- [ ] Body explains WHY, not WHAT (if included)
- [ ] Body lines are wrapped at 72 characters
- [ ] Footer has proper issue references
- [ ] Footer has BREAKING CHANGE if applicable
- [ ] Co-authors are properly attributed
- [ ] Code compiles and tests pass
- [ ] Commit is atomic (one logical change)

---

## Tools and Automation

### Git Hooks

Set up commit message validation:

```bash
# Install commitlint (requires Node.js)
npm install -g @commitlint/cli @commitlint/config-conventional

# Add to .git/hooks/commit-msg
#!/bin/sh
npx commitlint --edit $1
```

### Editor Integration

**VSCode:**
- Install "Conventional Commits" extension
- Provides commit message template

**IntelliJ/CLion:**
- Use "Git Commit Template" plugin
- Configure template in settings

### Commit Message Template

Create `.gitmessage` template:

```
# <type>(<scope>): <subject>
#
# <body>
#
# <footer>

# Types: feat, fix, docs, style, refactor, perf, test, chore, ci, build
# Scopes: core, ecs, renderer, networking, physics, audio, platform, etc.
# Subject: imperative mood, lowercase, no period, <72 chars
#
# Body: Explain WHY, not WHAT. Wrap at 72 chars.
#
# Footer:
# - Closes #123
# - BREAKING CHANGE: description
# - Co-authored-by: Name <email>
```

Configure git to use it:

```bash
git config commit.template .gitmessage
```

---

## Resources

- [Conventional Commits Specification](https://www.conventionalcommits.org/)
- [Semantic Versioning](https://semver.org/)
- [Git Commit Best Practices](https://chris.beams.io/posts/git-commit/)
- [How to Write a Git Commit Message](https://cbea.ms/git-commit/)

---

**Last Updated:** 2026-02-01
