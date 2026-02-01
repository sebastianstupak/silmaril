# Bencher.dev Integration Guide

[Bencher.dev](https://bencher.dev) is a continuous benchmarking platform that tracks performance over time and provides historical analysis. This is **optional** but recommended for long-term performance tracking.

---

## Why Use Bencher?

- **Historical tracking**: See performance trends over months/years
- **PR comments**: Automatic benchmark comparisons on pull requests
- **Alerts**: Email/Slack notifications on regressions
- **Charts**: Visualize performance over time
- **Platform comparison**: Compare Linux/Windows/macOS results

---

## Setup Instructions

### 1. Create Bencher Account

1. Go to https://bencher.dev
2. Sign up (free for open source)
3. Create organization (if needed)

### 2. Create Project

1. Click "New Project"
2. Name: `agent-game-engine`
3. Visibility: Public (for open source) or Private
4. Click "Create"

### 3. Get API Token

1. Go to Project Settings
2. Navigate to "API Tokens"
3. Click "Create Token"
4. Name: `github-actions`
5. Permissions: `write:benchmark`
6. Copy the token (you'll only see it once!)

### 4. Add GitHub Secret

1. Go to your GitHub repository
2. Settings → Secrets and variables → Actions
3. Click "New repository secret"
4. Name: `BENCHER_API_TOKEN`
5. Value: Paste the token from step 3
6. Click "Add secret"

### 5. Enable Workflow

Edit `.github/workflows/benchmark-regression.yml` and uncomment the `bencher-tracking` job:

```yaml
bencher-tracking:
  name: Track Benchmarks with Bencher
  runs-on: ubuntu-latest
  if: github.event_name == 'pull_request' || github.ref == 'refs/heads/main'
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2

    # Install Vulkan SDK (required for benchmarks)
    - name: Install Vulkan SDK
      run: |
        wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
        sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-jammy.list \
          https://packages.lunarg.com/vulkan/lunarg-vulkan-jammy.list
        sudo apt update
        sudo apt install vulkan-sdk

    - name: Install Bencher CLI
      run: |
        curl -sSL https://bencher.dev/install.sh | sh
        echo "$HOME/.bencher/bin" >> $GITHUB_PATH

    - name: Run benchmarks with Bencher
      env:
        BENCHER_API_TOKEN: ${{ secrets.BENCHER_API_TOKEN }}
      run: |
        bencher run \
          --project agent-game-engine \
          --adapter criterion \
          --testbed ubuntu-latest \
          --branch ${{ github.head_ref || github.ref_name }} \
          --threshold-measure latency \
          --threshold-test t_test \
          --threshold-max-sample-size 64 \
          --threshold-upper-boundary 0.20 \
          --err \
          "cargo bench --features profiling-puffin --no-fail-fast"
```

### 6. Test It

Create a test PR:

```bash
git checkout -b test-bencher
# Make a trivial change
echo "# Test" >> README.md
git add README.md
git commit -m "Test bencher integration"
git push origin test-bencher
```

Open a PR and check:
1. GitHub Actions runs `bencher-tracking` job
2. Bencher dashboard shows results
3. PR gets a comment with benchmark comparison (after second run)

---

## Configuration Options

### Adapters

Bencher supports multiple benchmark formats:

```bash
--adapter criterion     # For Rust Criterion (default)
--adapter iai           # For iai-callgrind
--adapter json          # Custom JSON format
```

### Testbeds

Track different platforms separately:

```bash
--testbed ubuntu-latest
--testbed windows-latest
--testbed macos-latest
```

### Branches

Bencher tracks benchmarks per branch:

```bash
--branch main                           # Main branch
--branch ${{ github.head_ref }}        # PR branch (auto)
--branch feature/my-feature            # Custom branch
```

### Thresholds

Configure regression detection:

```bash
--threshold-measure latency            # What to measure
--threshold-test t_test                # Statistical test
--threshold-max-sample-size 64         # Sample size
--threshold-upper-boundary 0.20        # 20% regression threshold
```

**Available tests:**
- `t_test` - Student's t-test (default, good for most cases)
- `z_score` - Z-score (for large sample sizes)
- `percentage` - Simple percentage change

---

## Viewing Results

### Dashboard

Go to https://bencher.dev/agent-game-engine

**What you'll see:**
- **Benchmarks**: All tracked benchmarks
- **Branches**: Performance per branch
- **Testbeds**: Performance per platform
- **Alerts**: Detected regressions
- **Charts**: Performance over time

### PR Comments

Bencher automatically comments on PRs with:
- Benchmark comparison table
- Detected regressions (highlighted in red)
- Link to detailed results
- Performance charts

### CLI

Query results from command line:

```bash
# View latest results
bencher perf --project agent-game-engine --branch main

# Compare branches
bencher perf \
  --project agent-game-engine \
  --branch main \
  --compare feature/my-feature

# View specific benchmark
bencher perf \
  --project agent-game-engine \
  --branch main \
  --benchmark profiling_overhead
```

---

## Advanced: Custom Metrics

Track custom metrics beyond wall-clock time:

```rust
// In your benchmark
use bencher::Bencher;

fn custom_metric(b: &mut Bencher) {
    b.iter(|| {
        // Your code
    });

    // Add custom metric
    b.metric("memory_mb", 123.45);
    b.metric("allocations", 1000);
}
```

Then configure Bencher to track it:

```bash
bencher run \
  --project agent-game-engine \
  --adapter criterion \
  --measure memory_mb \
  "cargo bench custom_metric"
```

---

## Troubleshooting

### "Project not found"

**Cause:** Wrong project name or API token doesn't have access.

**Fix:**
```bash
# Check project name
bencher project list

# Verify API token works
bencher project view --project agent-game-engine
```

### "No benchmarks found"

**Cause:** Bencher couldn't parse Criterion output.

**Fix:**
```bash
# Test locally
cargo bench --features profiling-puffin > bench.txt
bencher run --adapter criterion "cat bench.txt"

# Check adapter is correct
bencher run --adapter criterion --dry-run "cargo bench --features profiling-puffin"
```

### "Threshold exceeded"

**Cause:** Benchmark regressed beyond threshold.

**Fix:**
1. Check Bencher dashboard for details
2. Investigate the regression
3. Either fix the performance issue or adjust threshold if expected

---

## Cost

**Free tier:**
- Unlimited public projects
- Unlimited benchmarks
- 30 days of data retention
- Community support

**Paid tier:**
- Private projects
- Unlimited data retention
- Priority support
- Advanced analytics

**For open source:** Request free unlimited plan at https://bencher.dev/opensource

---

## Best Practices

1. **Run on main only initially** - Don't track every PR until you have stable baselines
2. **Use statistical tests** - `t_test` is more robust than simple percentage
3. **Set reasonable thresholds** - 20% for wall-clock time, 10% for instruction counts
4. **Track multiple platforms** - Performance varies across OS/architecture
5. **Review trends weekly** - Don't just react to individual regressions
6. **Document expected changes** - Add context to PRs that intentionally change performance

---

## Alternative: GitHub Actions Benchmark Action

If you don't want to use Bencher, you can use the built-in GitHub Actions benchmark action:

```yaml
- name: Store benchmark result
  uses: benchmark-action/github-action-benchmark@v1
  with:
    tool: 'criterion'
    output-file-path: target/criterion/
    github-token: ${{ secrets.GITHUB_TOKEN }}
    auto-push: true
```

This is simpler but has fewer features than Bencher.

---

## References

- [Bencher Documentation](https://bencher.dev/docs)
- [Criterion Adapter](https://bencher.dev/docs/adapters/criterion)
- [GitHub Actions Integration](https://bencher.dev/docs/ci/github-actions)
- [Statistical Tests](https://bencher.dev/docs/thresholds/statistical-tests)

---

**Last Updated:** 2026-02-01
**Status:** Optional (Task 0.5.8)
