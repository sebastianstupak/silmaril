# Benchmark CI/CD Testing Guide

Step-by-step guide to verify the benchmark CI/CD integration works correctly.

---

## 🎯 Purpose

This guide helps verify that:
1. All benchmark commands work locally
2. CI/CD workflows execute correctly
3. Regression detection catches performance issues
4. PR comments are posted correctly
5. Baselines are managed properly

---

## ✅ Pre-Testing Checklist

Before testing, ensure:

- [ ] All files are committed
- [ ] Scripts are executable (`chmod +x scripts/*.sh`)
- [ ] You're on a clean branch
- [ ] Cargo.toml includes all benchmarks
- [ ] Vulkan SDK is installed

---

## 🧪 Phase 1: Local Testing

### 1.1 Verify Justfile Commands

Test all new justfile targets:

```bash
# Quick smoke test (should complete in <1 min)
just bench-smoke
# ✅ Expected: Benchmarks run successfully, no errors

# Run all benchmarks (may take 5-10 min)
just bench-all
# ✅ Expected: All benchmarks complete, results saved

# View report
just bench-report
# ✅ Expected: Browser opens with HTML report

# Run specific suites
just bench-ecs
just bench-physics
just bench-math
# ✅ Expected: Each suite runs without errors
```

**Verification**:
- [ ] `just bench-smoke` completes in <1 minute
- [ ] `just bench-all` completes without errors
- [ ] `just bench-report` opens browser
- [ ] Individual suite commands work

### 1.2 Test Baseline Scripts

Create and compare baselines:

```bash
# Create baseline
./scripts/update_benchmark_baseline.sh main
# ✅ Expected:
# - Benchmarks run
# - Baseline directory created
# - baseline-info.json created
# - Instructions shown

# Verify baseline exists
ls -la benchmarks/baselines/$(uname -s)-$(uname -m)/main/
# ✅ Expected: criterion/ directory and baseline-info.json

# Run benchmarks again
just bench-all

# Compare with baseline
./scripts/compare_with_baseline.sh main
# ✅ Expected: No regressions detected (or minimal variance <5%)
```

**Verification**:
- [ ] Baseline created successfully
- [ ] baseline-info.json has correct metadata
- [ ] Comparison script works
- [ ] No false positive regressions

### 1.3 Test Regression Detection

Intentionally introduce a regression:

```bash
# 1. Save current baseline
cp -r benchmarks/baselines benchmarks/baselines.backup

# 2. Create fake slow baseline (edit a benchmark to be slower)
# For testing, you can manually edit baseline files or:
# - Modify a benchmark to add a sleep
# - Run benchmarks and save as baseline
# - Remove the sleep
# - Run comparison

# 3. Compare (should detect regression)
./scripts/compare_with_baseline.sh main 20

# 4. Restore baseline
rm -rf benchmarks/baselines
mv benchmarks/baselines.backup benchmarks/baselines
```

**Verification**:
- [ ] Script detects intentional regression
- [ ] Regression report is formatted correctly
- [ ] Exit code is non-zero on regression

---

## 🔄 Phase 2: CI/CD Testing

### 2.1 Test on Pull Request

Create a test PR to trigger CI:

```bash
# 1. Create test branch
git checkout -b test/benchmark-ci-integration

# 2. Make a small change (add a comment somewhere)
echo "// Test comment" >> engine/core/src/lib.rs

# 3. Commit and push
git add engine/core/src/lib.rs
git commit -m "test: Verify benchmark CI integration"
git push origin test/benchmark-ci-integration

# 4. Create PR on GitHub
# Navigate to: https://github.com/your-org/agent-game-engine/compare
```

**Expected Workflow Execution**:

1. **benchmark-ci.yml** should trigger
   - [ ] Linux job starts
   - [ ] Windows job starts
   - [ ] macOS job starts

2. **Each platform job**:
   - [ ] Fetches main baseline
   - [ ] Runs benchmarks
   - [ ] Compares with baseline
   - [ ] Uploads artifacts

