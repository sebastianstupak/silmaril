# Development Scripts

This directory contains scripts for development workflow automation.

## Setup Scripts

### setup-hooks.sh

Installs git pre-commit hooks and configures the development environment.

**Usage:**
```bash
./scripts/setup-hooks.sh
```

**What it does:**
- Installs pre-commit hook to `.git/hooks/pre-commit`
- Makes the hook executable
- Checks for optional development tools
- Displays installation confirmation

**Run this once after cloning the repository.**

## Git Hooks

### hooks/pre-commit

Pre-commit hook that runs automatically before each commit.

**Checks performed:**
1. Code formatting (`cargo fmt --check`)
2. Linting (`cargo clippy --all-targets -- -D warnings`)
3. Unit tests (`cargo test --lib`)
4. Dependency checks (`cargo deny check bans`, if installed)
5. Common issue detection:
   - `println!`/`eprintln!`/`dbg!` usage (should use `tracing` instead)
   - `anyhow::Result` usage (should use custom error types)
   - `Box<dyn Error>` usage (should use custom error types)

**Manual execution:**
```bash
.git/hooks/pre-commit
```

**Bypass (not recommended):**
```bash
git commit --no-verify
```

## Optional Development Tools

The scripts check for these optional tools:

- **cargo-deny**: Dependency auditing and policy enforcement
  ```bash
  cargo install cargo-deny
  ```

- **cargo-watch**: Auto-rebuild on file changes
  ```bash
  cargo install cargo-watch
  ```

- **cargo-flamegraph**: CPU profiling with flamegraphs
  ```bash
  cargo install flamegraph
  ```

## Troubleshooting

### Pre-commit hook fails

If the pre-commit hook fails, read the error messages carefully. They include:
- What check failed
- How to fix it (suggested commands)

Common fixes:
```bash
# Fix formatting
cargo fmt

# Fix clippy issues automatically
cargo clippy --fix --all-targets

# Run tests to see failures
cargo test --lib

# Check dependencies
cargo deny check bans
```

### Hook not running

Verify the hook is installed and executable:
```bash
ls -l .git/hooks/pre-commit
```

If missing, run setup again:
```bash
./scripts/setup-hooks.sh
```

### Permission denied

Make the hook executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Adding New Scripts

When adding new development scripts:

1. Place them in this directory
2. Make them executable: `chmod +x script-name.sh`
3. Add documentation to this README
4. Update `docs/development-workflow.md` if user-facing

## See Also

- [Development Workflow Documentation](../docs/development-workflow.md)
- [Coding Standards](../docs/rules/coding-standards.md)
- [Error Handling Guide](../docs/error-handling.md)
