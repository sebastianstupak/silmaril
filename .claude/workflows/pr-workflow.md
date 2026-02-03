# Workflow: Pull Request

> Complete automated workflow for creating and managing pull requests

---

## Prerequisites

- [ ] Work completed on feature branch
- [ ] All tests passing locally
- [ ] Code reviewed by yourself
- [ ] Documentation updated

---

## Step 1: Ensure Clean Working State

**Check git status:**
```bash
git status
```

**Stash uncommitted changes (if any):**
```bash
git stash save "WIP: {description}"
```

**Verify on correct branch:**
```bash
git branch --show-current
# Should be: feat/{feature-name} or fix/{bug-name}
```

---

## Step 2: Update from Main Branch

**Fetch latest:**
```bash
git fetch origin
```

**Rebase on main:**
```bash
git rebase origin/main
```

**If conflicts occur:**
```bash
# Resolve conflicts in editor
git add {resolved-files}
git rebase --continue

# Or abort if needed
git rebase --abort
```

---

## Step 3: Run Pre-PR Checks

**Format code:**
```bash
cargo fmt
```

**Check formatting:**
```bash
cargo fmt --check
```

**Run clippy:**
```bash
cargo clippy --workspace -- -D warnings
```

**Fix any clippy issues:**
```bash
# Auto-fix where possible
cargo clippy --fix --workspace

# Manual fixes for remaining issues
```

---

## Step 4: Run Full Test Suite

**Unit tests:**
```bash
cargo test --workspace --lib
```

**Integration tests:**
```bash
cargo test --workspace --tests
```

**Doc tests:**
```bash
cargo test --workspace --doc
```

**All features:**
```bash
cargo test --workspace --all-features
```

**If tests fail:**
- Fix failing tests
- Commit fixes
- Re-run tests
- Do NOT proceed until all tests pass

---

## Step 5: Build Documentation

**Generate docs:**
```bash
cargo doc --workspace --no-deps
```

**Open and review:**
```bash
cargo doc --workspace --no-deps --open
```

**Check for warnings:**
```bash
cargo doc --workspace --no-deps 2>&1 | grep warning
```

**Fix doc warnings:**
- Add missing documentation
- Fix broken links
- Correct formatting

---

## Step 6: Run Benchmarks (if applicable)

**For performance-related changes:**
```bash
# Save baseline from main
git checkout main
cargo bench -- --save-baseline main

# Run benchmarks on feature branch
git checkout feat/{feature-name}
cargo bench -- --baseline main
```

**Review results:**
```bash
# Check for regressions > 10%
cat target/criterion/*/report/index.html
```

**If significant regressions:**
- Profile and optimize
- Document intentional trade-offs
- Get approval for performance impact

---

## Step 7: Review Your Own Code

**Self-review checklist:**
```bash
# View diff
git diff origin/main...HEAD
```

**Check for:**
- [ ] Debug prints removed (no `println!`, use `tracing`)
- [ ] TODO comments addressed or tracked
- [ ] Commented-out code removed
- [ ] Test coverage for new code
- [ ] Error handling in place
- [ ] Documentation complete
- [ ] No hardcoded values (use config)
- [ ] No sensitive data (passwords, keys)
- [ ] Platform-specific code abstracted
- [ ] Performance considerations addressed

---

## Step 8: Clean Commit History

**View commit history:**
```bash
git log origin/main..HEAD --oneline
```

**Squash commits (if needed):**
```bash
# Interactive rebase
git rebase -i origin/main

# In editor, mark commits to squash:
# pick abc123 First commit
# squash def456 Fix typo
# squash ghi789 Address review comments
```

**Update commit message:**
```
feat(ecs): add query filtering

Implements .with() and .without() filters for entity queries.
Allows filtering entities by component presence/absence.

Performance:
- Filter overhead: < 0.1ms for 10k entities
- Memory overhead: O(1) per filter

Breaking changes: None

Closes #123

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

---

## Step 9: Push to Remote

**Push feature branch:**
```bash
git push -u origin feat/{feature-name}
```

**If already pushed and rebased:**
```bash
# Force push (with lease for safety)
git push --force-with-lease origin feat/{feature-name}
```

---

## Step 10: Gather PR Information

**Analyze changes:**
```bash
# Get list of changed files
git diff --name-only origin/main...HEAD

# Get commit range
git log origin/main..HEAD --oneline

# View full diff
git diff origin/main...HEAD
```

**Collect metrics:**
```bash
# Lines changed
git diff --stat origin/main...HEAD

# Test coverage (if available)
cargo tarpaulin --workspace

