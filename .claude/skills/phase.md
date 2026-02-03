---
name: phase
description: Show current project phase status and suggest next steps
trigger: /phase
---

# Project Phase Status

Shows the current phase status from ROADMAP.md, task completion, and suggests next steps.

## Instructions

1. **Read ROADMAP.md**
   ```bash
   # Read the current roadmap
   cat D:\dev\silmaril\ROADMAP.md
   ```

2. **Identify Current Phase**
   - Parse ROADMAP.md to find the phase marked as "In Progress" (🟢)
   - Note phases that are "Not Started" (⚪)
   - Note completed phases (✅)

3. **Parse Phase Tasks**
   - Extract all tasks for the current phase
   - Identify checked tasks `[x]` as completed
   - Identify unchecked tasks `[ ]` as pending
   - Note task files referenced (e.g., `docs/tasks/phase0-documentation.md`)

4. **Check Task File Details**
   If task files exist, read them for more details:
   ```bash
   # Read detailed task file if exists
   cat docs/tasks/phase0-documentation.md
   ```

5. **Calculate Progress**
   - Count total tasks in current phase
   - Count completed tasks
   - Calculate percentage complete
   - Identify next immediate task(s)

6. **Check Deliverables**
   - List phase deliverables from ROADMAP.md
   - Mark which are complete (✅) vs pending
   - Estimate remaining work

7. **Suggest Next Steps**
   Based on uncompleted tasks:
   - Prioritize tasks by dependencies
   - Suggest which task to work on next
   - Identify any blockers
   - Reference relevant documentation

8. **Show Timeline Context**
   - Current phase duration estimate
   - Time elapsed (if trackable)
   - Overall project timeline position

## Output Format

Provide a comprehensive status report:

```
Project Phase Status
====================

Current Phase: Phase 0 - Documentation & Foundation
Status: 🟢 IN PROGRESS
Timeline: Week 1 of 19 (5% complete)

Progress Overview:
------------------
Completed:   8 / 25 tasks (32%)
In Progress: 3 tasks
Pending:     14 tasks

Task Breakdown:
---------------

✅ 0.1 Documentation (2/11 complete)
  ✅ CLAUDE.md (AI agent guide)
  ✅ ROADMAP.md (implementation plan)
  ⏳ docs/architecture.md        [IN PROGRESS]
  ⏳ docs/ecs.md                  [IN PROGRESS]
  ☐  docs/networking.md
  ☐  docs/rendering.md
  ☐  docs/physics.md
  ☐  docs/platform-abstraction.md
  ☐  docs/error-handling.md
  ☐  docs/testing-strategy.md
  ☐  docs/performance-targets.md
  ☐  docs/development-workflow.md
  ☐  docs/rules/coding-standards.md

☐  0.2 Repository Setup (0/6 complete)
  ☐  Create workspace Cargo.toml
  ☐  Set up directory structure
  ☐  Configure .gitignore
  ☐  Set up .cargo/config.toml
  ☐  Create LICENSE (Apache-2.0)
  ☐  Create README.md

☐  0.3 CI/CD Setup (0/8 complete)
  ☐  GitHub Actions: Windows CI
  ☐  GitHub Actions: Linux CI
  ☐  GitHub Actions: macOS x64 CI
  ☐  GitHub Actions: macOS ARM CI
  ☐  GitHub Actions: WASM CI
  ☐  GitHub Actions: Clippy + fmt
  ☐  GitHub Actions: Security audit
  ☐  Branch protection rules

Deliverables Status:
--------------------
☐  Complete documentation structure
☐  CI/CD passing on all Tier 1 platforms
☐  Dev environment working locally

Next Steps:
-----------
1. Complete documentation files in progress:
   - Finish docs/architecture.md
   - Finish docs/ecs.md

2. High Priority Tasks:
   - docs/error-handling.md (needed before Phase 1)
   - docs/testing-strategy.md (needed before Phase 1)
   - docs/rules/coding-standards.md (blocks all code work)

3. Recommended Order:
   a) Complete 0.1 Documentation (finish remaining docs)
   b) Start 0.2 Repository Setup (Cargo.toml, structure)
   c) Start 0.3 CI/CD Setup (parallel with 0.2)

Blockers:
---------
None identified. All tasks can proceed.

Detailed Task Info:
-------------------
For detailed breakdowns, see:
  - docs/tasks/phase0-documentation.md
  - docs/tasks/phase0-repo-setup.md
  - docs/tasks/phase0-cicd.md

Overall Timeline:
-----------------
Total Project: 13-19 weeks (3-5 months)
Current: Phase 0, Week 1
Next Phase: Phase 1 (Core ECS + Basic Rendering)
```

## Phase Transition Checklist

When a phase is nearly complete, show:

```
Phase Completion Checklist
===========================

Before moving to Phase 1:
☐  All Phase 0 tasks completed
☐  All Phase 0 deliverables met
☐  Documentation reviewed
☐  CI/CD passing
☐  Retrospective completed

Phase 1 Prerequisites:
✅ Documentation complete
☐  CI/CD configured
☐  Development workflow tested
```

## Notes

- Update ROADMAP.md status when tasks complete
- Use task status markers: ✅ (done), 🟢 (in progress), ⚪ (not started)
- Reference task files in docs/tasks/ for detailed breakdowns
- Consider dependencies when suggesting next tasks
- Highlight blockers that prevent progress
- Show realistic time estimates based on ROADMAP.md
