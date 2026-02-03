# Workflow Templates

> Step-by-step guides for common development scenarios

---

## Overview

This directory contains workflow templates for automating common development tasks in the silmaril project. Each workflow is designed to be followed by both AI agents and human developers.

---

## Available Workflows

### 1. **[new-component.md](new-component.md)** - Add New ECS Component
Complete workflow for adding a new component to the ECS system.

**When to use:**
- Adding gameplay data (Health, Inventory, etc.)
- Adding rendering data (Mesh, Material, etc.)
- Adding physics data (RigidBody, Collider, etc.)
- Adding network-replicated data

**Key steps:**
1. Define component struct
2. Add to ComponentData enum
3. Register in World
4. Write tests
5. Add documentation

**Time estimate:** 30-60 minutes

---

### 2. **[new-system.md](new-system.md)** - Add New ECS System
Complete workflow for adding a new system to the ECS.

**When to use:**
- Adding gameplay logic (combat, AI, etc.)
- Adding rendering logic (culling, LOD, etc.)
- Adding physics logic (movement, forces, etc.)
- Adding network logic (replication, etc.)

**Key steps:**
1. Define system function
2. Write tests
3. Register in App
4. Handle execution order
5. Add profiling

**Time estimate:** 1-2 hours

---

### 3. **[new-phase-task.md](new-phase-task.md)** - Start New Phase Task
Complete workflow for beginning work on a roadmap task.

**When to use:**
- Starting any task from ROADMAP.md
- Beginning a new phase of development
- Picking up assigned work

**Key steps:**
1. Read task file
2. Check dependencies
3. Create work branch
4. Create task checklist
5. Set up environment
6. Implement feature
7. Track progress

**Time estimate:** Varies by task (hours to days)

---

### 4. **[debug-issue.md](debug-issue.md)** - Debug Issue
Comprehensive debugging workflow for troubleshooting problems.

**When to use:**
- Application crashes
- Tests failing
- Performance issues
- Rendering problems
- Network issues
- Memory leaks

**Key steps:**
1. Reproduce issue
2. Enable verbose logging
3. Add instrumentation
4. Use debugger
5. Check memory issues
6. Profile performance
7. Document findings
8. Implement fix

**Time estimate:** 1-4 hours (varies by complexity)

---

### 5. **[pr-workflow.md](pr-workflow.md)** - Pull Request Workflow
Complete automated workflow for creating and managing pull requests.

**When to use:**
- Submitting completed work for review
- Creating feature PRs
- Creating bug fix PRs
- Creating documentation PRs

**Key steps:**
1. Clean working state
2. Update from main
3. Run all checks
4. Review own code
5. Create PR
6. Address reviews
7. Merge

**Time estimate:** 30-60 minutes

---

## Quick Reference

### For New Features
1. Follow [new-phase-task.md](new-phase-task.md) to start work
2. Use [new-component.md](new-component.md) if adding components
3. Use [new-system.md](new-system.md) if adding systems
4. Use [debug-issue.md](debug-issue.md) if encountering issues
5. Use [pr-workflow.md](pr-workflow.md) to submit for review

### For Bug Fixes
1. Follow [debug-issue.md](debug-issue.md) to diagnose
2. Implement fix
3. Use [pr-workflow.md](pr-workflow.md) to submit

### For Documentation
1. Update relevant docs
2. Use [pr-workflow.md](pr-workflow.md) to submit

---

## Workflow Principles

All workflows in this directory follow these principles:

### 1. Step-by-Step
Every workflow breaks down tasks into clear, actionable steps with validation at each stage.

### 2. Comprehensive
Workflows include all necessary context, commands, and examples to complete the task without external references.

### 3. Validated
Each step includes validation commands to verify success before moving forward.

### 4. Error Handling
Common errors and solutions are documented for each workflow.

### 5. Checklist-Driven
Workflows provide checklists to ensure nothing is missed.

### 6. Reference-Rich
Workflows link to relevant documentation for deeper understanding.

---

## How to Use These Workflows

### For AI Agents
1. Select the appropriate workflow for the task
2. Follow steps sequentially
3. Run all validation commands
4. Check all boxes in checklists
5. Handle errors using documented solutions
6. Report any issues or blockers

### For Human Developers
1. Choose the workflow that matches your task
2. Use as a checklist or reference
3. Adapt steps to your specific situation
4. Skip familiar steps if confident
5. Contribute improvements based on experience

---

## Workflow Relationships

```
new-phase-task.md
    ├─> new-component.md (if task involves components)
    ├─> new-system.md (if task involves systems)
    ├─> debug-issue.md (if encountering issues)
    └─> pr-workflow.md (when complete)

debug-issue.md
    └─> pr-workflow.md (to submit fix)

Any workflow
    └─> pr-workflow.md (to submit changes)
```

---

## Examples

### Example 1: Adding a new component

```bash
# Start with the component workflow
cat .claude/workflows/new-component.md

# Follow steps 1-9
# When complete, use PR workflow
cat .claude/workflows/pr-workflow.md
```

### Example 2: Starting a phase task

```bash
# Read the task file
cat docs/tasks/phase1-ecs-core.md

# Follow the phase task workflow
cat .claude/workflows/new-phase-task.md

# This will reference:
# - new-component.md (for components)
# - new-system.md (for systems)
# - debug-issue.md (if issues arise)
# - pr-workflow.md (to submit work)
```

### Example 3: Debugging a crash

```bash
# Use debug workflow
cat .claude/workflows/debug-issue.md

# Follow steps to:
# 1. Reproduce
# 2. Diagnose
# 3. Fix
# 4. Submit PR using pr-workflow.md
```

---

## Contributing to Workflows

If you find a workflow missing steps or have improvements:

1. Create an issue describing the gap
2. Or submit a PR with improvements
3. Or add notes to the workflow file

**Keep workflows:**
- Simple and clear
- Complete and self-contained
- Practical and actionable
- Up-to-date with project changes

---

## Maintenance

These workflows should be updated when:
- Project structure changes
- Build process changes
- CI/CD pipeline changes
- Development tools change
- Best practices evolve

**Review frequency:** Quarterly or after major changes

---

## Additional Resources

### Project Documentation
- [README.md](../../README.md) - Project overview
- [CLAUDE.md](../../CLAUDE.md) - AI agent guide
- [ROADMAP.md](../../ROADMAP.md) - Development roadmap

### Architecture Docs
- [docs/architecture.md](../../docs/architecture.md) - System architecture
- [docs/development-workflow.md](../../docs/development-workflow.md) - Dev workflow
- [docs/testing-strategy.md](../../docs/testing-strategy.md) - Testing guide

### Task Files
- [docs/tasks/](../../docs/tasks/) - All phase tasks

### Coding Standards
- [docs/rules/coding-standards.md](../../docs/rules/coding-standards.md) - Code standards

---

## Support

If you have questions about these workflows:
1. Check the workflow's "Common Errors" section
2. Read referenced documentation
3. Search existing issues
4. Create a new issue with `workflow:` label

---

**Last Updated:** 2026-02-01
