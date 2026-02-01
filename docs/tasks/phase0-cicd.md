# Phase 0.3: CI/CD Pipeline

**Status:** ⚪ Not Started
**Estimated Time:** 1 day
**Priority:** Critical (prevents regressions)

---

## 🎯 **Objective**

Set up GitHub Actions CI/CD pipeline to automatically test on all platforms (Windows, Linux, macOS x64/ARM) and enforce code quality standards.

---

## 📋 **Tasks**

### **1. Main CI Workflow**

**File:** `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # Format check
  fmt:
    name: Format Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - name: Check formatting
        run: cargo fmt --all -- --check

  # Clippy lints
  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - name: Install Vulkan SDK
        run: |
          wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
          sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-jammy.list \
            https://packages.lunarg.com/vulkan/lunarg-vulkan-jammy.list
          sudo apt update
          sudo apt install vulkan-sdk
      - name: Run clippy
        run: cargo clippy --workspace --all-features -- -D warnings

  # Test matrix (all platforms)
  test:
    name: Test
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest, macos-14]
        rust: [stable]
        include:
          - os: ubuntu-latest
            platform: linux
          - os: windows-latest
            platform: windows
          - os: macos-latest
            platform: macos-x64
          - os: macos-14
            platform: macos-arm64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      # Platform-specific Vulkan setup
      - name: Install Vulkan SDK (Linux)
        if: matrix.platform == 'linux'
        run: |
          wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
          sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-jammy.list \
            https://packages.lunarg.com/vulkan/lunarg-vulkan-jammy.list
          sudo apt update
          sudo apt install vulkan-sdk libxcb1-dev libx11-dev

      - name: Install Vulkan SDK (Windows)
        if: matrix.platform == 'windows'
        shell: pwsh
        run: |
          $ProgressPreference = 'SilentlyContinue'
          Invoke-WebRequest -Uri "https://sdk.lunarg.com/sdk/download/latest/windows/vulkan-sdk.exe" -OutFile VulkanSDK.exe
          .\VulkanSDK.exe /S
          echo "VULKAN_SDK=C:\VulkanSDK" >> $env:GITHUB_ENV

      - name: Install MoltenVK (macOS)
        if: startsWith(matrix.platform, 'macos')
        run: |
          brew install molten-vk

      # Cache dependencies
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.os }}-${{ matrix.rust }}

      # Run tests
      - name: Run tests
        run: cargo test --workspace --all-features --verbose

      # Doc tests
      - name: Run doc tests
        run: cargo test --doc --workspace

  # Code coverage
  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Vulkan SDK
        run: |
          wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
          sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-jammy.list \
            https://packages.lunarg.com/vulkan/lunarg-vulkan-jammy.list
          sudo apt update
          sudo apt install vulkan-sdk
      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin
      - name: Generate coverage
        run: cargo tarpaulin --workspace --all-features --out Xml --timeout 300
      - name: Upload coverage to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml
          fail_ci_if_error: true

  # Documentation build
  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Vulkan SDK
        run: |
          wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
          sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-jammy.list \
            https://packages.lunarg.com/vulkan/lunarg-vulkan-jammy.list
          sudo apt update
          sudo apt install vulkan-sdk
      - name: Build docs
        run: cargo doc --no-deps --workspace --all-features
      - name: Check for warnings
        run: cargo doc --no-deps --workspace --all-features 2>&1 | grep -i warning && exit 1 || exit 0

  # Security audit
  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v1
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
```

---

### **2. Benchmark CI**

**File:** `.github/workflows/bench.yml`

```yaml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    name: Run Benchmarks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Vulkan SDK
        run: |
          wget -qO - https://packages.lunarg.com/lunarg-signing-key-pub.asc | sudo apt-key add -
          sudo wget -qO /etc/apt/sources.list.d/lunarg-vulkan-jammy.list \
            https://packages.lunarg.com/vulkan/lunarg-vulkan-jammy.list
          sudo apt update
          sudo apt install vulkan-sdk

      # Cache
      - uses: Swatinem/rust-cache@v2

      # Run benchmarks
      - name: Run benchmarks
        run: cargo bench --workspace -- --save-baseline current

      # Compare with main branch (if PR)
      - name: Compare with baseline
        if: github.event_name == 'pull_request'
        run: |
          git fetch origin main
          git checkout origin/main
          cargo bench --workspace -- --save-baseline main
          git checkout -
          cargo bench --workspace -- --baseline main

      # Fail if regression > 10%
      - name: Check for regressions
        if: github.event_name == 'pull_request'
        run: |
          # Parse Criterion output and fail if any benchmark is >10% slower
          # (Implementation depends on Criterion output format)
          echo "Checking for performance regressions..."
```

