---
name: commit
description: Smart commit with conventional commit format
trigger: /commit
---

# Smart Commit Skill

Creates a well-formatted commit following conventional commit standards and project conventions.

## Instructions

1. **Analyze Changes**
   - Run `git status` to see all changes (never use -uall flag)
   - Run `git diff --cached` to see staged changes
   - Run `git diff` to see unstaged changes
   - Run `git log -10 --oneline` to understand commit message style

2. **Generate Commit Message**
   - Analyze all changes to determine the type:
     - `feat`: New feature or capability
     - `fix`: Bug fix
     - `docs`: Documentation changes
     - `refactor`: Code restructuring without behavior change
     - `test`: Adding or updating tests
     - `perf`: Performance improvements
     - `chore`: Build process, dependencies, tooling
     - `style`: Code formatting (not CSS)
   - Write concise subject line (max 72 characters)
   - Focus on "why" not "what" (the diff shows the what)
   - Use imperative mood ("Add feature" not "Added feature")
   - Add detailed body if needed to explain context
   - Reference ROADMAP.md phase if applicable

3. **Stage Files**
   - If there are unstaged changes that should be committed, stage them
   - NEVER stage sensitive files (.env, credentials, secrets)
   - Prefer staging specific files by name rather than `git add -A`
   - Warn if sensitive files are about to be committed

4. **Create Commit**
   - Use heredoc format for proper message formatting:
   ```bash
   git commit -m "$(cat <<'EOF'
   type(scope): subject line

   Optional body with more details.

   Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
   EOF
   )"
   ```
   - ALWAYS include `Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>` at the end
   - Run `git status` after commit to verify success

5. **Handle Errors**
   - If pre-commit hooks fail, fix the issue and create a NEW commit (never --amend)
   - If no changes to commit, inform the user
   - If commit fails, show the error and suggest solutions

## Examples

### Example 1: Feature Addition
```
feat(ecs): add sparse set component storage

Implements sparse set data structure for efficient component storage
with O(1) add/remove and cache-friendly iteration. Part of Phase 1.1.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### Example 2: Bug Fix
```
fix(networking): prevent packet duplication on UDP channel

Adds sequence number tracking to detect and filter duplicate packets
that may arrive out of order.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

### Example 3: Documentation
```
docs(architecture): add ECS design documentation

Documents component storage, query system, and serialization approach
for Phase 1 implementation.

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

## Notes

- DO NOT push unless explicitly requested by the user
- NEVER use `git commit --amend` unless explicitly requested
- NEVER use `git add -A` or `git add .` without checking for sensitive files
- Follow project's conventional commit format
- Keep subject line under 72 characters
- Reference ROADMAP.md phase when applicable