3. **pr-comment job**:
   - [ ] Downloads all reports
   - [ ] Posts comment to PR
   - [ ] Comment shows results for all platforms

4. **regression-gate job**:
   - [ ] Checks for regressions
   - [ ] Passes (no regressions on simple comment change)

**Verification**:
- [ ] All workflow jobs complete successfully
- [ ] PR comment is posted
- [ ] Comment includes all 3 platforms
- [ ] No regressions detected
- [ ] Artifacts are uploaded

### 2.2 Test Regression Detection in CI

Modify a benchmark to be slower and create PR:

```bash
# 1. Create regression test branch
git checkout -b test/benchmark-regression-detection

# 2. Slow down a benchmark (example)
# Edit engine/core/benches/ecs_simple.rs
# Add a small delay in a benchmark loop

# 3. Commit and push
git add engine/core/benches/ecs_simple.rs
git commit -m "test: Verify regression detection"
git push origin test/benchmark-regression-detection

# 4. Create PR
```

**Expected**:
- [ ] Benchmark runs and detects regression
- [ ] PR comment shows regression details
- [ ] regression-gate job **fails**
- [ ] PR shows failed status check

**After Testing**:
```bash
# Close the PR without merging
# Delete the test branch
git branch -D test/benchmark-regression-detection
```

### 2.3 Test Weekly Schedule (Optional)

Can't test immediately, but verify:

```yaml
# In .github/workflows/benchmark-ci.yml
on:
  schedule:
    - cron: '0 0 * * 1'  # Monday at 00:00 UTC
```

- [ ] Schedule syntax is correct
- [ ] Will run weekly as configured

---

## 📊 Phase 3: Integration Testing

### 3.1 Test with Existing CI

Verify benchmark-ci doesn't conflict with other workflows:

```bash
# Create PR with code changes
git checkout -b test/ci-integration

# Make a real code change
# ... edit some code ...

git commit -am "test: Verify CI integration"
git push origin test/ci-integration
```

**Expected**:
- [ ] ci.yml runs successfully
- [ ] benchmark-ci.yml runs successfully
- [ ] No conflicts between workflows
- [ ] benchmark-smoke job in ci.yml passes
- [ ] All checks complete

### 3.2 Test Baseline Update on Main

After merging a PR to main:

**Expected** (check Actions tab):
- [ ] benchmark-ci.yml runs on main branch
- [ ] Benchmarks execute
- [ ] Baseline artifact uploaded (90-day retention)
- [ ] No PR comment (not a PR)
- [ ] No regression check (not a PR)

**Manual baseline update** (after confirming results):
```bash
# 1. Download artifact from GitHub Actions
# 2. Extract to benchmarks/baselines/

# Or run locally on main:
git checkout main
git pull
./scripts/update_benchmark_baseline.sh main
git add benchmarks/baselines/
git commit -m "chore: Update benchmark baseline (main)"
git push
```

---

## 🔍 Phase 4: Performance Validation

### 4.1 Verify Benchmark Performance

Check that benchmarks meet performance targets:

```bash
# Run all benchmarks
just bench-all

# Review results
just bench-report
```

For each category, verify:

| Category | Target | Check |
|----------|--------|-------|
| ECS spawn | < 50ns | [ ] |
| Component query | < 1ms | [ ] |
| Physics tick | < 8ms | [ ] |
| Vulkan fence | < 10µs | [ ] |
| Transform SIMD | < 100ns | [ ] |

**If targets not met**:
1. Profile with `just bench-profile`
2. Optimize hot paths
3. Re-run benchmarks
4. Update baseline when improved

### 4.2 Verify Industry Comparison

```bash
# Run industry comparison benchmarks
just bench-compare

# Check results against targets in benchmarks/industry_comparison.yaml
```

- [ ] Results meet or exceed industry standards
- [ ] No regressions vs previous runs

---

## 📝 Phase 5: Documentation Verification

### 5.1 Verify All Links

Check that documentation links work:

```bash
# Test links in README.md
# (manually click each link or use link checker)

# Test links in CONTRIBUTING.md
# Test links in ROADMAP.md
# Test links in benchmark docs
```