---

### **3. Release Workflow**

**File:** `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build Release Binaries
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            artifact: client-linux-x64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            artifact: client-windows-x64.exe
          - os: macos-latest
            target: x86_64-apple-darwin
            artifact: client-macos-x64
          - os: macos-14
            target: aarch64-apple-darwin
            artifact: client-macos-arm64
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      # Install platform dependencies
      - name: Install dependencies (Linux)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt update
          sudo apt install vulkan-sdk libxcb1-dev libx11-dev

      - name: Install dependencies (Windows)
        if: matrix.os == 'windows-latest'
        shell: pwsh
        run: |
          $ProgressPreference = 'SilentlyContinue'
          Invoke-WebRequest -Uri "https://sdk.lunarg.com/sdk/download/latest/windows/vulkan-sdk.exe" -OutFile VulkanSDK.exe
          .\VulkanSDK.exe /S

      - name: Install dependencies (macOS)
        if: startsWith(matrix.os, 'macos')
        run: brew install molten-vk

      # Build
      - name: Build release
        run: cargo build --release --bin client --target ${{ matrix.target }}

      # Upload artifact
      - name: Upload binary
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.artifact }}
          path: target/${{ matrix.target }}/release/client${{ matrix.os == 'windows-latest' && '.exe' || '' }}

  create-release:
    name: Create GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Download all artifacts
      - uses: actions/download-artifact@v3

      # Create release
      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            client-linux-x64/client
            client-windows-x64.exe/client.exe
            client-macos-x64/client
            client-macos-arm64/client
          generate_release_notes: true
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

---

### **4. Docker Build**

**File:** `.github/workflows/docker.yml`

```yaml
name: Docker

on:
  push:
    branches: [main]
    tags:
      - 'v*.*.*'

jobs:
  build-server:
    name: Build Server Image
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ghcr.io/${{ github.repository }}/server
          tags: |
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          file: ./engine/binaries/server/Dockerfile
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

---

### **5. E2E Tests**

**File:** `.github/workflows/e2e.yml`

```yaml
name: E2E Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  e2e:
    name: End-to-End Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Build test images
        run: |
          docker-compose -f tests/e2e/docker-compose.yml build

      - name: Run E2E tests
        run: |
          docker-compose -f tests/e2e/docker-compose.yml up --abort-on-container-exit

      - name: Cleanup
        if: always()
        run: |
          docker-compose -f tests/e2e/docker-compose.yml down -v
```

---

### **6. Dependabot Configuration**

**File:** `.github/dependabot.yml`

```yaml
version: 2
updates:
  # Cargo dependencies
  - package-ecosystem: "cargo"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 10
    reviewers:
      - "maintainers"
    labels:
      - "dependencies"
      - "rust"

  # GitHub Actions
  - package-ecosystem: "github-actions"
    directory: "/"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5
    reviewers:
      - "maintainers"
    labels:
      - "dependencies"
      - "ci"

  # Docker
  - package-ecosystem: "docker"
    directory: "/engine/binaries/server"
    schedule:
      interval: "weekly"
    open-pull-requests-limit: 5
```

---

### **7. PR Template**

**File:** `.github/pull_request_template.md`

```markdown
## Description

<!-- Brief description of changes -->

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Refactoring

## Checklist

- [ ] Code follows [coding standards](../docs/rules/coding-standards.md)
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] All tests pass (`cargo test --all-features`)
- [ ] Added tests for new functionality
- [ ] Documentation updated (rustdoc, .md files)
- [ ] Benchmarks run (if performance-sensitive)
- [ ] No performance regressions (>10%)
- [ ] Tested on all platforms (or CI handles it)

## Related Issues

<!-- Link to related issues: Closes #123 -->

## Screenshots (if applicable)

<!-- Add screenshots for visual changes -->

## Performance Impact

<!-- Describe any performance implications -->

## Breaking Changes

<!-- List any breaking changes and migration path -->
```