# Benchmark results (if ran benchmarks)
cat target/criterion/*/report/index.html
```

---

## Step 11: Create Pull Request

**Using GitHub CLI (recommended):**
```bash
gh pr create \
  --title "feat: Add query filtering to ECS" \
  --body "$(cat <<'EOF'
## Summary
Implements query filtering functionality for the ECS, allowing entities to be filtered by component presence or absence.

### Changes
- Added `QueryFilter` trait for composable filters
- Implemented `.with()` and `.without()` filter methods
- Added filter composition with `.and()` and `.or()`
- Full test coverage (15 new tests)
- Documentation and examples

### Performance
- Filter overhead: < 0.1ms for 10k entities
- Memory overhead: O(1) per filter
- No regression in existing query performance

### Breaking Changes
None. This is purely additive.

### Testing
- Unit tests: 15 new tests
- Integration tests: 3 scenarios
- Benchmarks: All targets met
- Manual testing: Verified in example game

### Documentation
- API documentation complete
- Examples added to rustdoc
- Updated architecture docs

### Related Issues
Closes #123

### Checklist
- [x] Tests passing
- [x] Benchmarks meet targets
- [x] Documentation complete
- [x] No clippy warnings
- [x] Code formatted
- [x] Self-reviewed

Generated with Claude Code
EOF
)" \
  --reviewer {reviewer-username} \
  --label enhancement
```

**Without GitHub CLI:**
```bash
# Push branch
git push -u origin feat/{feature-name}

# Open GitHub in browser
gh browse

# Or manually navigate to:
# https://github.com/{org}/silmaril/compare/main...feat/{feature-name}

# Fill in PR template
```

---

## Step 12: PR Title Format

**Convention:**
```
<type>(<scope>): <subject>

Examples:
feat(ecs): add query filtering
fix(renderer): resolve Vulkan validation errors
docs(architecture): update ECS documentation
perf(networking): optimize packet serialization
refactor(physics): simplify collision detection
test(ecs): add integration tests for queries
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style/formatting
- `refactor`: Code refactor (no behavior change)
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Build, tooling, dependencies

---

## Step 13: PR Description Template

**Use this template:**
```markdown
## Summary
Brief description of changes (1-2 sentences).

### Changes
- Bullet list of what changed
- Keep it high-level
- Focus on "what" not "how"

### Performance
{if applicable}
- Metric 1: {value}
- Metric 2: {value}

### Breaking Changes
{if any, otherwise "None"}

### Testing
- Unit tests: {count} tests
- Integration tests: {count} scenarios
- Manual testing: {description}

### Documentation
- API docs: {complete/incomplete}
- Examples: {yes/no}
- Architecture docs: {updated/not needed}

### Related Issues
Closes #{issue-number}
Fixes #{issue-number}
Relates to #{issue-number}

### Checklist
- [ ] Tests passing
- [ ] Benchmarks meet targets
- [ ] Documentation complete
- [ ] No clippy warnings
- [ ] Code formatted
- [ ] Self-reviewed

### Screenshots
{if UI/visual changes}

Generated with Claude Code
```

---

## Step 14: Request Reviews

**Assign reviewers:**
```bash
# Using GitHub CLI
gh pr edit --add-reviewer {username}
gh pr edit --add-reviewer {team-name}

# Or in GitHub web UI
```

**Who to request:**
- **Core maintainers:** For architectural changes
- **Domain experts:** For specialized components (rendering, networking)
- **Documentation team:** For public API changes
- **Performance team:** For performance-critical code

**Notify reviewers:**
- Comment on PR with context
- Mention specific areas needing attention
- Link to relevant documentation

---

## Step 15: Address Review Comments

**When reviews come in:**

**For requested changes:**
```bash
# Make changes
{edit files}

# Commit
git add .
git commit -m "Address review comments

- Fix typo in documentation
- Add error handling for edge case
- Improve test coverage"

# Push
git push origin feat/{feature-name}
```

**Respond to comments:**
- Mark as resolved when fixed
- Explain decisions if not changing
- Ask clarifying questions
- Thank reviewers for feedback

**Re-request review:**
```bash
gh pr ready
# Or click "Re-request review" in GitHub UI
```

---

## Step 16: Handle CI Failures

**If CI fails:**

**Check CI logs:**
```bash
gh pr checks
# Or view in GitHub web UI
```

**Common failures:**

1. **Format check:**
```bash
cargo fmt
git add .
git commit -m "Fix formatting"
git push
```

2. **Clippy warnings:**
```bash
cargo clippy --fix --workspace
git add .
git commit -m "Fix clippy warnings"
git push
```

3. **Test failures:**
```bash
# Run failing test locally
cargo test {test_name}

# Fix issue
{edit files}

git add .
git commit -m "Fix failing test"
git push
```

4. **Platform-specific failures:**
```bash
# Test on specific platform
cargo test --target x86_64-pc-windows-msvc

# Or use CI artifacts to debug
gh run download {run-id}
```

---

## Step 17: Update PR if Needed

**Add more commits:**
```bash
# Make changes
{edit files}

git add .
git commit -m "feat(ecs): add additional filter methods"
git push origin feat/{feature-name}
```

**Update PR description:**
```bash
gh pr edit --body "Updated description..."

# Or edit in GitHub web UI
```

**Add labels:**
```bash
gh pr edit --add-label "needs-performance-review"
gh pr edit --add-label "breaking-change"
```

---

## Step 18: Merge PR

**After approval:**

**Ensure CI is green:**
```bash
gh pr checks
# All checks should be passing
```

**Update from main (if needed):**
```bash
git fetch origin
git rebase origin/main
git push --force-with-lease origin feat/{feature-name}
```

**Merge PR:**
```bash
# Squash and merge (recommended for feature branches)
gh pr merge --squash --delete-branch

# Or merge commit (for important features)
gh pr merge --merge --delete-branch

# Or rebase (for clean history)
gh pr merge --rebase --delete-branch
```

**Verify merge:**
```bash
git checkout main
git pull origin main
git log --oneline -5
```

---

## Step 19: Clean Up

**Delete local branch:**
```bash
git branch -d feat/{feature-name}
```

**Delete remote branch (if not auto-deleted):**
```bash
git push origin --delete feat/{feature-name}
```

**Prune remote references:**
```bash
git fetch --prune
```

---

## Step 20: Post-Merge Tasks

**Update issue:**
```bash
# If PR closed an issue, verify it's closed
gh issue view {issue-number}
```

**Update documentation (if needed):**
- Update CHANGELOG.md
- Update migration guides
- Update examples

**Announce (if significant):**
- Post in team chat
- Update roadmap
- Create release notes

---

## Common PR Scenarios

### Scenario: Large Feature

**Split into multiple PRs:**
1. **PR 1:** Core infrastructure
2. **PR 2:** Basic functionality
3. **PR 3:** Advanced features
4. **PR 4:** Documentation and examples

**Benefits:**
- Easier to review
- Faster iteration
- Less risk of conflicts
- Can be merged incrementally

---

### Scenario: Breaking Change

**Extra requirements:**
1. Document breaking changes clearly
2. Provide migration guide
3. Update version (semantic versioning)
4. Get approval from maintainers
5. Announce to users

**PR description additions:**
```markdown
## Breaking Changes

### What changed
{Description of breaking change}

### Migration Guide
```rust
// Before
let query = world.query::<&Transform>();

// After
let query = world.query::<&Transform>().filter(Active);
```

### Affected Users
{Who is impacted}

### Workaround
{If any temporary workaround exists}
```

---

### Scenario: Hotfix

**Fast-track process:**
```bash
# Create fix branch from main
git checkout main
git pull origin main
git checkout -b fix/critical-bug

# Make minimal fix
{edit files}

# Test thoroughly
cargo test --workspace --all-features

# Commit
git add .
git commit -m "fix: resolve critical entity leak"

# Push and create PR
git push -u origin fix/critical-bug
gh pr create --title "fix: Resolve critical entity leak" \
  --body "Critical fix for production issue" \
  --label hotfix

# Request immediate review
gh pr edit --add-reviewer {maintainer}
```

---

## PR Checklist

**Before creating PR:**
- [ ] Branch up-to-date with main
- [ ] All tests passing
- [ ] Code formatted
- [ ] No clippy warnings
- [ ] Documentation complete
- [ ] Benchmarks run (if applicable)
- [ ] Self-reviewed code
- [ ] Commit history clean

**In PR:**
- [ ] Clear title following convention
- [ ] Comprehensive description
- [ ] Related issues linked
- [ ] Reviewers assigned
- [ ] Labels added
- [ ] Breaking changes documented
- [ ] Migration guide (if needed)

**After review:**
- [ ] All comments addressed
- [ ] CI passing
- [ ] Re-review requested
- [ ] Approvals received

**Before merge:**
- [ ] Up-to-date with main
- [ ] CI green
- [ ] Approvals in place
- [ ] Breaking changes approved

**After merge:**
- [ ] Branches deleted
- [ ] Issues closed
- [ ] Documentation updated
- [ ] Announced (if significant)

---

## Automation Tips

**Pre-commit hook:**
```bash
# .git/hooks/pre-commit
#!/bin/bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

**Pre-push hook:**
```bash
# .git/hooks/pre-push
#!/bin/bash
cargo test --workspace
```

**GitHub Actions:**
- Auto-format on push
- Run tests on all platforms
- Generate benchmark comparisons
- Check documentation coverage

---

## References

- [docs/development-workflow.md](../../docs/development-workflow.md) - Dev workflow
- [docs/rules/coding-standards.md](../../docs/rules/coding-standards.md) - Code standards
- [CLAUDE.md](../../CLAUDE.md) - AI agent guide

---

**Last Updated:** 2026-02-01
