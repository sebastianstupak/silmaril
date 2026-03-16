# Silmaril

**Work in progress.** A data-driven, server-authoritative game engine designed for AI agent workflows.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.75%2B-orange)](https://www.rust-lang.org/)

---

## What is this?

Silmaril is a game engine built around the idea that AI agents should be able to build, run, and iterate on games without human intervention. That means:

- Programmatic control over everything — no GUI required
- ECS architecture with inspectable/serializable world state
- Server-authoritative multiplayer from day one
- Cross-platform: Windows, Linux, macOS (x64 + ARM64)

The engine is under active development. Most subsystems exist but are not production-ready.

---

## Prerequisites

- Rust 1.75+ ([rustup.rs](https://rustup.rs/))
- Vulkan SDK ([vulkan.lunarg.com](https://vulkan.lunarg.com/))
  - Windows: Install Vulkan SDK
  - Linux: `sudo apt install vulkan-tools libvulkan-dev`
  - macOS: `brew install molten-vk`

## Build

```bash
git clone https://github.com/your-org/silmaril.git
cd silmaril/engine
cargo build
cargo test --all-features
```

---

## Documentation

- [CLAUDE.md](CLAUDE.md) — rules and conventions for AI agents working on this repo
- [ROADMAP.md](ROADMAP.md) — implementation plan and current status
- [docs/architecture.md](docs/architecture.md) — system design
- [docs/ecs.md](docs/ecs.md) — ECS implementation
- [docs/networking.md](docs/networking.md) — client/server architecture
- [docs/rendering.md](docs/rendering.md) — Vulkan renderer
- [docs/error-handling.md](docs/error-handling.md) — error type conventions
- [docs/TESTING_ARCHITECTURE.md](docs/TESTING_ARCHITECTURE.md) — test organization
- [docs/rules/coding-standards.md](docs/rules/coding-standards.md) — coding standards

---

## Repository Layout

```
silmaril/
├── engine/          # Engine crates (core, renderer, networking, physics, audio, ...)
├── examples/        # Example games (singleplayer, mmorpg, moba, turn-based)
├── docs/            # Technical documentation
├── scripts/         # Build and dev utilities
├── benchmarks/      # Benchmark results and baselines
└── tests/           # Integration tests
```

---

## Development

```bash
# Format + lint + test
cargo xtask check

# Run specific test suites
cargo xtask test ecs
cargo xtask test physics

# Benchmarks
cargo xtask bench all
```

See [docs/development-workflow.md](docs/development-workflow.md) for the full workflow.

---

## License

Apache License 2.0 — see [LICENSE](LICENSE).