---

### **8. Issue Templates**

**File:** `.github/ISSUE_TEMPLATE/bug_report.md`

```markdown
---
name: Bug Report
about: Report a bug or issue
title: '[BUG] '
labels: bug
assignees: ''
---

## Description

<!-- Clear description of the bug -->

## Steps to Reproduce

1.
2.
3.

## Expected Behavior

<!-- What should happen -->

## Actual Behavior

<!-- What actually happens -->

## Environment

- OS: [Windows/Linux/macOS]
- Rust version: [output of `rustc --version`]
- Engine version: [commit hash or tag]
- Graphics card: [GPU model]
- Vulkan version: [if applicable]

## Logs

<!-- Paste relevant logs (use RUST_LOG=trace) -->

```

**File:** `.github/ISSUE_TEMPLATE/feature_request.md`

```markdown
---
name: Feature Request
about: Suggest a new feature
title: '[FEATURE] '
labels: enhancement
assignees: ''
---

## Feature Description

<!-- Clear description of the feature -->

## Use Case

<!-- Why is this feature needed? -->

## Proposed Solution

<!-- How should this be implemented? -->

## Alternatives Considered

<!-- What other approaches were considered? -->

## Additional Context

<!-- Any other relevant information -->
```

---

## ✅ **Acceptance Criteria**

- [ ] CI runs on all platforms (Ubuntu, Windows, macOS x64, macOS ARM64)
- [ ] Format check enforced (`cargo fmt --check`)
- [ ] Clippy lints enforced (`cargo clippy -- -D warnings`)
- [ ] All tests run on all platforms
- [ ] Code coverage tracked (>80% target)
- [ ] Documentation builds without warnings
- [ ] Security audit runs (cargo-audit)
- [ ] Benchmarks run with regression detection
- [ ] Release builds for all platforms
- [ ] Docker images build and push
- [ ] E2E tests run in containers
- [ ] Dependabot configured
- [ ] PR template in place
- [ ] Issue templates created

---

## 🎯 **Quality Gates**

CI must pass ALL checks before merge:

1. **Format** - Code formatted with rustfmt
2. **Lints** - No clippy warnings
3. **Tests** - All unit/integration/doc tests pass
4. **Coverage** - >80% code coverage (or no decrease)
5. **Docs** - Documentation builds without warnings
6. **Security** - No known vulnerabilities
7. **Benchmarks** - No regressions >10%
8. **Platform** - Tests pass on all 4 platforms

---

## 🚀 **CI Performance Targets**

| Check | Target | Critical |
|-------|--------|----------|
| Format check | < 30s | < 1m |
| Clippy (cold cache) | < 5m | < 10m |
| Tests per platform | < 10m | < 20m |
| Coverage | < 15m | < 30m |
| Docs build | < 3m | < 5m |
| Full CI run | < 20m | < 40m |

---

## 💡 **Implementation Notes**

1. **Caching Strategy:**
   - Use `Swatinem/rust-cache@v2` for Cargo cache
   - Cache keyed by OS + Rust version + Cargo.lock hash
   - Separate caches for different jobs

2. **Platform Testing:**
   - All 4 platforms tested on every PR
   - Use `fail-fast: false` to see all platform failures
   - Platform-specific Vulkan SDK installation

3. **Benchmark Comparison:**
   - Save baseline on main branch
   - Compare PR benchmarks against baseline
   - Fail if >10% regression in any benchmark

4. **Security:**
   - Dependabot for automatic dependency updates
   - `cargo-audit` for known vulnerabilities
   - Review security advisories weekly

5. **Release Automation:**
   - Tag with `v*.*.*` triggers release build
   - Builds for all platforms uploaded as assets
   - Docker images tagged with version

---

**Dependencies:** [phase0-repo-setup.md](phase0-repo-setup.md)
**Next:** [phase0-dev-tools.md](phase0-dev-tools.md)