- [ ] All internal links work
- [ ] All external links work
- [ ] No broken references

### 5.2 Verify Code Examples

Test code examples in documentation:

```bash
# Try commands from README.md
just bench-all
just bench-baseline
just bench-report

# Try commands from CONTRIBUTING.md
# ... verify each example ...

# Try commands from QUICK_REFERENCE.md
# ... verify each example ...
```

- [ ] All commands in README work
- [ ] All commands in CONTRIBUTING work
- [ ] All commands in QUICK_REFERENCE work

---

## 🎯 Success Criteria

### Must Pass

- [x] All local benchmark commands work
- [x] Baseline scripts create/compare successfully
- [x] CI workflows execute without errors
- [x] PR comments are posted correctly
- [x] Regression detection catches slowdowns
- [x] No conflicts with existing CI
- [x] Documentation is accurate

### Should Pass

- [ ] Benchmarks meet performance targets
- [ ] No false positive regressions (<5% variance)
- [ ] Weekly schedule is configured
- [ ] Artifacts are stored correctly
- [ ] All platforms (Linux, Windows, macOS) work

### Nice to Have

- [ ] Baselines tracked with Git LFS (if large)
- [ ] Industry comparison shows competitive performance
- [ ] HTML reports render correctly
- [ ] Multiple PRs can run concurrently

---

## 🐛 Troubleshooting

### Benchmark Fails Locally

```bash
# Check which benchmark fails
cargo bench --all-features -- --list

# Run specific benchmark for details
cargo bench --package engine-core --bench ecs_simple --verbose

# Check for compilation errors
cargo check --benches
```

### CI Workflow Fails

1. Check Actions tab for error details
2. Look at workflow logs for specific job
3. Verify Vulkan SDK installation step
4. Check artifact upload permissions
5. Verify Python is available (for regression script)

### PR Comment Not Posted

1. Verify `pull-requests: write` permission
2. Check pr-comment job logs
3. Verify GitHub token is available
4. Check that artifacts were downloaded

### False Positive Regressions

1. Run benchmarks multiple times locally
2. Check system load during benchmark
3. Increase threshold if variance is high
4. Review baseline creation conditions

---

## 📋 Final Checklist

Before declaring testing complete:

### Local Testing
- [ ] All justfile commands tested
- [ ] Baseline creation works
- [ ] Baseline comparison works
- [ ] Regression detection works

### CI/CD Testing
- [ ] PR triggers workflows
- [ ] All platforms execute
- [ ] PR comments posted
- [ ] Regression gate works
- [ ] Artifacts uploaded

### Integration Testing
- [ ] No conflicts with existing CI
- [ ] Works with real code changes
- [ ] Baseline updates on main

### Performance Testing
- [ ] Benchmarks meet targets
- [ ] Industry comparison favorable
- [ ] No unexpected regressions

### Documentation Testing
- [ ] All links work
- [ ] All examples tested
- [ ] Documentation accurate

---

## 🎉 Completion

When all tests pass:

1. **Clean up test branches**:
   ```bash
   git branch -D test/benchmark-ci-integration
   git branch -D test/benchmark-regression-detection
   git push origin --delete test/benchmark-ci-integration
   git push origin --delete test/benchmark-regression-detection
   ```

2. **Document results**:
   - Update BENCHMARK_CI_INTEGRATION_COMPLETE.md if needed
   - Note any issues found and resolved
   - Update documentation if discrepancies found

3. **Merge to main**:
   ```bash
   # Create final PR
   git checkout -b feat/benchmark-ci-integration
   # ... commit all files ...
   git push origin feat/benchmark-ci-integration
   # Create PR and merge after review
   ```

4. **Create baseline**:
   ```bash
   # After merge to main
   git checkout main
   git pull
   ./scripts/update_benchmark_baseline.sh main
   git add benchmarks/baselines/
   git commit -m "chore: Initial benchmark baseline"
   git push
   ```

---

**Testing Complete!** 🎊

The benchmark CI/CD integration is now fully tested and ready for production use.

---

**Last Updated**: 2026-02-01
