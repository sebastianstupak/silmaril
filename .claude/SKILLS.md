# Claude Code Skills Reference

> **Comprehensive guide to creating and using skills in the Agent Game Engine project**

---

## Table of Contents

- [Overview](#overview)
- [Skill Fundamentals](#skill-fundamentals)
- [Creating Skills](#creating-skills)
- [Skill Examples](#skill-examples)
- [Advanced Patterns](#advanced-patterns)
- [Skill Reference](#skill-reference)

---

## Overview

Skills are reusable, filesystem-based resources that provide Claude with domain-specific expertise, workflows, and best practices. Unlike one-off prompts, skills load automatically when relevant and eliminate the need to repeatedly provide the same guidance.

### Key Concepts

**Skills vs Prompts**:
- **Prompts**: One-time instructions in a conversation
- **Skills**: Persistent workflows loaded on-demand

**Progressive Loading**:
1. **Metadata** (always loaded): Name and description
2. **Instructions** (loaded when triggered): Main SKILL.md content
3. **Resources** (loaded as needed): Supporting files, scripts, examples

**When to Use Skills**:
- ✅ Repetitive workflows (code review, commits, deployments)
- ✅ Domain-specific knowledge (API conventions, coding standards)
- ✅ Task automation (testing, formatting, validation)
- ❌ One-off tasks (better as direct prompts)

---

## Skill Fundamentals

### Skill Structure

Every skill is a directory with a `SKILL.md` file:

```
my-skill/
├── SKILL.md           # Required: main instructions
├── REFERENCE.md       # Optional: detailed documentation
├── EXAMPLES.md        # Optional: usage examples
├── template.md        # Optional: templates to fill
└── scripts/
    └── helper.sh      # Optional: executable scripts
```

### SKILL.md Format

```yaml
---
name: skill-name
description: Clear description of what the skill does and when to use it
disable-model-invocation: false
user-invocable: true
allowed-tools: Read, Grep, Glob
model: sonnet
context: fork
agent: Explore
---

# Skill Title

Main instructions for Claude to follow when this skill is invoked.

## Usage

Step-by-step guidance...

## Examples

Concrete examples...
```

### Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Recommended | Skill name (lowercase, hyphens only). Defaults to directory name |
| `description` | Recommended | What the skill does and when to use it. Claude uses this for auto-invocation |
| `argument-hint` | No | Hint shown during autocomplete (e.g., `[filename]`) |
| `disable-model-invocation` | No | Set `true` to prevent auto-invocation (manual `/skill-name` only) |
| `user-invocable` | No | Set `false` to hide from menu (background knowledge only) |
| `allowed-tools` | No | Tools Claude can use without permission when skill is active |
| `model` | No | Model to use (`sonnet`, `opus`, `haiku`, or `inherit`) |
| `context` | No | Set `fork` to run in isolated subagent |
| `agent` | No | Subagent type when `context: fork` (e.g., `Explore`, `Plan`) |

### Invocation Modes

| Frontmatter | You Can Invoke | Claude Can Invoke | Use Case |
|-------------|---------------|-------------------|----------|
| (default) | Yes | Yes | General-purpose skills |
| `disable-model-invocation: true` | Yes | No | Manual workflows (deploy, commit) |
| `user-invocable: false` | No | Yes | Background knowledge |

---

## Creating Skills

### Method 1: Interactive Creation

```
# Ask Claude
What skills are available?
# Follow prompts to create new skill
```

### Method 2: Manual Creation

```bash
# Create skill directory
mkdir -p .claude/skills/my-skill

# Create SKILL.md
cat > .claude/skills/my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: Description here
---

# My Skill

Instructions here
EOF
```

### Skill Locations

| Location | Scope | Use Case |
|----------|-------|----------|
| `~/.claude/skills/` | All your projects | Personal workflows |
| `.claude/skills/` | This project only | Project-specific (commit to git) |
| Plugin `skills/` | Where plugin enabled | Share via Claude Code plugins |

### Passing Arguments

Skills can accept arguments via `$ARGUMENTS` placeholder:

```yaml
---
name: fix-issue
description: Fix a GitHub issue by number
---

Fix GitHub issue $ARGUMENTS following coding standards.

1. Read issue description
2. Implement fix
3. Write tests
4. Commit changes
```

**Usage**:
```
/fix-issue 123
# Becomes: "Fix GitHub issue 123 following coding standards."
```

**Individual Arguments**:
```yaml
---
name: migrate-component
description: Migrate component between frameworks
---

Migrate $0 from $1 to $2.
```

**Usage**:
```
/migrate-component SearchBar React Vue
# $0 = SearchBar, $1 = React, $2 = Vue
```

---

## Skill Examples

### Code Review Skill

**Purpose**: Review code for quality, security, and best practices

`.claude/skills/review-code/SKILL.md`:
```yaml
---
name: review-code
description: Review code for quality, security, and best practices. Use when code changes are made or when explicitly asked.
allowed-tools: Read, Grep, Glob
---

# Code Review Skill

When reviewing code, analyze these aspects:

## 1. Code Quality

- **Naming**: Variables, functions, types are descriptive
- **Complexity**: Functions are focused and understandable
- **Duplication**: No repeated code blocks
- **Comments**: Complex logic is explained

## 2. Security

- **Secrets**: No API keys, passwords in code
- **Validation**: User inputs are validated
- **Error Handling**: No sensitive data in error messages
- **Authentication**: Proper access controls

## 3. Project Standards

Reference [CLAUDE.md](../../CLAUDE.md) for:
- No `println!`, use `tracing` macros
- Custom error types (no `anyhow` or `Box<dyn Error>`)
- Platform abstraction (no `#[cfg]` in business logic)
- Testing requirements

## 4. Performance

- Efficient algorithms and data structures
- Appropriate use of allocations
- No obvious bottlenecks

## 5. Testing

- Unit tests for functions
- Integration tests for features
- Property tests for serialization/math
- Edge cases covered

## Output Format

Organize findings by priority:

### Critical (Must Fix)
- Security vulnerabilities
- Bugs that break functionality
- Violations of MANDATORY rules

### Important (Should Fix)
- Code quality issues
- Missing tests
- Performance problems

### Suggestions (Consider)
- Style improvements
- Refactoring opportunities
- Documentation additions

Include:
- File path and line numbers
- Current code snippet
- Proposed fix
- Rationale for change
```

**Supporting File** - `.claude/skills/review-code/CHECKLIST.md`:
```markdown
# Code Review Checklist

## Security Checklist

- [ ] No hardcoded credentials
- [ ] No SQL injection vulnerabilities
- [ ] Proper input validation
- [ ] Safe error handling
- [ ] Authentication checks

## Quality Checklist

- [ ] Clear naming
- [ ] No code duplication
- [ ] Proper error handling
- [ ] Efficient algorithms
- [ ] Tests included

## Project Standards

- [ ] No print statements
- [ ] Custom error types
- [ ] Platform abstraction
- [ ] Documentation
```

**Usage**:
```
/review-code src/ecs/systems/health.rs

# Or let Claude invoke automatically
Review the recent changes for issues
```

### Commit Skill

**Purpose**: Create well-formatted commits following project conventions

`.claude/skills/commit/SKILL.md`:
```yaml
---
name: commit
description: Create git commits following project conventions
disable-model-invocation: true
allowed-tools: Bash(git *)
---

# Git Commit Skill

Create commits following this project's standards.

## Process

1. **Check Status**
   ```bash
   git status
   ```

2. **Review Changes**
   ```bash
   git diff
   ```

3. **Draft Message**
   - Subject: Imperative mood, < 70 chars
   - Body: Explain WHY, not what
   - Footer: Issue references, breaking changes

4. **Stage Files**
   - Stage specific files, not `git add -A`
   - Avoid staging: `.env`, temp files, binaries

5. **Commit**
   ```bash
   git commit -m "$(cat <<'EOF'
   Subject line here

   Detailed explanation of changes.

   Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
   EOF
   )"
   ```

## Message Format

```
<type>: <subject>

<body>

<footer>

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

## Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code restructuring
- `test`: Test additions/changes
- `chore`: Maintenance tasks
- `perf`: Performance improvements

## Examples

### Feature Commit
```
feat: add health regeneration system

Implement ECS system for automatic health recovery over time.
Components: Health, RegenerationRate
Systems: health_regeneration_system

Addresses health mechanics requirement from #42

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### Bug Fix Commit
```
fix: prevent health overflow in regeneration

Health could exceed max_health when regeneration rate was high.
Now clamps to max_health using .min().

Fixes #156

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```
```

**Usage**:
```
/commit
```

### Testing Skill

**Purpose**: Run tests and fix failures

`.claude/skills/test-runner/SKILL.md`:
```yaml
---
name: test-runner
description: Run tests and fix failures. Use proactively after code changes.
context: fork
agent: general-purpose
allowed-tools: Bash, Read, Edit, Grep
---

# Test Runner Skill

Run tests and fix failures systematically.

## Process

1. **Run Full Suite**
   ```bash
   cargo test --all-features
   ```

2. **Identify Failures**
   - Parse test output
   - List failing tests
   - Categorize by module

3. **For Each Failure**
   a. Read test code
   b. Read implementation
   c. Identify root cause
   d. Implement minimal fix
   e. Re-run test
   f. Verify fix

4. **Re-run Full Suite**
   ```bash
   cargo test --all-features
   ```

5. **Report Results**
   - Tests fixed
   - Tests still failing
   - New issues introduced

## Fix Strategies

### Compilation Errors
1. Read error message carefully
2. Check recent changes
3. Fix type mismatches, missing imports

### Logic Errors
1. Add debug logging
2. Check expected vs actual
3. Fix algorithm/logic

### Flaky Tests
1. Identify non-determinism
2. Fix race conditions
3. Mock external dependencies

## Output Format

```
Test Results:
✓ 47 tests passed
✗ 3 tests failed

Failures Fixed:
1. test_health_regeneration
   - Issue: Overflow when regen_rate high
   - Fix: Clamp to max_health
   - Status: FIXED

2. test_entity_despawn
   - Issue: Component not removed
   - Fix: Call world.remove_all()
   - Status: FIXED

Remaining Failures:
3. test_network_sync
   - Issue: Timing-dependent failure
   - Next: Needs investigation
```
```

**Usage**:
```
/test-runner

# Or automatic
Use test-runner to fix failing tests
```

### Documentation Skill

**Purpose**: Generate documentation from code

`.claude/skills/document/SKILL.md`:
```yaml
---
name: document
description: Generate documentation for code, modules, and APIs
allowed-tools: Read, Grep, Glob, Write
---

# Documentation Skill

Generate comprehensive documentation from code.

## Process

1. **Analyze Code**
   - Read source files
   - Identify public API
   - Understand structure

2. **Generate Docs**
   - Module overview
   - Public types/functions
   - Usage examples
   - Error handling

3. **Write Documentation**
   - README.md for modules
   - Rustdoc comments for items
   - Examples in docs/

## Documentation Levels

### Module Documentation

```rust
//! # Module Name
//!
//! Brief description of module purpose.
//!
//! ## Overview
//!
//! Detailed explanation...
//!
//! ## Examples
//!
//! ```
//! use module::Type;
//!
//! let instance = Type::new();
//! ```
```

### Item Documentation

```rust
/// Brief description of function.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function fails
///
/// # Examples
///
/// ```
/// let result = function(param);
/// ```
pub fn function(param: Type) -> Result<Output, Error> {
    // ...
}
```

## Output Format

Generate:
1. Rustdoc comments in source
2. README.md for module
3. Examples in docs/examples/
4. API reference in docs/
```

**Usage**:
```
/document src/ecs/world.rs

# Generate README
/document the networking module and create a README
```

### Debugging Skill

**Purpose**: Systematic debugging workflow

`.claude/skills/debug/SKILL.md`:
```yaml
---
name: debug
description: Debug errors, failures, and unexpected behavior systematically
context: fork
allowed-tools: Read, Edit, Bash, Grep, Glob
---

# Debugging Skill

Systematic approach to finding and fixing bugs.

## Process

1. **Capture Error**
   - Get error message
   - Get stack trace
   - Identify file/line

2. **Reproduce**
   - Minimal reproduction steps
   - Consistent environment
   - Document inputs

3. **Isolate**
   - Binary search for cause
   - Comment out sections
   - Add logging

4. **Diagnose**
   - Form hypothesis
   - Test hypothesis
   - Identify root cause

5. **Fix**
   - Minimal change
   - Preserve existing behavior
   - Add regression test

6. **Verify**
   - Original case works
   - Tests pass
   - No new issues

## Debugging Tools

### Add Logging
```rust
use tracing::{debug, info, error};

debug!(value = ?x, "Checkpoint A");
```

### Run Specific Test
```bash
cargo test test_name -- --nocapture
```

### Enable Trace Logging
```bash
RUST_LOG=trace cargo run
```

### Use Debugger
```bash
rust-gdb target/debug/binary
```

## Output Format

```
Bug Analysis:
-----------
Error: "index out of bounds"
Location: src/ecs/world.rs:142
Trigger: Despawning entity while iterating

Root Cause:
----------
Iterator invalidated when entity removed during iteration.

Fix:
---
Collect entity IDs first, then despawn in separate loop.

Verification:
-----------
✓ Original case fixed
✓ All tests pass
✓ Regression test added
```
```

**Usage**:
```
/debug

# Or with context
Debug why the health regeneration system crashes
```

---

## Advanced Patterns

### Dynamic Context Injection

Use `!`command`` syntax to run shell commands before skill content is sent:

```yaml
---
name: pr-summary
description: Summarize pull request changes
context: fork
---

## Pull Request Context

- Diff: !`gh pr diff`
- Comments: !`gh pr view --comments`
- Files: !`gh pr diff --name-only`

## Task

Summarize this PR focusing on:
1. What changed
2. Why it changed
3. Potential risks
```

Commands run immediately, output replaces placeholder:
1. `gh pr diff` executes
2. Output inserted into prompt
3. Claude receives fully-rendered content

### Skills with Subagents

Use `context: fork` to run skill in isolated subagent:

```yaml
---
name: research
description: Deep research on codebase topic
context: fork
agent: Explore
---

Research $ARGUMENTS thoroughly:

1. Find all relevant files
2. Analyze implementations
3. Identify patterns
4. Summarize findings
```

**When to Fork**:
- ✅ Self-contained research tasks
- ✅ High-volume output operations
- ✅ Isolated validation checks
- ❌ Interactive workflows
- ❌ Tasks needing conversation context

### Bundled Scripts

Skills can include executable scripts:

`.claude/skills/visualize/SKILL.md`:
```yaml
---
name: visualize
description: Generate codebase visualization
allowed-tools: Bash(python *)
---

# Codebase Visualizer

Generate interactive HTML tree view.

Run:
```bash
python ~/.claude/skills/visualize/scripts/visualize.py .
```

Opens codebase-map.html in browser showing:
- Collapsible directory tree
- File sizes
- Type breakdowns
```

`.claude/skills/visualize/scripts/visualize.py`:
```python
#!/usr/bin/env python3
import json
from pathlib import Path

# Script generates HTML visualization
# (see README.md example for full implementation)
```

### Supporting Files Pattern

Keep SKILL.md focused, move details to separate files:

```yaml
---
name: api-conventions
description: API design patterns for this codebase
---

# API Conventions

Follow these patterns when designing APIs:

## REST Conventions
See [rest-guide.md](rest-guide.md) for detailed REST patterns.

## Error Handling
See [errors.md](errors.md) for error response formats.

## Examples
See [examples/](examples/) for API endpoint examples.
```

**Benefits**:
- SKILL.md stays under 500 lines
- Claude loads details only when needed
- Easier to maintain and update

---

## Skill Reference

### Common Patterns

#### Validation Skill

```yaml
---
name: validate
description: Validate code follows project standards
allowed-tools: Read, Grep, Bash
---

Check for:
1. No println!/dbg!/eprintln! (use tracing)
2. Custom error types (no anyhow/Box<dyn Error>)
3. Platform abstraction (no #[cfg] in business logic)
4. Tests included
5. Documentation present
```

#### Deployment Skill

```yaml
---
name: deploy
description: Deploy application to production
disable-model-invocation: true
---

1. Run test suite
2. Build release binary
3. Run security checks
4. Deploy to environment
5. Verify deployment
6. Monitor for issues
```

#### Performance Analysis Skill

```yaml
---
name: perf-analyze
description: Analyze performance and identify bottlenecks
context: fork
allowed-tools: Bash, Read, Grep
---

1. Run benchmarks
2. Profile with Tracy
3. Analyze results
4. Identify hotspots
5. Suggest optimizations
```

### Best Practices

1. **Keep SKILL.md Focused**: < 500 lines, move details to supporting files
2. **Clear Descriptions**: Help Claude know when to auto-invoke
3. **Concrete Examples**: Show don't tell
4. **Tool Restrictions**: Grant minimum necessary permissions
5. **Test Your Skills**: Invoke manually to verify behavior

### Troubleshooting

**Skill Not Auto-Invoking**:
- Check description is clear and relevant to task
- Try explicit invocation: `/skill-name`
- Verify skill in "What skills are available?"

**Skill Errors**:
- Validate YAML frontmatter
- Check script permissions (`chmod +x`)
- Test scripts independently
- Check logs: `claude --debug`

**Context Issues**:
- Use `context: fork` for high-volume output
- Keep main skills under 500 lines
- Reference supporting files when needed

---

## Additional Resources

- **[README.md](./README.md)**: General Claude Code setup guide
- **[AGENTS.md](./AGENTS.md)**: Agent configuration guide
- **[Official Skills Docs](https://code.claude.com/docs/en/skills)**: Complete reference
- **[Skills Repository](https://github.com/anthropics/skills)**: Community skills

---

**Last Updated**: 2026-02-01
