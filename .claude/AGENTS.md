# Claude Code Agents Reference

> **Complete guide to using and creating agents in the Silmaril project**

---

## Table of Contents

- [Overview](#overview)
- [Built-in Agents](#built-in-agents)
- [Creating Custom Agents](#creating-custom-agents)
- [Agent Configuration](#agent-configuration)
- [Working with Agents](#working-with-agents)
- [Agent Examples](#agent-examples)
- [Advanced Patterns](#advanced-patterns)
- [Best Practices](#best-practices)

---

## Overview

Agents (subagents) are specialized AI assistants that handle specific types of tasks in isolated contexts. Each agent runs with:

- **Custom system prompt**: Focused instructions for the task
- **Specific tool access**: Only the tools needed
- **Independent context**: Separate from main conversation
- **Permission modes**: Different approval requirements

### Why Use Agents?

**Preserve Context**: Keep exploration and research out of main conversation

**Enforce Constraints**: Limit tools to read-only, specific commands, etc.

**Reuse Configurations**: Share agents across projects via user-level configs

**Specialize Behavior**: Domain-specific prompts and workflows

**Control Costs**: Route simple tasks to faster, cheaper models

### Agents vs Skills

| Feature | Agents | Skills |
|---------|--------|--------|
| **Context** | Isolated (separate context window) | Inline (main conversation) |
| **Tools** | Configurable per agent | Uses main conversation tools |
| **Duration** | Multi-turn, can be resumed | Single invocation |
| **Use Case** | Complex research, isolated tasks | Workflows, reference knowledge |

**Use Agents When**:
- Task produces verbose output
- Need tool restrictions
- Work is self-contained
- Want to enforce permissions

**Use Skills When**:
- Need conversation context
- Reusable workflows
- Domain knowledge/patterns
- Inline guidance

---

## Built-in Agents

### Explore Agent

**Purpose**: Fast, read-only codebase exploration

**Configuration**:
- **Model**: Haiku (fast, low-cost)
- **Tools**: Read, Grep, Glob (read-only)
- **Permission Mode**: Default

**When Claude Uses It**:
- Finding code patterns
- Understanding structure
- Researching implementations
- Mapping dependencies

**Thoroughness Levels**:
- **Quick**: Targeted lookups
- **Medium**: Balanced exploration
- **Very thorough**: Comprehensive analysis

**Example**:
```
Use the Explore agent to find all ECS component definitions

# Or let Claude choose
Find all physics-related code
```

**Output**: Summary of findings returned to main conversation

### Plan Agent

**Purpose**: Research during plan mode

**Configuration**:
- **Model**: Inherits from main conversation
- **Tools**: Read, Grep, Glob (read-only)
- **Permission Mode**: Plan mode

**When Claude Uses It**:
- Automatically in `/plan` mode
- Gathering context for planning
- Understanding codebase before proposing changes

**Example**:
```
/plan
Implement a new LOD system for network optimization

# Plan agent automatically researches existing LOD code
```

**Output**: Research used to create detailed plan

### General-Purpose Agent

**Purpose**: Complex multi-step tasks requiring both exploration and action

**Configuration**:
- **Model**: Inherits from main conversation
- **Tools**: All tools (Read, Write, Edit, Bash, etc.)
- **Permission Mode**: Default

**When Claude Uses It**:
- Tasks needing exploration + modification
- Complex reasoning over results
- Multi-step dependent operations

**Example**:
```
Use a general-purpose agent to refactor the ECS query system
```

**Output**: Complete task results

---

## Creating Custom Agents

### Method 1: Interactive (`/agents` Menu)

```
/agents

# Select "Create new agent"
# Choose "User-level" or "Project-level"
# Describe the agent or generate with Claude
# Select tools and model
# Choose color
# Save
```

**Recommended**: Easiest way to create well-configured agents

### Method 2: Manual Creation

Create agent file in `.claude/agents/`:

```bash
mkdir -p .claude/agents/test-runner
cat > .claude/agents/test-runner/AGENT.md << 'EOF'
---
name: test-runner
description: Run tests and fix failures. Use proactively after code changes.
tools: Read, Bash, Edit, Grep
model: sonnet
---

You are a test runner specialist.

When invoked:
1. Run test suite
2. Identify failures
3. Fix issues
4. Verify fixes

Report results clearly.
EOF
```

### Agent Locations

| Location | Scope | When to Use |
|----------|-------|-------------|
| `--agents` CLI flag | Current session only | Testing, one-off configs |
| `.claude/agents/` | This project | Project-specific (commit to git) |
| `~/.claude/agents/` | All your projects | Personal agents |
| Plugin `agents/` | Where plugin enabled | Share via plugins |

**Priority**: CLI > Project > User > Plugin

### CLI-Defined Agents

Pass agent as JSON when launching Claude Code:

```bash
claude --agents '{
  "code-reviewer": {
    "description": "Expert code reviewer. Use proactively after changes.",
    "prompt": "You are a senior code reviewer focusing on quality and security.",
    "tools": ["Read", "Grep", "Glob"],
    "model": "sonnet"
  }
}'
```

**Use Case**: Automation scripts, testing configurations

---

## Agent Configuration

### AGENT.md Structure

```yaml
---
name: agent-name
description: When Claude should delegate to this agent
tools: Read, Grep, Bash
disallowedTools: Write, Edit
model: sonnet
permissionMode: default
skills:
  - api-conventions
  - error-handling
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "./scripts/validate.sh"
---

# Agent System Prompt

You are a [specialist type].

When invoked:
1. Step one
2. Step two
3. Step three

## Guidelines

- Guideline 1
- Guideline 2

## Output Format

How to present results.
```

### Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | Yes | Unique identifier (lowercase, hyphens) |
| `description` | Yes | When Claude should delegate to this agent |
| `tools` | No | Allowed tools (inherits all if omitted) |
| `disallowedTools` | No | Tools to deny (removed from allowed list) |
| `model` | No | `sonnet`, `opus`, `haiku`, or `inherit` (default) |
| `permissionMode` | No | `default`, `acceptEdits`, `dontAsk`, `bypassPermissions`, `plan` |
| `skills` | No | Skills to preload into agent context |
| `hooks` | No | Lifecycle hooks for this agent |

### Tool Configuration

**Allow Specific Tools**:
```yaml
tools: Read, Grep, Glob, Bash
```

**Allow All Except**:
```yaml
# Omit tools field (inherits all)
disallowedTools: Write, Edit
```

**Read-Only Agent**:
```yaml
tools: Read, Grep, Glob
```

**Full Access**:
```yaml
# Omit both tools and disallowedTools
```

### Model Selection

| Model | Speed | Capability | Cost | Use Case |
|-------|-------|------------|------|----------|
| `haiku` | Fastest | Basic | Lowest | Simple searches, read-only |
| `sonnet` | Fast | High | Medium | Most tasks, balanced |
| `opus` | Slower | Highest | Highest | Complex reasoning |
| `inherit` | - | - | - | Same as main conversation |

**Recommendations**:
- **Explore-like agents**: Use `haiku`
- **Code review, debugging**: Use `sonnet`
- **Complex refactoring**: Use `opus` or `inherit`

### Permission Modes

| Mode | Behavior | Use Case |
|------|----------|----------|
| `default` | Standard permission prompts | Normal operations |
| `acceptEdits` | Auto-accept file edits | Trusted modifications |
| `dontAsk` | Auto-deny (only pre-allowed tools work) | Restricted operations |
| `bypassPermissions` | Skip all permission checks | Fully automated (use with caution) |
| `plan` | Read-only exploration | Research without changes |

**Warning**: `bypassPermissions` skips all checks. Use only for trusted, well-tested agents.

### Preloading Skills

Inject skill content at agent startup:

```yaml
---
name: api-developer
description: Implement API endpoints following conventions
skills:
  - api-conventions
  - error-handling-patterns
---

Implement API endpoints using the preloaded conventions.
```

**Effect**: Full skill content loaded into agent context immediately, not just available for invocation.

---

## Working with Agents

### Invoking Agents

**Automatic Delegation**:
```
Find all rendering-related code
# Claude may choose Explore agent based on description
```

**Explicit Request**:
```
Use the Explore agent to research the physics module
Have the test-runner agent fix the failing tests
```

**With Arguments**:
```
Use code-reviewer to review src/ecs/world.rs
```

### Foreground vs Background

**Foreground** (default):
- Blocks main conversation
- Can ask clarifying questions
- Permission prompts pass through
- Results return when complete

**Background**:
- Runs concurrently
- Pre-approves permissions upfront
- Auto-denies clarifying questions
- Results return on next turn

**Enable Background**:
```
Run tests in the background while I continue working

# Or press Ctrl+B during execution
```

**Disable Background**:
```bash
# Set environment variable
export CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1
```

### Resuming Agents

Each agent invocation creates fresh context. To continue previous work:

```
Use the code-reviewer agent to review auth module
[Agent completes]

Continue that review and now check authorization logic
[Claude resumes same agent with full history]
```

**Agent IDs**: Found in `~/.claude/projects/{project}/{sessionId}/subagents/agent-{id}.jsonl`

**Persistence**:
- Survives main conversation compaction
- Persists within session (even after restart via `/resume`)
- Cleaned up after `cleanupPeriodDays` (default: 30)

### Context Management

**Auto-Compaction**:
- Triggers at ~95% capacity (configurable via `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`)
- Logged in agent transcript

**Transcript Location**:
```
~/.claude/projects/{project-id}/{session-id}/subagents/agent-{agent-id}.jsonl
```

---

## Agent Examples

### Code Reviewer Agent

**Purpose**: Review code without modifying it

`.claude/agents/code-reviewer/AGENT.md`:
```yaml
---
name: code-reviewer
description: Expert code review specialist. Use proactively after writing or modifying code.
tools: Read, Grep, Glob, Bash
model: inherit
---

You are a senior code reviewer ensuring high standards.

When invoked:
1. Run `git diff` to see recent changes
2. Focus on modified files
3. Begin review immediately

Review checklist:
- Code is clear and readable
- Functions and variables are well-named
- No duplicated code
- Proper error handling
- No exposed secrets or API keys
- Input validation implemented
- Good test coverage
- Performance considerations addressed

Provide feedback organized by priority:

## Critical Issues (Must Fix)
List security vulnerabilities, bugs, violations of MANDATORY rules.

## Warnings (Should Fix)
List code quality issues, missing tests, performance problems.

## Suggestions (Consider)
List style improvements, refactoring opportunities.

Include specific examples of how to fix issues.
```

**Usage**:
```
Use code-reviewer after I make changes
/code-reviewer
```

### Test Runner Agent

**Purpose**: Run tests and fix failures

`.claude/agents/test-runner/AGENT.md`:
```yaml
---
name: test-runner
description: Test execution and debugging specialist. Use after code changes or when tests fail.
tools: Read, Edit, Bash, Grep, Glob
model: sonnet
---

You are an expert at running tests and fixing failures.

When invoked:
1. Run full test suite: `cargo test --all-features`
2. Identify failing tests
3. For each failure:
   a. Read test code
   b. Read implementation
   c. Diagnose root cause
   d. Implement minimal fix
   e. Re-run test to verify
4. Run full suite again
5. Report results

Debugging process:
- Analyze error messages and stack traces
- Check recent code changes
- Form and test hypotheses
- Add strategic logging if needed
- Fix the underlying issue, not symptoms

For each fixed test:
- Explain the root cause
- Show the fix
- Confirm it now passes

For remaining failures:
- Explain what was tried
- Suggest next steps
```

**Usage**:
```
/test-runner
Use test-runner to fix the ECS tests
```

### Database Query Agent

**Purpose**: Read-only database access with validation

`.claude/agents/db-reader/AGENT.md`:
```yaml
---
name: db-reader
description: Execute read-only database queries for analysis and reporting
tools: Bash
hooks:
  PreToolUse:
    - matcher: "Bash"
      hooks:
        - type: command
          command: "$CLAUDE_PROJECT_DIR/.claude/hooks/validate-readonly-query.sh"
---

You are a database analyst with read-only access.

When asked to analyze data:
1. Identify which tables contain relevant data
2. Write efficient SELECT queries with appropriate filters
3. Present results clearly with context

You can only execute SELECT queries. If asked to INSERT, UPDATE, DELETE, or modify schema, explain that you only have read access and suggest who to contact for write operations.

Format results as tables with:
- Column headers
- Row data
- Summary statistics where relevant
```

**Validation Script** - `.claude/hooks/validate-readonly-query.sh`:
```bash
#!/bin/bash
INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty')

if [ -z "$COMMAND" ]; then
  exit 0
fi

# Block write operations
if echo "$COMMAND" | grep -iE '\b(INSERT|UPDATE|DELETE|DROP|CREATE|ALTER|TRUNCATE)\b' > /dev/null; then
  echo "Blocked: Write operations not allowed. Use SELECT queries only." >&2
  exit 2
fi

exit 0
```

**Usage**:
```
/db-reader
Use db-reader to analyze user signup trends
```

### Debugger Agent

**Purpose**: Systematic debugging workflow

`.claude/agents/debugger/AGENT.md`:
```yaml
---
name: debugger
description: Debugging specialist for errors, test failures, and unexpected behavior
tools: Read, Edit, Bash, Grep, Glob
model: sonnet
---

You are an expert debugger specializing in root cause analysis.

When invoked:
1. Capture error message and stack trace
2. Identify reproduction steps
3. Isolate the failure location
4. Implement minimal fix
5. Verify solution works

Debugging process:
- Analyze error messages and logs
- Check recent code changes with `git log` and `git diff`
- Form and test hypotheses
- Add strategic debug logging with tracing macros
- Inspect variable states

For each issue, provide:
- Root cause explanation
- Evidence supporting diagnosis
- Specific code fix
- Testing approach
- Prevention recommendations

Focus on fixing the underlying issue, not the symptoms.

Use tracing for debug output:
```rust
use tracing::debug;
debug!(value = ?x, "Checkpoint");
```

Enable trace logs:
```bash
RUST_LOG=trace cargo test test_name -- --nocapture
```
```

**Usage**:
```
/debugger
Debug why health regeneration crashes
```

### Data Scientist Agent

**Purpose**: Data analysis and SQL queries

`.claude/agents/data-scientist/AGENT.md`:
```yaml
---
name: data-scientist
description: Data analysis expert for SQL queries, BigQuery operations, and data insights
tools: Bash, Read, Write
model: sonnet
---

You are a data scientist specializing in SQL and BigQuery analysis.

When invoked:
1. Understand the data analysis requirement
2. Write efficient SQL queries
3. Use BigQuery CLI tools (bq) when appropriate
4. Analyze and summarize results
5. Present findings clearly

Key practices:
- Write optimized SQL with proper filters
- Use appropriate aggregations and joins
- Include comments explaining complex logic
- Format results for readability
- Provide data-driven recommendations

For each analysis:
- Explain the query approach
- Document any assumptions
- Highlight key findings
- Suggest next steps based on data

Always ensure queries are efficient and cost-effective.

Example BigQuery command:
```bash
bq query --use_legacy_sql=false 'SELECT...'
```
```

**Usage**:
```
/data-scientist
Analyze user engagement metrics for last quarter
```

### Security Scanner Agent

**Purpose**: Security vulnerability scanning

`.claude/agents/security-scanner/AGENT.md`:
```yaml
---
name: security-scanner
description: Security vulnerability scanner. Use before commits and releases.
tools: Read, Grep, Glob
model: sonnet
permissionMode: plan
---

You are a security specialist scanning for vulnerabilities.

When invoked:
1. Scan codebase for security issues
2. Categorize by severity
3. Provide remediation guidance

Check for:

## Critical Vulnerabilities
- SQL injection points
- XSS vulnerabilities
- Command injection
- Path traversal
- Deserialization vulnerabilities
- Hardcoded secrets (API keys, passwords)

## Important Issues
- Missing input validation
- Insecure authentication
- Insufficient authorization checks
- Sensitive data in logs
- Insecure dependencies

## Recommendations
- Security best practices
- Defense in depth
- Secure defaults

For each finding:
- **Severity**: Critical / High / Medium / Low
- **Location**: File and line number
- **Description**: What the vulnerability is
- **Impact**: What could happen
- **Remediation**: How to fix it
- **Example**: Secure code pattern
```

**Usage**:
```
/security-scanner
Scan the authentication module for security issues
```

---

## Advanced Patterns

### Conditional Tool Access with Hooks

Allow some operations, block others using hooks:

```yaml
---
name: safe-modifier
description: Modify files with safety checks
tools: Bash, Read, Edit
hooks:
  PreToolUse:
    - matcher: "Edit"
      hooks:
        - type: command
          command: "./scripts/validate-edit.sh"
---
```

**Validation Script**:
```bash
#!/bin/bash
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path')

# Block editing critical files
if [[ "$FILE_PATH" == *".env"* ]] || [[ "$FILE_PATH" == *"secrets"* ]]; then
  echo "Blocked: Cannot edit sensitive files" >&2
  exit 2
fi

exit 0
```

### Skills in Agents

Preload domain knowledge:

```yaml
---
name: api-implementer
description: Implement API endpoints following conventions
skills:
  - api-conventions
  - rest-patterns
  - error-handling
tools: Read, Write, Edit, Bash
---

Implement API endpoints using preloaded conventions.

Follow the patterns from api-conventions skill.
Handle errors according to error-handling skill.
```

**Effect**: All three skills fully loaded at agent startup

### Chain Agents

Sequential agent workflow:

```
Use code-reviewer to find performance issues
[Agent completes, returns findings]

Use optimizer agent to fix the performance issues identified
```

**Each agent**:
1. Gets task from Claude
2. Works independently
3. Returns results
4. Claude decides next step

### Parallel Agents

Independent research:

```
Research the authentication, database, and API modules in parallel using separate agents
```

**Warning**: Each agent's detailed results return to main conversation. Many concurrent agents = context consumption.

---

## Best Practices

### Agent Design Principles

1. **Single Responsibility**: Each agent excels at one type of task
2. **Clear Descriptions**: Help Claude know when to delegate
3. **Minimal Tools**: Grant only what's needed
4. **Detailed Prompts**: Specify workflows and output formats
5. **Model Selection**: Balance speed vs capability

**Good Agent**:
```yaml
---
name: test-fixer
description: Fix failing unit tests. Use after test failures.
tools: Read, Edit, Bash, Grep
model: sonnet
---

Focused instructions for fixing tests.
Clear step-by-step process.
Specific output format.
```

**Bad Agent**:
```yaml
---
name: helper
description: Help with various tasks
# All tools (unfocused)
---

Generic instructions.
```

### When to Use Which Agent

| Task | Recommended Agent | Why |
|------|------------------|-----|
| Find code patterns | Explore | Fast, read-only, focused |
| Review code | Custom code-reviewer | Specialized prompt, read-only |
| Fix tests | Custom test-runner | Edit access, testing workflow |
| Debug issues | Custom debugger | Systematic approach, edit access |
| Research architecture | Explore or custom | Depends on depth needed |
| Implement feature | General-purpose or custom | Needs exploration + modification |

### Agent vs Main Conversation

**Use Agent When**:
- ✅ Produces verbose output you don't need in main context
- ✅ Want to enforce tool restrictions
- ✅ Self-contained work with summary result
- ✅ Can specify latency (agent startup takes time)

**Use Main Conversation When**:
- ✅ Need frequent back-and-forth
- ✅ Multiple phases share significant context
- ✅ Quick, targeted change
- ✅ Latency matters

### Managing Agent Context

1. **Resuming**: Continue previous agent's work instead of starting fresh
2. **Compaction**: Agents auto-compact like main conversation
3. **Cleanup**: Transcripts cleaned after `cleanupPeriodDays` setting
4. **Isolation**: Agents don't see each other's context

### Debugging Agents

**View Agent Transcripts**:
```
~/.claude/projects/{project}/{session}/subagents/agent-{id}.jsonl
```

**Check Agent List**:
```
/agents
```

**Debug Delegation**:
```bash
claude --debug

# Shows when Claude delegates to agents
```

**Test Agent Directly**:
```
/agent-name
# Forces invocation for testing
```

---

## Troubleshooting

### Agent Not Delegating

**Symptoms**: Claude doesn't use your agent

**Solutions**:
1. Be explicit: "Use the [agent-name] agent to..."
2. Check description matches task type
3. Verify agent in `/agents` menu
4. Restart session to load new agents

**Example**:
```
# Vague (may not trigger)
Fix the tests

# Explicit (will trigger)
Use the test-runner agent to fix the failing tests
```

### Agent Fails Permissions

**Symptoms**: Agent blocked from using tools

**Solutions**:
1. Check `tools` field includes needed tools
2. Verify `permissionMode` is appropriate
3. For background agents, pre-approve permissions
4. Check hooks aren't blocking tools

**Debug**:
```
/agents
# View agent configuration
# Check tools list
```

### Agent Context Issues

**Symptoms**: Agent doesn't have information it needs

**Solutions**:
1. Check if skills are preloaded (`skills` field)
2. Verify agent can read necessary files (`Read` tool)
3. Consider whether main conversation context would be better
4. Resume agent to continue previous work

### Infinite Agent Loops

**Symptoms**: Agent keeps spawning agents

**Solutions**:
- Agents cannot spawn other agents (by design)
- If happening, likely main conversation delegating repeatedly
- Use `disable-model-invocation: true` in skills to prevent auto-trigger
- Check agent descriptions aren't too broad

---

## Additional Resources

- **[README.md](./README.md)**: General Claude Code setup guide
- **[SKILLS.md](./SKILLS.md)**: Skills documentation
- **[Official Agents Docs](https://code.claude.com/docs/en/sub-agents)**: Complete reference
- **[Hooks Reference](https://code.claude.com/docs/en/hooks)**: Hook configuration

### Related Topics

- **Plugins**: Bundle agents with plugins for team sharing
- **MCP Servers**: Give agents access to external tools/data
- **Headless Mode**: Run agents in CI/CD pipelines

---

**Last Updated**: 2026-02-01
