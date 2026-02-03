---
name: setup
description: First-time project setup and environment verification
trigger: /setup
---

# Project Setup

Performs first-time setup and verification for silmaril development environment.

## Instructions

1. **Check System Requirements**

   ### Required Tools
   ```bash
   # Check Rust installation
   rustc --version
   cargo --version

   # Check for minimum version (1.70.0+)
   # Suggest: rustup update if outdated

   # Check git
   git --version
   ```

2. **Verify Vulkan SDK** (for client development)

   ```bash
   # Windows
   echo %VK_SDK_PATH%
   dir "%VK_SDK_PATH%\Include\vulkan"

   # Linux/macOS
   echo $VULKAN_SDK
   ls $VULKAN_SDK/include/vulkan

   # Check Vulkan version
   vulkaninfo --summary
   ```

   If Vulkan SDK not found:
   - Windows: Download from https://vulkan.lunarg.com/
   - Linux: `sudo apt install vulkan-sdk` or distro equivalent
   - macOS: Download MoltenVK from https://vulkan.lunarg.com/

3. **Install Rust Dependencies**

   ```bash
   # Install additional targets if needed
   rustup target add x86_64-pc-windows-msvc  # Windows
   rustup target add x86_64-unknown-linux-gnu # Linux
   rustup target add x86_64-apple-darwin      # macOS Intel
   rustup target add aarch64-apple-darwin     # macOS ARM

   # Install development tools
   cargo install cargo-watch    # Auto-rebuild on file changes
   cargo install cargo-bench    # Benchmarking
   cargo install flamegraph     # Performance profiling
   ```

4. **Run Setup Script** (if exists)

   ```bash
   # Check if setup script exists
   if [ -f "scripts/setup.sh" ]; then
       bash scripts/setup.sh
   fi

   # Or Windows equivalent
   if exist "scripts\setup.bat" (
       scripts\setup.bat
   )
   ```

5. **Verify Project Structure**

   Check that essential directories exist:
   ```bash
   # Verify structure
   ls -la
   ls engine/
   ls docs/
   ls scripts/
   ```

   If missing, create according to structure in CLAUDE.md

6. **Install Dependencies**

   ```bash
   # Check Cargo.toml exists
   if [ -f "Cargo.toml" ]; then
       # Fetch dependencies
       cargo fetch

       # Build workspace to verify everything works
       cargo check --workspace --all-features
   else
       echo "No Cargo.toml found - workspace not yet initialized"
       echo "This is expected for Phase 0"
   fi
   ```

7. **Verify Development Tools**

   ```bash
   # Check for recommended tools
   gh --version           # GitHub CLI (for PR reviews)
   docker --version       # Docker (for dev environment)
   code --version         # VS Code (optional)
   ```

8. **Configure Git Hooks** (if exists)

   ```bash
   # Check for pre-commit hooks
   if [ -f ".git/hooks/pre-commit" ]; then
       chmod +x .git/hooks/pre-commit
   fi
   ```

9. **Run Initial Tests**

   If workspace is initialized:
   ```bash
   # Run basic tests to verify setup
   cargo test --workspace --all-features
   ```

10. **Generate Setup Report**

    Create comprehensive report of what's installed and what's missing

## Output Format

```
Project Setup Report
====================

System Information:
-------------------
OS:              Windows 11 / Linux / macOS
Architecture:    x86_64 / aarch64
Rust Version:    1.75.0 ✓
Cargo Version:   1.75.0 ✓

Core Requirements:
------------------
✓ Rust toolchain (1.75.0 >= 1.70.0 required)
✓ Git (2.40.1)
✓ Vulkan SDK (1.3.268)
  - VK_SDK_PATH: C:\VulkanSDK\1.3.268.0
  - Headers found: ✓
  - Libraries found: ✓

Development Tools:
------------------
✓ cargo-watch (8.4.0)
✓ cargo-bench (0.1.0)
✓ flamegraph (0.6.3)
✓ GitHub CLI (2.40.0)
✓ Docker (24.0.7)
☐ VS Code (not found - optional)

Rust Targets:
-------------
✓ x86_64-pc-windows-msvc (installed)
✓ x86_64-unknown-linux-gnu (installed)
✓ x86_64-apple-darwin (installed)
✓ aarch64-apple-darwin (installed)

Project Status:
---------------
✓ Directory structure verified
✓ CLAUDE.md found
✓ ROADMAP.md found
☐ Cargo.toml (not yet created - expected for Phase 0)
☐ Dependencies (workspace not initialized)

Setup Scripts:
--------------
☐ scripts/setup.sh (not found)
☐ scripts/dev.sh (not found)
☐ docker-compose.yml (not found)

Initial Build:
--------------
⏭  Skipped (workspace not yet initialized)
   This is expected during Phase 0 - Documentation

Environment Ready: ✓ READY
----------------------------

Next Steps:
-----------
Your development environment is ready!

Current Phase: Phase 0 - Documentation
To see project status, run: /phase

To start development:
1. Complete Phase 0 tasks (see /phase)
2. Once Cargo.toml is created, run:
   - cargo check --workspace
   - cargo test --workspace
3. Start coding according to ROADMAP.md

For development workflow, see:
  docs/development-workflow.md (when created)

All systems ready for silmaril development!
```

If issues found:

```
Project Setup Report
====================

Issues Found:
-------------
❌ Vulkan SDK not found
   Solution: Download from https://vulkan.lunarg.com/
            Set VK_SDK_PATH environment variable
            Restart terminal after installation

❌ Rust version too old (1.65.0 < 1.70.0 required)
   Solution: rustup update

⚠️  cargo-watch not installed
   Solution: cargo install cargo-watch

Environment Ready: ❌ NOT READY
--------------------------------

Please fix the critical issues (❌) before continuing.
Run /setup again after installing required components.
```

## Setup Validation

### Critical (must have)
- Rust 1.70.0+
- Git
- Vulkan SDK (for client work)

### Recommended (should have)
- cargo-watch
- cargo-bench
- GitHub CLI
- Docker

### Optional (nice to have)
- VS Code
- flamegraph
- Tracy profiler

## Platform-Specific Notes

### Windows
- Vulkan SDK from LunarG
- Visual Studio Build Tools (for Rust MSVC)
- Set VK_SDK_PATH environment variable

### Linux
- Vulkan SDK via package manager
- Build essentials (gcc, make)
- X11/Wayland development libraries

### macOS
- MoltenVK for Vulkan support
- Xcode Command Line Tools
- Homebrew recommended

## Notes

- Run setup after fresh clone
- Re-run if environment issues occur
- Verifies all tools needed for development
- References CLAUDE.md for requirements
- Checks against current ROADMAP.md phase
- Platform-specific verification
