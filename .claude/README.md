# Claude Code Configuration

> **Complete guide to using Claude Code with the Agent Game Engine project**
>
> This document covers the new config.json structure, custom prompts, git hooks, agents, and best practices for working with this codebase.

---

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration Files](#configuration-files)
- [Custom Prompts](#custom-prompts)
- [Git Hooks](#git-hooks)
- [Agent Configurations](#agent-configurations)
- [Available Skills](#available-skills)
- [Workflow Templates](#workflow-templates)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

---

## Overview

This project uses Claude Code's advanced features to streamline development:

- **Config.json**: Centralized project configuration with agents, hooks, and rules
- **Custom Prompts**: Phase-guide and code-review prompts for context-aware assistance
- **Git Hooks**: Automated pre-commit and post-test quality checks
- **Agents**: Specialized roles (architect, implementer, reviewer, tester)
- **Skills**: Reusable workflows and domain expertise
- **Workflows**: Common development patterns and task sequences

### What's New

This configuration adds:
- **D:\dev\agent-game-engine\.claude\config.json** - Main configuration file
- **D:\dev\agent-game-engine\.claude\prompts\phase-guide.md** - Phase-aware development guide
- **D:\dev\agent-game-engine\.claude\prompts\code-review.md** - Comprehensive review checklist
- **D:\dev\agent-game-engine\.claude\hooks\pre-commit.sh** - Quality checks before commit
- **D:\dev\agent-game-engine\.claude\hooks\post-test.sh** - Test metrics and coverage reporting

### Directory Structure

```
.claude/
├── config.json              # Main configuration (NEW)
├── README.md                # This file
├── prompts/                 # Custom prompts (NEW)
│   ├── phase-guide.md       # Phase-aware guidance
│   └── code-review.md       # Code review checklist
├── hooks/                   # Git hooks (NEW)
│   ├── pre-commit.sh        # Pre-commit checks
│   └── post-test.sh         # Post-test metrics
├── skills/                  # Custom skills
├── agents/                  # Custom agents
├── metrics/                 # Generated metrics (NEW)
│   ├── coverage/            # HTML coverage reports
│   ├── test-metrics-*.json  # Daily test metrics
│   └── trends.csv           # Historical trends
└── settings.local.json      # Local overrides
```

---

## Configuration Files

### config.json

Main configuration file defining:

- **Project metadata** (name, version, platforms)
- **Custom prompts** and their triggers
- **Git hooks** configuration
- **Agent roles** (architect, implementer, reviewer, tester)
- **Coding rules** enforcement
- **Linting configuration** (clippy, rustfmt)
- **Testing requirements** (coverage targets, CI checks)
- **Performance targets**
- **Documentation standards**

**Location:** `D:\dev\agent-game-engine\.claude\config.json`

**Key sections:**
```json
{
  "project": { ... },
  "prompts": { ... },
  "hooks": { ... },
  "agents": { ... },
  "rules": { ... },
  "lints": { ... },
  "testing": { ... }
}
```

---

## Custom Prompts

### Phase Guide (`prompts/phase-guide.md`)

Phase-aware development assistant that:

- Detects current phase from ROADMAP.md
- Loads relevant task files and documentation
- Provides step-by-step implementation guidance
- Enforces coding standards
- Tracks progress and suggests next steps

**Trigger keywords:** `phase`, `roadmap`, `task`, `next step`

**Example usage:**
```
User: "What should I work on next?"
Claude: [Uses phase-guide to identify current phase and suggest next task]
```

**Features:**
- Automatic phase detection
- Context-aware documentation loading
- Task-by-task guidance
- Progress tracking
- Next step suggestions

---

### Code Review Prompt (`prompts/code-review.md`)

Comprehensive code review checklist that:

- Checks for forbidden patterns (println!, unwrap!, etc.)
- Verifies documentation completeness
- Reviews test coverage
- Checks performance implications
- Validates security practices
- Ensures architecture patterns are followed

**Trigger keywords:** `review`, `check code`, `verify standards`

**Example usage:**
```
User: "Review my changes"
Claude: [Runs through comprehensive checklist, provides detailed feedback]
```

**Critical checks:**
1. No forbidden functions (println!, dbg!)
2. No unwrap/expect/panic
3. Custom error types only
4. No unsafe outside FFI
5. Platform abstraction used
6. Complete documentation
7. Test coverage adequate

---

## Git Hooks

### Pre-Commit Hook (`hooks/pre-commit.sh`)

**Runs before each commit to ensure code quality.**

Checks:
1. ✅ No forbidden patterns (println!, dbg!, etc.)
2. ✅ Code formatting (cargo fmt --check)
3. ✅ Clippy lints (cargo clippy -- -D warnings)
4. ⚠️ TODO/FIXME markers (warning only)
5. ⚠️ Large files (warning only)
6. ⚠️ Potential sensitive data (warning only)

**Blocking:** Yes (unless bypassed with `git commit --no-verify`)

**Setup:**
```bash
# Enable the hook (run from project root)
ln -s ../../.claude/hooks/pre-commit.sh .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

**Output:**
```
🔍 Running pre-commit checks...
📋 Checking for forbidden patterns...
✓ No forbidden patterns found
🎨 Checking code formatting (cargo fmt)...
✓ Code is formatted correctly
🔧 Running cargo clippy...
✓ Clippy checks passed
========================================
✓ All pre-commit checks passed!
```

---

### Post-Test Hook (`hooks/post-test.sh`)

**Runs after tests to collect metrics and generate reports.**

Actions:
1. 📊 Collects test metrics (total, passed, failed, pass rate)
2. 📈 Generates coverage report (if cargo-tarpaulin is installed)
3. ⚡ Checks for benchmark results
4. 💾 Updates metrics files (.claude/metrics/)
5. 📉 Updates historical trends
6. 🎯 Checks against targets (80% coverage, 100% pass rate)
7. 💡 Provides recommendations

**Blocking:** No (informational only)

**Setup:**
```bash
# Enable the hook (run from project root)
ln -s ../../.claude/hooks/post-test.sh .git/hooks/post-test
chmod +x .git/hooks/post-test

# Install cargo-tarpaulin for coverage (optional)
cargo install cargo-tarpaulin
```

**Output:**
```
📊 Running post-test analysis...
🧪 Collecting test metrics...
  Total tests: 125
  Passed: 125
  Failed: 0
  Pass rate: 100.00%
📈 Generating coverage report...
  Coverage: 85.32%
📋 Test Summary
========================================
✓ Coverage target met (80%)
✓ All tests passing
```

**Generated files:**
- `.claude/metrics/test-metrics-YYYY-MM-DD.json` - Daily metrics
- `.claude/metrics/coverage/index.html` - HTML coverage report
- `.claude/metrics/trends.csv` - Historical trends

---

## Agent Configurations

The config.json defines four specialized agent roles:

### Architect Agent

**Role:** System architecture and design decisions

**Context:**
- docs/architecture.md
- docs/platform-abstraction.md
- ROADMAP.md

**Expertise:** ECS design, networking, rendering, cross-platform

**Constraints:**
- Must maintain cross-platform compatibility
- No unsafe code except FFI
- Server-authoritative architecture required

**Use when:** Making architectural decisions, designing new systems

---

### Implementer Agent

**Role:** Code implementation following task specifications

**Context:**
- docs/rules/coding-standards.md
- docs/testing-strategy.md
- docs/tasks/*.md

**Expertise:** Rust, Vulkan, networking, ECS

**Constraints:**
- Follow coding-standards.md strictly
- 100% test coverage for new code
- No println!/dbg!/eprintln!
- All public APIs must be documented

**Use when:** Implementing features, writing code

---

### Reviewer Agent

**Role:** Code review and quality assurance

**Context:**
- docs/rules/coding-standards.md
- docs/performance-targets.md
- .claude/prompts/code-review.md

**Expertise:** Code review, performance, security

**Constraints:**
- Check for anti-patterns
- Verify test coverage
- Validate error handling
- Ensure documentation completeness

**Use when:** Reviewing PRs, checking code quality

---

### Tester Agent

**Role:** Test strategy and implementation

**Context:**
- docs/testing-strategy.md
- docs/performance-targets.md

**Expertise:** Unit testing, integration testing, benchmarking

**Constraints:**
- Property-based tests for serialization
- Platform-specific integration tests
- Performance regression tests

**Use when:** Writing tests, improving coverage

---

## Quick Start

### Initialize Claude Code

```bash
# Start Claude Code in the project root
cd D:\dev\agent-game-engine
claude

# Or specify a specific model
claude --model sonnet
```

### Your First Skill

Skills are invoked automatically when relevant or manually with `/skill-name`:

```bash
# List available skills
What skills are available?

# Invoke a skill manually
/review-code src/ecs/world.rs

# Let Claude choose when to use skills
Review the recent changes for code quality issues
```

### Your First Agent

Agents handle specialized tasks in isolated contexts:

```bash
# Use the Explore agent for codebase research
Use the Explore agent to find all ECS component definitions

# Use a custom agent (if defined)
Use the test-runner agent to fix failing tests
```

### Your First Hook

Hooks automate workflows at lifecycle points:

```bash
# View configured hooks
/hooks

# Add a hook via the interactive menu
/hooks
# Select "Create new hook" and follow prompts
```

---

## Available Skills

### How Skills Work

Skills are stored in `.claude/skills/` and consist of:
- `SKILL.md` - Main instructions with YAML frontmatter
- Optional supporting files (templates, scripts, examples)

Skills load automatically based on their `description` field or can be invoked directly with `/skill-name`.

### Skill Locations

| Location | Scope | When to Use |
|----------|-------|-------------|
| `~/.claude/skills/` | All your projects | Personal workflows you use everywhere |
| `.claude/skills/` | This project only | Project-specific workflows (commit to git) |
| Plugins | Where enabled | Share across team via Claude Code plugins |

### Creating Skills

Create skills interactively or manually:

**Interactive (Recommended)**:
```bash
# Use the /skills menu
What skills are available?
# Follow prompts to create new skill
```

**Manual**:
```bash
mkdir -p .claude/skills/my-skill
cat > .claude/skills/my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: What this skill does and when to use it
---

# My Skill

Instructions for Claude to follow when this skill is invoked.

## Usage
Step-by-step guidance here.
EOF
```

### Skill Examples

#### Code Review Skill

`.claude/skills/review-code/SKILL.md`:
```yaml
---
name: review-code
description: Review code for quality, security, and best practices. Use after code changes or when explicitly asked.
disable-model-invocation: false
---

# Code Review Skill

When reviewing code, check for:

1. **Code Quality**
   - Clear, descriptive names
   - No duplicated logic
   - Proper error handling
   - Following project coding standards (see CLAUDE.md)

2. **Security**
   - No exposed secrets or API keys
   - Proper input validation
   - Safe error messages (no sensitive data leaks)

3. **Performance**
   - Efficient algorithms
   - Appropriate data structures
   - No obvious bottlenecks

4. **Testing**
   - Unit tests for new functions
   - Integration tests for features
   - Edge cases covered

## Output Format

Organize feedback by priority:
- **Critical** (must fix): Security issues, bugs
- **Important** (should fix): Code quality, performance
- **Suggestions** (consider): Style improvements, refactoring

Include specific file paths and line numbers.
```

#### Git Commit Skill

`.claude/skills/commit/SKILL.md`:
```yaml
---
name: commit
description: Create well-formatted git commits following project conventions
disable-model-invocation: true
---

# Git Commit Skill

Create commits following this project's standards:

1. Run `git status` to see changes
2. Run `git diff` to review modifications
3. Draft commit message:
   - Subject: < 70 chars, imperative mood ("Add", "Fix", "Update")
   - Body: Explain WHY, not what (the diff shows what)
   - Reference issues if applicable

4. Stage relevant files (avoid `git add -A`)
5. Create commit with:
   ```
   Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
   ```

## Commit Message Template

```
<type>: <short summary>

<detailed explanation of changes>
<why these changes were needed>
<any breaking changes or migration notes>

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

**Types**: feat, fix, docs, refactor, test, chore
```

---

## Using Agents

### Built-in Agents

Claude Code provides three built-in agents:

#### Explore Agent
- **Purpose**: Fast codebase exploration and analysis
- **Model**: Haiku (fast, low-cost)
- **Tools**: Read-only (Read, Grep, Glob)
- **When to Use**: Finding code, understanding structure, researching patterns

**Example**:
```
Use the Explore agent to find all physics-related components
```

#### Plan Agent
- **Purpose**: Research during plan mode
- **Model**: Inherits from main conversation
- **Tools**: Read-only
- **When to Use**: Automatically used in plan mode

**Example**:
```
# Enable plan mode
/plan

# Claude uses Plan agent automatically for research
Implement a new ECS system for health regeneration
```

#### General-Purpose Agent
- **Purpose**: Complex multi-step tasks
- **Model**: Inherits from main conversation
- **Tools**: All tools
- **When to Use**: Tasks requiring both exploration and modification

### Custom Agents

Create project-specific agents in `.claude/agents/`:

**Example: Test Runner Agent**

`.claude/agents/test-runner/AGENT.md`:
```yaml
---
name: test-runner
description: Run tests and fix failures. Use proactively after code changes.
tools: Read, Bash, Edit, Grep
model: sonnet
---

You are a test runner specialist.

When invoked:
1. Run the full test suite with `cargo test --all-features`
2. Identify failing tests
3. Read the test code and implementation
4. Fix issues
5. Re-run tests to verify
6. Report results

For each failure:
- Show the error message
- Identify root cause
- Propose minimal fix
- Update code
- Verify fix works

Focus on fixing the underlying issue, not symptoms.
```

**Usage**:
```
Use the test-runner agent to fix the failing ECS tests
```

### Agent Invocation

**Automatic**: Claude delegates based on agent descriptions
```
Find all rendering-related code
# Claude may use Explore agent automatically
```

**Explicit**: Request specific agent
```
Use the Explore agent to research the networking module
Have the test-runner agent fix the physics tests
```

**Background**: Long-running tasks
```
Run tests in the background while I continue working
# Press Ctrl+B to background a running task
```

---

## Workflow Templates

Complete step-by-step workflow templates are available in `.claude/workflows/`:

### Available Workflows

1. **[new-component.md](workflows/new-component.md)** - Add new ECS component
   - Define component struct
   - Add to ComponentData enum
   - Register in World
   - Write tests and documentation

2. **[new-system.md](workflows/new-system.md)** - Add new ECS system
   - Define system function
   - Handle client/server split
   - Register and configure execution order
   - Add profiling and benchmarks

3. **[new-phase-task.md](workflows/new-phase-task.md)** - Start new phase task
   - Read task file and dependencies
   - Create work branch and checklist
   - Track progress
   - Handle blockers

4. **[debug-issue.md](workflows/debug-issue.md)** - Debug issues
   - Reproduce and diagnose problems
   - Enable verbose logging and profiling
   - Use debugger and sanitizers
   - Document findings and implement fix

5. **[pr-workflow.md](workflows/pr-workflow.md)** - Create pull request
   - Run all pre-PR checks
   - Create comprehensive PR description
   - Handle reviews and CI failures
   - Merge and clean up

See **[workflows/README.md](workflows/README.md)** for complete documentation.

### Development Workflow

**1. Feature Development**
```
# Research phase (uses Explore agent)
Explore the ECS module to understand how systems work

# Planning
/plan
Implement a new system for health regeneration

# Implementation (using workflows)
Follow .claude/workflows/new-system.md

# Testing
Use the test-runner agent to ensure all tests pass

# Review
/review-code on the new health system

# Commit (using workflow)
Follow .claude/workflows/pr-workflow.md
```

**2. Bug Fix Workflow**
```
# Investigate (using workflow)
Follow .claude/workflows/debug-issue.md

# Fix
Implement the minimal fix

# Verify
Run tests to confirm the fix

# Commit
Follow .claude/workflows/pr-workflow.md
```

**3. Code Review Workflow**
```
# Review recent changes
/review-code

# Check specific file
/review-code src/ecs/systems/health.rs

# Review with git diff
git diff main...HEAD
/review-code the changes in this diff
```

### Research Workflows

**Codebase Exploration**
```
# High-level overview
Use the Explore agent with very thorough mode to map the architecture

# Find specific patterns
Use Explore to find all places where we create entities

# Understand dependencies
Show me the dependency graph between ECS modules
```

**Documentation Research**
```
# Find documentation
Where is the ECS system documented?

# Understand design decisions
Why does the networking module use TCP + UDP?

# Find examples
Show me examples of custom ECS components
```

### Testing Workflows

**Run Tests**
```
# All tests
cargo test --all-features

# Specific module
cargo test --package agent-game-engine-core

# With output
cargo test -- --nocapture
```

**Fix Failing Tests**
```
Use the test-runner agent to fix all failing tests
```

**Add Test Coverage**
```
/plan
Add comprehensive tests for the new LOD system
```

---

## Hook Configuration

### What are Hooks?

Hooks are shell commands that run automatically at specific lifecycle points:

- **SessionStart**: When Claude Code starts or resumes
- **UserPromptSubmit**: Before processing your input
- **PreToolUse**: Before Claude uses a tool (can block it)
- **PostToolUse**: After successful tool execution
- **Stop**: When Claude finishes responding
- **And more**: See [HOOKS.md](./HOOKS.md) for complete list

### Hook Configuration File

Hooks are defined in `.claude/settings.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/lint-check.sh",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

### Common Hook Examples

#### Auto-format on File Changes

`.claude/settings.json`:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Write|Edit",
        "hooks": [
          {
            "type": "command",
            "command": "cargo fmt --manifest-path \"$CLAUDE_PROJECT_DIR\"/Cargo.toml",
            "statusMessage": "Running cargo fmt..."
          }
        ]
      }
    ]
  }
}
```

#### Block Dangerous Commands

`.claude/hooks/block-rm.sh`:
```bash
#!/bin/bash
INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command')

if echo "$COMMAND" | grep -q 'rm -rf'; then
  echo '{"decision":"block","reason":"Destructive rm -rf blocked"}'
  exit 0
fi

exit 0
```

`.claude/settings.json`:
```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/block-rm.sh"
          }
        ]
      }
    ]
  }
}
```

#### Run Tests After Code Changes

`.claude/hooks/test-on-change.sh`:
```bash
#!/bin/bash
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Only run tests for Rust source files
if [[ "$FILE_PATH" == *.rs ]]; then
  cargo test --quiet 2>&1
  if [ $? -ne 0 ]; then
    echo '{"systemMessage":"Tests failed after editing '"$FILE_PATH"'"}'
  fi
fi

exit 0
```

`.claude/settings.json`:
```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/test-on-change.sh",
            "async": true,
            "timeout": 120
          }
        ]
      }
    ]
  }
}
```

### Managing Hooks

**Interactive Menu**:
```
/hooks
```

**View Hook Output**:
```
# Toggle verbose mode
Ctrl+O
```

**Debug Hooks**:
```
# Start with debug logging
claude --debug
```

---

## Best Practices

### Skill Design

1. **Focused Purpose**: Each skill should do one thing well
2. **Clear Descriptions**: Help Claude know when to use it
3. **Supporting Files**: Keep SKILL.md under 500 lines, move details to separate files
4. **Examples**: Include concrete examples in your skills

**Good Skill Structure**:
```
my-skill/
├── SKILL.md           # Overview and workflow (< 500 lines)
├── REFERENCE.md       # Detailed API documentation
├── EXAMPLES.md        # Usage examples
└── scripts/
    └── helper.sh      # Utility scripts
```

### Agent Design

1. **Single Responsibility**: Each agent specializes in one type of task
2. **Tool Restrictions**: Grant only necessary permissions
3. **Model Selection**: Use Haiku for speed, Sonnet for capability
4. **Detailed Prompts**: Specify exactly what the agent should do

**Example Agent Configuration**:
```yaml
---
name: security-scanner
description: Scan code for security vulnerabilities. Use proactively before commits.
tools: Read, Grep, Glob
model: sonnet
permissionMode: plan
---

You scan for security issues:
- SQL injection vulnerabilities
- XSS vulnerabilities
- Exposed secrets
- Unsafe deserialization

Report findings with severity and remediation.
```

### Hook Design

1. **Keep Hooks Fast**: Session hooks run on every start
2. **Handle Errors**: Exit codes matter (0 = success, 2 = block)
3. **Quote Variables**: Always use `"$VAR"` not `$VAR`
4. **Use Async for Slow Operations**: Don't block Claude for long tasks

**Hook Template**:
```bash
#!/bin/bash
set -e

# Read input
INPUT=$(cat)
FIELD=$(echo "$INPUT" | jq -r '.some_field // empty')

# Validate
if [ -z "$FIELD" ]; then
  exit 0
fi

# Perform check
if some_condition; then
  echo '{"decision":"block","reason":"Reason here"}'
fi

exit 0
```

### Working with Claude

1. **Be Specific**: "Review src/ecs/world.rs" vs "review code"
2. **Use Plan Mode**: `/plan` for complex tasks
3. **Leverage Agents**: Delegate specialized work
4. **Check Context**: `/context` shows what Claude can see
5. **Compact When Needed**: `/compact` when context is full

### Project Organization

**Recommended `.claude/` Structure**:
```
.claude/
├── README.md              # This file
├── SKILLS.md              # Skill documentation
├── AGENTS.md              # Agent documentation
├── settings.json          # Hooks and project settings
├── settings.local.json    # Local overrides (gitignored)
├── skills/
│   ├── review-code/
│   │   └── SKILL.md
│   ├── commit/
│   │   └── SKILL.md
│   └── test-runner/
│       └── SKILL.md
├── agents/
│   ├── test-runner/
│   │   └── AGENT.md
│   └── security-scanner/
│       └── AGENT.md
└── hooks/
    ├── block-rm.sh
    ├── lint-check.sh
    └── test-on-change.sh
```

**What to Commit**:
- ✅ `.claude/skills/` - Share workflows with team
- ✅ `.claude/agents/` - Share specialized agents
- ✅ `.claude/settings.json` - Share hooks and config
- ✅ `.claude/hooks/` - Share hook scripts
- ❌ `.claude/settings.local.json` - Personal overrides

---

## Troubleshooting

### Skills Not Loading

**Symptom**: Claude doesn't use your skill

**Solutions**:
1. Check description is clear and relevant
2. Verify YAML frontmatter is valid
3. Restart Claude Code session
4. Check skill appears in "What skills are available?"

**Debug**:
```
# List all skills
What skills are available?

# Invoke skill directly
/skill-name

# Check logs
claude --debug
```

### Hooks Not Firing

**Symptom**: Hook command not executing

**Solutions**:
1. Verify hook is in correct settings file
2. Check matcher pattern matches tool name
3. Ensure script is executable (`chmod +x`)
4. Test script independently

**Debug**:
```
# View hooks
/hooks

# Enable debug mode
claude --debug

# Check verbose output
Ctrl+O
```

### Agent Not Delegating

**Symptom**: Claude doesn't use subagent

**Solutions**:
1. Be explicit: "Use the [agent-name] agent to..."
2. Check agent description matches task
3. Verify agent file is in `.claude/agents/`
4. Restart session to load new agents

### Context Window Full

**Symptom**: "Context window is full" error

**Solutions**:
```
# Compact conversation
/compact

# Or compact with custom instructions
/compact Keep all information about the ECS architecture

# Use subagents for research
Use the Explore agent to research X
# (keeps verbose output out of main context)
```

### Permissions Issues

**Symptom**: Hook script can't read files / access resources

**Solutions**:
1. Check script has execute permissions
2. Use absolute paths: `"$CLAUDE_PROJECT_DIR"/script.sh`
3. Verify current working directory in hook input
4. Check `cwd` field in hook JSON input

### JSON Parsing Errors in Hooks

**Symptom**: "JSON validation failed"

**Solutions**:
1. Ensure stdout contains only JSON (no debug prints)
2. Validate JSON with `jq`: `echo "$OUTPUT" | jq .`
3. Check shell profile doesn't print on startup
4. Use `>&2` for debug output (goes to stderr)

**Example**:
```bash
# Bad
echo "Debug info"
echo '{"decision":"allow"}'

# Good
echo "Debug info" >&2
echo '{"decision":"allow"}'
```

### Getting Help

1. **Built-in Help**: `/help` for commands
2. **Documentation**: Review [SKILLS.md](./SKILLS.md), [AGENTS.md](./AGENTS.md)
3. **Project Docs**: See [CLAUDE.md](../CLAUDE.md) for project-specific rules
4. **Official Docs**: https://code.claude.com/docs

---

## Additional Resources

- **[SKILLS.md](./SKILLS.md)**: Detailed skill documentation with examples
- **[AGENTS.md](./AGENTS.md)**: Complete agent configuration guide
- **[../CLAUDE.md](../CLAUDE.md)**: Project-specific AI development rules
- **[../ROADMAP.md](../ROADMAP.md)**: Project implementation plan

### External Documentation

- [Claude Code Official Docs](https://code.claude.com/docs)
- [Agent Skills Guide](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview)
- [Skills Repository](https://github.com/anthropics/skills)
- [Claude Code Examples](https://github.com/anthropics/claude-code/tree/main/examples)

---

**Last Updated**: 2026-02-01
