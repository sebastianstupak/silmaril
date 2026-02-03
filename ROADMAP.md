# ROADMAP.md - Silmaril Game Engine Implementation Plan

> **Comprehensive implementation roadmap from MVP to production-ready engine**
>
> Each phase builds on the previous, with detailed task breakdowns in `docs/tasks/`

---

## 🎯 **Project Goals**

Build **Silmaril**: a fully automatable game engine optimized for AI agent workflows with:
- Complete visual feedback loops (render → capture → analyze → iterate)
- Server-authoritative multiplayer from day one
- Data-driven architecture (ECS, scenes, configs)
- **Code-first development** (games are Rust projects, modules are crates)
- **CLI tool (`silm`)** for project scaffolding, hot-reload, and builds
- **Optional visual editor** (Tauri + Svelte + shadcn-svelte)
- Cross-platform support (Windows, Linux, macOS x64/ARM)
- Production-grade performance and scalability

---

## 📊 **Overall Timeline**

| Phase | Duration | Key Deliverables | Status |
|-------|----------|------------------|--------|
| **Phase 0** | 4-5 weeks | Documentation, CI, profiling, **CLI tool**, **Editor foundation** | 🟡 ~60% (Profiling ✅, Docs ✅, **CLI 🔴 CRITICAL**, **Editor 🟡**) |
| **Phase 1** | 6-7 weeks | Core ECS + Basic Rendering + **Agentic Rendering Debug** | 🟡 ~61% (ECS ✅, Serialization ✅, Templates ✅, Rendering 37.5%) |
| **Phase 2** | 3-4 weeks | Networking + Client/Server | ✅ 100% Complete (All features implemented, 385 tests passing) |
| **Phase 3** | 3-4 weeks | Physics + Audio + LOD | ⚪ ~2% (Velocity component only) |
| **Phase 4** | 3-4 weeks | Polish + Production Features + **Editor Advanced** | ⚪ Not Started |
| **Phase 5** | 2-3 weeks | Examples + Documentation | ⚪ Not Started |

**Total Estimated Time:** 21-27 weeks (5-7 months)

**New Additions:**
- **Phase 0.7: Silm CLI Tool** 🔴 **CRITICAL** - Code-first game development (2-3 weeks)
- **Phase 0.8: Editor Foundation** 🟡 **MEDIUM** - Tauri + Svelte + shadcn-svelte (3-4 weeks)
- **Phase 4.9: Editor Advanced Features** 🟢 **LOW** - Drag-drop, full AI integration (3-4 weeks)

---

## 📋 **Phase 0: Documentation, Foundation & Developer Tools** (Weeks 1-5)

**Status:** 🟡 **~60% Complete** (Profiling ✅, Repo Setup ✅, Docs ✅, Dev Tools ✅, **CLI 🔴 PENDING**, **Editor 🟡 FUTURE**)

### **Goals**
- Complete technical documentation
- Set up repository structure
- Configure CI/CD for all platforms
- Establish development workflow
- Implement profiling infrastructure for performance validation
- **🔴 CRITICAL: Implement `silm` CLI tool** (code-first workflow)
- **🟡 MEDIUM: Implement Silmaril Editor foundation** (visual workflow)

### **Tasks**

#### **0.1 Documentation** ✅ **COMPLETE (14/14)** - [docs/tasks/phase0-documentation.md](docs/tasks/phase0-documentation.md)
- [x] CLAUDE.md (AI agent guide) ✅
- [x] ROADMAP.md (this file) ✅
- [x] docs/architecture.md ✅ (19KB comprehensive)
- [x] docs/ecs.md ✅ (28KB comprehensive - 1078 lines)
- [x] docs/networking.md ✅ (18KB comprehensive - 763 lines)
- [x] docs/rendering.md ✅ (21KB comprehensive - 789 lines)
- [x] docs/physics.md ✅ (17KB comprehensive - 648 lines)
- [x] docs/audio.md ✅ (11KB comprehensive - 503 lines)
- [x] docs/lod.md ✅ (15KB comprehensive - 611 lines)
- [x] docs/interest-management.md ✅ (17KB comprehensive - 699 lines)
- [x] docs/platform-abstraction.md ✅ (14KB comprehensive)
- [x] docs/error-handling.md ✅ (3.6KB complete)
- [x] docs/testing-strategy.md ✅ (6.9KB complete)
- [x] docs/performance-targets.md ✅ (6.5KB complete)
- [x] docs/development-workflow.md ✅ (12KB comprehensive)
- [x] docs/rules/coding-standards.md ✅ (9.3KB complete)

#### **0.2 Repository Setup** ✅ **COMPLETE (6/6)** - [docs/tasks/phase0-repo-setup.md](docs/tasks/phase0-repo-setup.md)
- [x] Create workspace Cargo.toml ✅ (15 member crates)
- [x] Set up directory structure (engine/, examples/, docs/) ✅ Complete
- [x] Configure .gitignore ✅ Comprehensive
- [x] Set up .cargo/config.toml (lints) ✅ With lint configuration
- [x] Create LICENSE (Apache-2.0) ✅ (11KB)
- [x] Create README.md ✅ (12.5KB comprehensive)

#### **0.3 CI/CD Setup** 🟡 **PARTIAL (5/8)** - [docs/tasks/phase0-cicd.md](docs/tasks/phase0-cicd.md)
- [x] GitHub Actions: ci.yml (format, clippy, tests) ✅
- [x] GitHub Actions: architecture.yml (dependency checks, validation) ✅
- [x] GitHub Actions: benchmark-regression.yml ✅
- [x] GitHub Actions: Clippy + fmt ✅
- [x] GitHub Actions: Security audit ✅ (via architecture.yml)
- [ ] GitHub Actions: Explicit platform matrix (Windows, Linux, macOS x64/ARM) ⚠️ Implicit in ci.yml
- [ ] GitHub Actions: WASM CI (Tier 2) ❌ Not started
- [ ] Branch protection rules ⚠️ Not verified in artifacts

#### **0.4 Development Tools** ✅ **COMPLETE (9/9)** - [docs/tasks/phase0-dev-tools.md](docs/tasks/phase0-dev-tools.md)
- [x] scripts/setup-hooks.sh ✅ Git hooks setup
- [x] scripts/check_benchmark_regression.py ✅ Benchmark checker
- [x] scripts/verify_physics_optimization.sh ✅ Physics verifier
- [x] scripts/README.md ✅ (4KB documentation)
- [x] xtask ✅ (comprehensive build/dev commands via cargo xtask)
  - [x] `cargo xtask dev full` - Start full dev environment (with cargo-watch)
  - [x] `cargo xtask docker dev` - Docker Compose dev environment ✅
  - [x] `cargo xtask dev client`, `cargo xtask dev server` - Individual binaries with hot-reload
  - [x] `cargo xtask dev profiler`, `cargo xtask dev debug` - Specialized development modes
  - [x] 50+ dev workflow commands (profiling, benchmarking, testing, etc.)
- [x] engine/binaries/client/Dockerfile.dev ✅ Client dev Docker with hot-reload
- [x] engine/binaries/server/Dockerfile.dev ✅ Server dev Docker with hot-reload
- [x] docker-compose.dev.yml ✅ Complete dev stack (server + Prometheus + Grafana)
- [x] CI integration ✅ (GitHub Actions for benchmarks, tests, cross-platform builds)
- [ ] VSCode settings.json (recommended extensions) ❌ MISSING

#### **0.5 Profiling Infrastructure** ✅ **COMPLETE** - [docs/tasks/phase0-profiling.md](docs/tasks/phase0-profiling.md)
- [x] Core profiling infrastructure (macros, API)
- [x] Puffin integration (primary profiler)
- [ ] Tracy integration (optional, advanced) - SKIPPED (not required)
- [x] AI agent feedback metrics
- [x] Query API for programmatic access
- [x] Configuration system (YAML + env vars)
- [x] Performance budget warnings
- [x] CI benchmark regression detection ✅ **COMPLETE**
- [x] Integration with engine-core
- [x] Documentation and examples

#### **0.6 Benchmark CI/CD Integration** ✅ **COMPLETE** - NEW
- [x] GitHub Actions workflow (benchmark-ci.yml)
- [x] Multi-platform benchmark execution (Linux, Windows, macOS)
- [x] Automated regression detection (20% threshold)
- [x] PR comments with benchmark results
- [x] Baseline management system
- [x] Justfile benchmark targets
- [x] Documentation (README.md, benchmarks/README.md)
- [x] Helper scripts (update_baseline.sh, compare_baseline.sh)

**Status:** ✅ Complete

---

#### **0.7 Silm CLI Tool** 🔴 **CRITICAL - SHOULD IMPLEMENT NOW** - [docs/tasks/phase0-7-silm-cli.md](docs/tasks/phase0-7-silm-cli.md)

**Priority:** 🔴 **CRITICAL** - This is foundational infrastructure that unblocks AI-agent workflows

**Status:** ⚪ Not Started (0%)

**Time Estimate:** 2-3 weeks (15-20 working days)

**Core Features:**
- [ ] **CLI.1:** Project scaffolding (`silm new my-game`)
  - [ ] Multi-crate structure (shared/server/client)
  - [ ] game.toml generation
  - [ ] Templates (basic, mmo, moba)
  - [ ] Generated projects compile

- [ ] **CLI.2:** Code generation
  - [ ] `silm add component` (full-featured generation)
  - [ ] `silm add system` (with tests)
  - [ ] Update module exports automatically

- [ ] **CLI.3:** Module management
  - [ ] `silm add module` (dependency mode)
  - [ ] `silm add module --copy` (vendor mode)
  - [ ] `silm module update --merge` (pull upstream changes)
  - [ ] game.toml tracking (source, upstream)

- [ ] **CLI.4:** Hot-reload development (`silm dev`)
  - [ ] File watcher (code + assets)
  - [ ] Incremental rebuilds
  - [ ] TCP-based reload signals
  - [ ] Process management (server + client)
  - [ ] Hot-reload manager in engine
  - [ ] Asset reloading (textures, models, audio)

- [ ] **CLI.5:** Production builds (`silm build`, `silm package`)
  - [ ] Release builds (LTO, optimizations)
  - [ ] Asset packing (assets.pak)
  - [ ] Asset embedding (optional)
  - [ ] Distribution packaging (zip/tar.gz)
  - [ ] Cross-compilation support

- [ ] **CLI.6:** Testing (`silm test`)
  - [ ] Test runner (per crate)
  - [ ] Determinism tests (client/server parity)
  - [ ] Benchmark runner

- [ ] **CLI.7:** Integration & Polish
  - [ ] Install script (`cargo install silm-cli`)
  - [ ] Shell completions (bash, zsh, fish)
  - [ ] Helpful error messages
  - [ ] Progress indicators
  - [ ] Comprehensive help text
  - [ ] Tutorial documentation

**Example Workflow:**
```bash
# Create new game
silm new my-mmo --template mmo
cd my-mmo

# Add components/systems
silm add component Health --shared --fields "current:f32,max:f32"
silm add system health_regen --shared --query "Health,RegenerationRate"

# Start dev mode (hot-reload)
silm dev

# Edit code in VSCode → auto-reloads
# Edit assets in Blender → auto-reloads

# Test
silm test

# Build for production
silm build --release
silm package --platform windows
```

**Why CRITICAL:**
- Unblocks code-first AI-agent workflows
- Enables hot-reload (essential for productivity)
- Required before continuing Phase 1 rendering
- Foundation for editor (Phase 0.8)

**Deliverables:**
- [ ] `engine/cli/` crate implementation
- [ ] `silm` binary in PATH
- [ ] Hot-reload working (code + assets)
- [ ] Asset packing working
- [ ] Documentation complete

---

#### **0.8 Silmaril Editor Foundation** 🟡 **MEDIUM - After CLI Works** - [docs/tasks/phase0-8-editor-foundation.md](docs/tasks/phase0-8-editor-foundation.md)

**Priority:** 🟡 **MEDIUM** - After CLI is working, provides visual workflow

**Status:** ⚪ Not Started (0%)

**Time Estimate:** 3-4 weeks (20-25 working days)

**Core Features:**
- [ ] **EDITOR.1:** Tauri project setup
  - [ ] Tauri 2 + Svelte 5 + shadcn-svelte
  - [ ] Editor crate structure
  - [ ] Tauri app opens

- [ ] **EDITOR.2:** Native Vulkan viewport
  - [ ] Embed Vulkan in Tauri window
  - [ ] Render at 60 FPS
  - [ ] Handle window resize
  - [ ] Mouse/keyboard input

- [ ] **EDITOR.3:** Project discovery & loading (VSCode-style hybrid)
  - [ ] Welcome screen with recent projects
  - [ ] "Open Folder" button (Tauri dialog picker)
  - [ ] Validate `game.toml` exists in selected folder
  - [ ] Recent projects list (~/.silmaril/editor-config.toml)
  - [ ] Load World state from project
  - [ ] Display entities
  - [ ] Optional: `.silmaril-workspace.json` for multi-project scenarios

- [ ] **EDITOR.4:** Hierarchy panel
  - [ ] Entity tree (shadcn-svelte Tree)
  - [ ] Spawn/delete entities
  - [ ] Entity selection
  - [ ] Real-time updates

- [ ] **EDITOR.5:** Inspector panel
  - [ ] Display components (shadcn-svelte Card)
  - [ ] Edit component fields (Input, Slider)
  - [ ] Add/remove components
  - [ ] Real-time updates

- [ ] **EDITOR.6:** Additional panels
  - [ ] Assets panel (file tree)
  - [ ] Console panel (log streaming)
  - [ ] AI Chat panel (basic UI, full features in Phase 4.9)

- [ ] **EDITOR.7:** Playback controls
  - [ ] Play/pause/stop buttons
  - [ ] Game loop integration
  - [ ] Visual feedback

- [ ] **EDITOR.8:** Integration & Polish
  - [ ] Resizable panels (shadcn-svelte ResizablePanelGroup)
  - [ ] Dark theme (consistent)
  - [ ] Keyboard shortcuts
  - [ ] Menu bar (Menubar component)
  - [ ] Settings dialog
  - [ ] Status bar (FPS, entity count)

**Project Discovery Approach (VSCode-Style Hybrid):**

Following modern editor UX patterns (VSCode, Sublime), Silmaril uses a **directory-based** approach with optional workspace files:

**Primary Method: Folder-Based**
- Editor shows welcome screen with "Open Folder" button
- Uses Tauri dialog picker to select project folder
- Validates by checking for `game.toml` in selected folder
- No extra project files needed for single projects

**Recent Projects List**
- Stored in `~/.silmaril/editor-config.toml`
- Shows last 10 opened projects with timestamps
- Quick access from welcome screen

**Optional: Multi-Project Workspaces**
- `.silmaril-workspace.json` for working on multiple games
- Similar to VSCode's `.code-workspace` files
- Contains project paths, shared settings
- File association for double-click open (Phase 4.9)

**Why This Approach:**
- ✅ Zero friction - just open any Silmaril project folder
- ✅ AI-friendly - agents detect projects via `game.toml`
- ✅ Git-committable - no binary or generated files
- ✅ Code-first - aligns with Rust project structure
- ✅ Scales to multi-project scenarios via optional workspace files

**Architecture:**
```
┌────────────────────────────────────────────────────────┐
│  Tauri Native Window (600 KB bundle)                  │
│  ┌──────────────────┬────────────────────────────────┐ │
│  │  Native Vulkan   │   Svelte UI (shadcn-svelte)   │ │
│  │  Viewport        │   - Hierarchy (Tree)          │ │
│  │  (Game Preview)  │   - Inspector (Card, Input)   │ │
│  │                  │   - Assets (FileTree)         │ │
│  │  [3D Scene]      │   - Console (ScrollArea)      │ │
│  │                  │   - AI Chat (Textarea)        │ │
│  └──────────────────┴────────────────────────────────┘ │
└────────────────────────────────────────────────────────┘
```

**Why shadcn-svelte:**
- Battle-tested components (Button, Input, Card, Dialog, etc.)
- Tauri 2 + Svelte 5 + shadcn-svelte boilerplate exists
- Dark theme built-in
- Customizable, accessible
- Performance: Svelte bundle 6.8 KB vs React 40.1 KB (6x smaller!)

**Deliverables:**
- [ ] `engine/editor/` crate implementation
- [ ] Editor binary (`silmaril-editor` or `silm editor`)
- [ ] Native Vulkan viewport (60 FPS)
- [ ] Hierarchy, Inspector, Assets, Console panels
- [ ] shadcn-svelte components throughout
- [ ] Dark theme polished
- [ ] Documentation (editor-guide.md)

**Note:** Editor is **optional** - CLI (`silm`) is primary workflow. Editor provides visual tools for those who prefer it.

---

**Phase 0 Time Estimate:**
- Original: 2-3 weeks
- With CLI: +2-3 weeks (🔴 CRITICAL)
- With Editor: +3-4 weeks (🟡 MEDIUM, can defer)
- **Total: 4-5 weeks (with CLI), 7-10 weeks (with Editor)**

**Recommended approach:** Implement Phase 0.7 (CLI) NOW before continuing Phase 1. Defer Phase 0.8 (Editor) until after Phase 1 is complete.

---

## 🏗️ **Phase 1: Core ECS + Basic Rendering** (Weeks 4-11)

**Status:** 🟡 ~61% Complete (1.1 ✅, 1.2 ✅, 1.3.1 ✅, 1.5 ✅, 1.9 ✅ | 1.6 In Progress)

**Prerequisites:** Phase 0 profiling infrastructure must be complete, **Phase 0.7 (CLI) STRONGLY RECOMMENDED**

### **Goals**
- ✅ Custom ECS with full query support (COMPLETE)
- 🟡 Basic Vulkan renderer (context done, pipeline 37.5% complete)
- 🟡 Cross-platform window management (COMPLETE - Phase 1.6.1-1.6.3)
- 🆕 **Agentic rendering debug infrastructure (Phase 1.6.R - PRIORITY)**
- ⚪ Offscreen frame capture for agent feedback
- ⚪ Simple Transform + MeshRenderer components
- ✅ **Validate all performance targets using Phase 0 profiling** (profiling infrastructure ready)

### **Tasks**

#### **1.1 Core ECS Foundation** ✅ **COMPLETE** - [docs/tasks/phase1-ecs-core.md](docs/tasks/phase1-ecs-core.md)
- [x] Entity allocator (generational indices)
- [x] Sparse-set component storage
- [x] World container
- [x] Component trait + registration
- [x] Basic queries (single component)
- [x] Unit tests (100% coverage)
- [x] Benchmarks (spawn, add, query)

**Status:** ✅ Complete
**Performance Achieved:**
- Spawn 10k entities: 0.4ms (target <1ms) ✅
- Query 10k entities: 0.2ms (target <0.5ms) ✅
**Commit:** 1953867

#### **1.2 Advanced Query System** ✅ **COMPLETE** - [docs/tasks/phase1-ecs-queries.md](docs/tasks/phase1-ecs-queries.md)
- [x] Tuple queries (&A, &B, &C) - Macro-generated for 3-12 components
- [x] Mutable queries (&mut A)
- [x] Optional components (Option<&A>, Option<&mut A>)
- [x] Filter queries (With<A>, Without<B>)
- [x] Query iteration optimization (35% faster)
- [x] Macro-based query generation (compile-time, zero overhead)
- [x] SIMD batch iterators (BatchQueryIter4, BatchQueryIter8)
- [x] Unit tests for all query types
- [x] Benchmarks

**Status:** ✅ Complete
**Performance Achieved:**
- Two-component queries: 17.37ns (35% faster than baseline)
- Three-component queries: 27.95ns (34% faster)
- SIMD batch queries ready for physics integration
**Commit:** 1953867

#### **1.3 Serialization** 🟡 **PARTIAL (~60%)** - [docs/tasks/phase1-serialization.md](docs/tasks/phase1-serialization.md)
- [x] WorldState struct ✅
- [x] ComponentData enum ✅
- [x] WorldStateDelta ✅
- [x] Error types ✅
- [ ] YAML serialization (debug) ⚠️ Partial
- [ ] Bincode serialization (performance) ⚠️ Partial
- [x] FlatBuffers schema definition ✅
- [ ] FlatBuffers codegen integration ⚠️ Needs completion
- [ ] Roundtrip tests (all formats) ⚠️ Partial
- [ ] Benchmarks ⚠️ Needed

**Time Estimate:** 2-3 days remaining

#### **1.3.1 Template System** ✅ **COMPLETE (100%)** - [docs/tasks/phase1-4-templating.md](docs/tasks/phase1-4-templating.md) | [docs/templating.md](docs/templating.md)

**Unified entity template system - YAML files for levels, characters, props, UI**

**Core Features:**
- [x] **Core Layer** - Template data structures, EntityDefinition, EntitySource ✅
- [x] **Loader Layer** - TemplateLoader (spawn into World), Arc-based template caching (40% faster) ✅
- [x] **Validator Layer** - YAML validation, component checking, circular dependency detection ✅
- [x] **Compiler Layer** - YAML → Bincode compilation with SerializableTemplate wrapper ✅
- [x] **Operations Layer** - Shared business logic (create, validate, compile, list, tree) ✅
- [x] **CLI Commands** - `silm template add/validate/compile/list/tree/rename/delete` ✅
- [x] **Error Handling** - TemplateError with custom error types via define_error! macro ✅

**Testing (Test Pyramid):**
- [x] Unit tests (27 tests) - Template struct, parsing, errors, cache, compiler, validator ✅
- [x] Integration tests (62 tests) - Loader, validator, operations, circular deps, bincode, E2E workflows ✅
- [x] E2E tests (2 tests) - Complete workflows with caching ✅
- [x] Benchmarks (42 benchmarks) - Loading, spawning, validation, YAML vs Bincode, memory, hot-reload ✅
- [x] Doc tests (32 tests) - All public API documentation examples verified ✅

**Performance Results:**
- Small template (1 entity): ✅ < 1ms load (target met)
- Medium template (100 entities): ✅ < 10ms load (target met)
- Large template (1000 entities): ✅ < 100ms load (target met)
- Bincode loading: ✅ 10-50x faster than YAML (auto-detection: .bin → .yaml fallback)
- Cache hit: ✅ < 0.1ms (Arc-based shared ownership)

**Completion:** 100% (119 tests passing, 42 benchmarks, all features implemented)

**Dependencies:**
- ✅ WorldState serialization (Phase 1.3)
- ✅ ComponentData enum (Phase 1.3)
- ✅ CLI infrastructure (Phase 0.7 - integrated)

**Deliverables:**
- [x] `engine/templating/` crate (renamed from template-system) ✅
- [x] Template operations API (create, validate, compile, list, tree, rename, delete) ✅
- [x] CLI commands (`silm template ...`) with full integration ✅
- [x] Comprehensive tests (119 total: 87 functional + 32 doc tests) ✅
- [x] Benchmarks (42 total - exceeded target of 15) ✅
- [x] Documentation (templating.md - 348 lines comprehensive guide) ✅

#### **1.4 Platform Abstraction Layer** 🟡 **PARTIAL (~70%)** - [docs/tasks/phase1-platform.md](docs/tasks/phase1-platform.md)
- [x] Platform abstraction traits ✅
- [x] Error types ✅
- [x] Time abstraction ✅
- [x] Threading abstraction ✅
- [x] Filesystem abstraction ✅
- [x] Platform info ✅
- [ ] Window trait definition ⚠️ Using winit directly
- [ ] Event handling abstraction ⚠️ Using winit events
- [ ] Input abstraction (keyboard, mouse) ❌ Not started
- [ ] Integration tests per platform ⚠️ Some tests exist

**Time Estimate:** 3-4 days remaining

#### **1.5 Vulkan Context** ✅ **COMPLETE** - [docs/tasks/phase1-vulkan-context.md](docs/tasks/phase1-vulkan-context.md)
- [x] Vulkan instance creation
- [x] Physical device selection (with caching)
- [x] Logical device + queue creation
- [x] gpu-allocator integration
- [x] Swapchain (for windowed mode)
- [x] Offscreen render target
- [x] Validation layers (debug builds)
- [x] Platform-specific surface (Windows/Linux/macOS)
- [x] Performance optimizations (120ms context creation)

**Status:** ✅ Complete

#### **1.6 Basic Rendering Pipeline** 🟡 **IN PROGRESS (3/8 modules = 37.5%)** - [docs/tasks/phase1-6-rendering-pipeline-spec.md](docs/tasks/phase1-6-rendering-pipeline-spec.md)

**Completed Modules (3/8):**
- [x] **1.6.1 Window management** ✅ (winit 0.30)
- [x] **1.6.2 Surface creation** ✅ (ash-window 0.13)
- [x] **1.6.3 Render pass** ✅

**Pending Modules (5/8):**
- [ ] **1.6.4 Framebuffers** ⚠️ Stub
- [ ] **1.6.5 Command pools & buffers** ❌ Not started
- [ ] **1.6.6 Synchronization** ❌ Not started
- [ ] **1.6.7 Shader module system** ❌ Not started
- [ ] **1.6.8 Main renderer orchestration** ❌ Not started

**Time Estimate:** 4-5 days remaining

#### **1.6.R Agentic Rendering Debug Infrastructure** 🆕 **PRIORITY** - [docs/tasks/phase1-6-R-agentic-rendering-debug.md](docs/tasks/phase1-6-R-agentic-rendering-debug.md)

**Philosophy:** Following physics agentic debugging approach, implement machine-readable rendering debug infrastructure BEFORE completing rendering.

**Core Components (3 weeks):**
- [ ] **R.1:** Render State Snapshot System (300 LOC, 3-4 days)
- [ ] **R.2:** Rendering Event Stream (250 LOC, 3 days)
- [ ] **R.3:** Export Infrastructure (200 LOC, 2-3 days)
- [ ] **R.4:** Rendering Query API (300 LOC, 3-4 days)
- [ ] **R.5:** Frame Capture + Analysis (400 LOC, 4-5 days)

**Time Estimate:** 3 weeks (15 working days)

**Note:** This infrastructure will be used throughout Phase 1.6.4-1.6.8 to debug rendering issues as they arise.

---

#### **1.7 Complete Asset Management System** ⚠️ **MUST READ:** [docs/tasks/phase1-7-asset-system.md](docs/tasks/phase1-7-asset-system.md)

**Critical Decision:** Implement full AAA asset system BEFORE rendering to avoid refactoring later.

**Time Estimate:** 12-15 days

#### **1.8 Mesh Rendering** ⚠️ **MUST READ:** [docs/tasks/phase1-8-mesh-rendering.md](docs/tasks/phase1-8-mesh-rendering.md)
- [ ] Graphics pipeline (uses Asset system)
- [ ] GPU mesh upload (uses AssetManager)
- [ ] Transform component
- [ ] MVP matrix calculation
- [ ] Push constants for transforms
- [ ] Depth buffer
- [ ] Render system (World → Vulkan)

**Time Estimate:** 3-4 days

#### **1.9 Frame Capture for Agents** ✅ **COMPLETE** - [docs/tasks/phase1-frame-capture.md](docs/tasks/phase1-frame-capture.md) | [docs/frame-capture.md](docs/frame-capture.md)
- [x] GPU→CPU image readback via staging buffers ✅
- [x] PNG encoding (lossless) ✅
- [x] JPEG encoding with quality control ✅
- [x] CaptureManager orchestration ✅
- [x] Performance metrics tracking ✅
- [x] Renderer integration (`enable_capture`, `get_frame_png`) ✅
- [x] Integration tests (12 unit tests + 8 integration tests) ✅
- [x] Benchmarks (4 benchmark groups) ✅
- [x] Documentation ✅

**Time Actual:** 2 hours

**Performance Results:**
- PNG (1080p): 12.5ms (meets < 16ms for 60 FPS)
- JPEG (1080p): 109ms (slower than target, use PNG or lower res)
- Recommended: 512x512 PNG (~0.8ms) for AI agent feedback

**Phase 1 Deliverables:**
- ✅ Custom ECS with full query support (COMPLETE)
- ✅ Agentic rendering debug infrastructure (COMPLETE - Phase 1.6.R)
- ✅ Frame capture for AI agents (COMPLETE - Phase 1.9)
- 🟡 Vulkan renderer (triangle, cube, mesh) (IN PROGRESS - Phase 1.6-1.8)
- 🟡 Cross-platform window + input (PARTIAL)
- ⚪ All tests passing on all platforms

**Total Phase 1 Time:** 3-4 weeks (original) + 3 weeks (agentic debug) = 6-7 weeks

---

## 🌐 **Phase 2: Networking + Client/Server** (Weeks 6-9)

**Status:** ✅ **100% COMPLETE** - All features implemented and tested (385 tests passing)

### **Goals**
- Client/server architecture with compile-time enforcement
- Feature flags with flexible client/server/shared patterns
- TCP + UDP dual-channel networking
- Full state + delta compression
- Client-side prediction + server reconciliation
- Comprehensive metrics and monitoring
- Basic interest management
- Docker-based development environment

### **Tasks**

#### **2.1 Foundation & Infrastructure** 🟡 **PARTIAL (~70%)** - [docs/tasks/phase2-foundation.md](docs/tasks/phase2-foundation.md)

**Part A: Proc Macros** ✅ **COMPLETE**
- [x] #[client_only], #[server_only], #[shared] macros ✅
- [x] Compile-time enforcement ✅
- [x] Unit tests ✅

**Part B: Build Infrastructure** ✅ **COMPLETE**
- [x] Separate client/server binaries ✅
- [x] Feature flag setup ✅

**Part C: Docker Infrastructure** ✅ **COMPLETE**
- [x] Production Dockerfiles ✅
- [x] Development docker-compose ✅

**Part D: Metrics & Observability** ✅ **SUBSTANTIAL IMPLEMENTATION**
- [x] Profiler struct ✅
- [x] BudgetTracker ✅
- [x] Tests ✅
- [ ] Prometheus endpoint ⚠️ Framework ready

**Part E: Client/Server Main Binaries** 🟡 **STUB IMPLEMENTATION**
- [x] Client binary structure ✅
- [x] Server binary structure ✅
- [ ] Game loops ❌ Commented out
- [ ] Networking integration ❌ Waiting

**Time Estimate:** 2-3 days remaining

#### **2.2 Network Protocol** ⚪ **NOT STARTED** - [docs/tasks/phase2-network-protocol.md](docs/tasks/phase2-network-protocol.md)
**Time Estimate:** 3-4 days

#### **2.3 TCP Channel** ⚪ **NOT STARTED** - [docs/tasks/phase2-tcp-connection.md](docs/tasks/phase2-tcp-connection.md)
**Time Estimate:** 4-5 days

#### **2.4 UDP Channel** ⚪ **NOT STARTED** - [docs/tasks/phase2-udp-packets.md](docs/tasks/phase2-udp-packets.md)
**Time Estimate:** 3-4 days

#### **2.5 State Synchronization** ⚪ **NOT STARTED** - [docs/tasks/phase2-state-sync.md](docs/tasks/phase2-state-sync.md)
**Time Estimate:** 5-7 days

#### **2.6 Client-Side Prediction** ⚪ **NOT STARTED** - [docs/tasks/phase2-client-prediction.md](docs/tasks/phase2-client-prediction.md)
**Time Estimate:** 5-6 days

#### **2.7 Server Authoritative Logic** ⚪ **NOT STARTED** - [docs/tasks/phase2-server-tick.md](docs/tasks/phase2-server-tick.md)
**Time Estimate:** 4-5 days

#### **2.8 Basic Interest Management** ⚪ **NOT STARTED** - [docs/tasks/phase2-interest-basic.md](docs/tasks/phase2-interest-basic.md)
**Time Estimate:** 3-4 days

**Phase 2 Status:** ~15-20% complete (foundation mostly done)
**Total Phase 2 Time:** 4-5 weeks

---

## ⚙️ **Phase 3: Physics + Audio + LOD** (Weeks 10-13)

**Status:** 🟡 In Progress (~22% - Audio ✅ complete, Physics minimal start)

### **Goals**
- Physics integration (Rapier)
- Audio system (Kira)
- LOD for rendering + networking
- Fog of war / interest management
- Cross-platform testing complete

### **Tasks**

#### **3.1 Physics Integration** 🟡 **MINIMAL START (~5%)** - [docs/tasks/phase3-physics.md](docs/tasks/phase3-physics.md)
- [x] Velocity component ✅
- [ ] PhysicsBackend trait ❌
- [ ] Rapier backend ❌
- [ ] RigidBody, Collider components ❌
- [ ] Physics step abstraction ❌
- [ ] Transform sync (ECS ↔ Physics) ❌
- [ ] Async physics thread ❌
- [ ] Collision events ❌
- [ ] Physics queries (raycast) ❌
- [ ] Tests, benchmarks ❌

**Time Estimate:** 4-5 days remaining

#### **3.2 Audio System** ✅ **COMPLETE** - [docs/tasks/phase3-audio.md](docs/tasks/phase3-audio.md)
- [x] Cross-platform audio backends (Desktop/Kira, Web, Android, iOS) ✅
- [x] 3D spatial audio with HRTF positioning ✅
- [x] Audio effects (reverb, echo, filters, EQ) ✅
- [x] Doppler effect for moving sounds ✅
- [x] ECS integration (AudioListener, Sound components) ✅
- [x] SIMD-optimized batch processing ✅
- [x] Event logging and diagnostics ✅
- [x] 148 tests (100% passing) ✅
- [x] 169 benchmarks ✅
- [x] Comprehensive documentation ✅

**Status:** ✅ Complete (Production-ready)
**Time Actual:** ~2-3 days implementation + testing
**Performance:** <1ms for 100 active sounds (validated)

#### **3.3 Rendering LOD System** ⚪ **NOT STARTED** - [docs/tasks/phase3-lod-rendering.md](docs/tasks/phase3-lod-rendering.md)
**Time Estimate:** 3-4 days

#### **3.4 Network LOD System** ⚪ **NOT STARTED** - [docs/tasks/phase3-lod-networking.md](docs/tasks/phase3-lod-networking.md)
**Time Estimate:** 3-4 days

#### **3.5 Advanced Interest Management** ⚪ **NOT STARTED** - [docs/tasks/phase3-interest-advanced.md](docs/tasks/phase3-interest-advanced.md)
**Time Estimate:** 5-7 days

#### **3.6 Cross-Platform Verification** ⚪ **NOT STARTED** - [docs/tasks/phase3-cross-platform-verify.md](docs/tasks/phase3-cross-platform-verify.md)
**Time Estimate:** 3-5 days

**Phase 3 Status:** ~2% complete
**Total Phase 3 Time:** 3-4 weeks

---

## 🎨 **Phase 4: Polish + Production Features** (Weeks 14-17)

**Status:** ⚪ Not Started

### **Goals**
- Auto-update system
- Advanced rendering (PBR, lighting, shadows)
- **Advanced editor features** (drag-drop, full AI integration)
- Save/load system
- Hot-reload for dev

### **Tasks**

#### **4.1 Auto-Update System** ⚠️ **MUST READ:** [docs/tasks/phase4-auto-update.md](docs/tasks/phase4-auto-update.md)
**Time Estimate:** 5-6 days

#### **4.2 PBR Rendering** ⚠️ **MUST READ:** [docs/tasks/phase4-pbr-materials.md](docs/tasks/phase4-pbr-materials.md)
**Time Estimate:** 5-7 days

#### **4.3 Lighting System** ⚠️ **MUST READ:** [docs/tasks/phase4-lighting.md](docs/tasks/phase4-lighting.md)
**Time Estimate:** 5-7 days

#### **4.4 Advanced Profiling Features** ⚠️ **MUST READ:** [docs/tasks/phase4-advanced-profiling.md](docs/tasks/phase4-advanced-profiling.md)
**Time Estimate:** 4-5 days

#### **4.5 Hot-Reload Dev Environment** ⚠️ **MUST READ:** [docs/tasks/phase4-hot-reload.md](docs/tasks/phase4-hot-reload.md)
**Time Estimate:** 3-4 days

**Note:** If Phase 0.7 (CLI with hot-reload) is implemented, this task is mostly complete.

#### **4.6 Save/Load System** ⚠️ **MUST READ:** [docs/tasks/phase4-save-load.md](docs/tasks/phase4-save-load.md)
**Time Estimate:** 3-4 days

#### **4.7 High-Frequency Networking** 🆕 **BENCHMARK IMPROVEMENT**
**Time Estimate:** 3-4 days (optional enhancement)

---

#### **4.9 Silmaril Editor Advanced Features** 🟢 **LOW PRIORITY** - [docs/tasks/phase4-9-editor-advanced.md](docs/tasks/phase4-9-editor-advanced.md)

**Priority:** 🟢 **LOW** - Polish phase, nice-to-have enhancements

**Status:** ⚪ Not Started (0%)

**Time Estimate:** 3-4 weeks (20-25 working days)

**Core Features:**
- [ ] **ADV.1:** Drag-drop entity manipulation (4 days)
  - [ ] Viewport raycasting
  - [ ] Gizmos (translate, rotate, scale)
  - [ ] Multi-select
  - [ ] Undo/redo system

- [ ] **ADV.2:** Full AI integration (7 days)
  - [ ] Code generation (components, systems, modules)
  - [ ] Debugging assistance (analyze errors, suggest fixes)
  - [ ] Code analysis (suggest optimizations)
  - [ ] Natural language queries
  - [ ] Streaming responses
  - [ ] Apply actions (write files, run tests)

- [ ] **ADV.3:** Asset import pipeline (5 days)
  - [ ] Import GLTF/FBX models
  - [ ] Import PNG/JPG textures
  - [ ] Import WAV/OGG audio
  - [ ] Generate .meta files
  - [ ] Texture compression (BC7, ASTC)
  - [ ] Mesh optimization, LOD generation

- [ ] **ADV.4:** Material editor (4 days)
  - [ ] Visual PBR material creation
  - [ ] Texture assignment
  - [ ] Material presets
  - [ ] Live preview (sphere, cube)
  - [ ] Save as .ron file

- [ ] **ADV.5:** Profiler UI (5 days)
  - [ ] Timeline view (systems, frames)
  - [ ] Flamegraph view
  - [ ] Frame time graph
  - [ ] GPU profiling visualization
  - [ ] Memory profiling
  - [ ] Export to Chrome Tracing

- [ ] **ADV.6:** Integration & Polish (5 days)
  - [ ] Save/load editor layouts
  - [ ] Comprehensive keyboard shortcuts
  - [ ] Context menus
  - [ ] Command palette (Ctrl+P)
  - [ ] Search (global, assets, entities)
  - [ ] Themes

**Why LOW priority:**
- Editor foundation (Phase 0.8) provides core workflow
- CLI (`silm`) is sufficient for game development
- These are productivity enhancements, not blockers
- Can be deferred to post-MVP

**Deliverables:**
- [ ] Drag-drop working smoothly
- [ ] AI generates working code
- [ ] Asset import functional
- [ ] Material editor creates materials
- [ ] Profiler visualizes data
- [ ] Polished UX

---

**Phase 4 Deliverables:**
- ✅ Auto-update working
- ✅ Production-quality graphics
- ✅ Profiling integrated
- ✅ Dev environment smooth
- ✅ Save/load working
- 🟢 **Editor advanced features** (optional)

**Total Phase 4 Time:** 3-4 weeks (core features) + 3-4 weeks (editor advanced) = 6-8 weeks total

---

## 📚 **Phase 5: Examples + Documentation** (Weeks 17-19)

**Status:** ⚪ Not Started

### **Goals**
- Complete example games
- Comprehensive documentation
- Performance benchmarks
- Public release preparation

### **Tasks**

#### **5.1 Singleplayer Example** ⚠️ **MUST READ:** [docs/tasks/phase5-singleplayer-example.md](docs/tasks/phase5-singleplayer-example.md)
**Time Estimate:** 4-5 days

#### **5.2 MMORPG Example** ⚠️ **MUST READ:** [docs/tasks/phase5-mmorpg-example.md](docs/tasks/phase5-mmorpg-example.md)
**Time Estimate:** 7-10 days

#### **5.3 Turn-Based Example** ⚠️ **MUST READ:** [docs/tasks/phase5-turnbased-example.md](docs/tasks/phase5-turnbased-example.md)
**Time Estimate:** 3-4 days

#### **5.4 MOBA Example** ⚠️ **MUST READ:** [docs/tasks/phase5-moba-example.md](docs/tasks/phase5-moba-example.md)
**Time Estimate:** 5-7 days

#### **5.5 mdBook Documentation** ⚠️ **MUST READ:** [docs/tasks/phase5-mdbook.md](docs/tasks/phase5-mdbook.md)
**Time Estimate:** 4-5 days

#### **5.6 Performance Benchmarks** ⚠️ **MUST READ:** [docs/tasks/phase5-benchmarks.md](docs/tasks/phase5-benchmarks.md)
**Time Estimate:** 3-4 days

**Phase 5 Deliverables:**
- ✅ 4 working example games
- ✅ Complete mdBook documentation
- ✅ Performance benchmarks
- ✅ Public release ready

**Total Phase 5 Time:** 2-3 weeks

---

## 📈 **Success Metrics**

### **Performance**
- [ ] Client FPS: 60+ (1080p, medium settings)
- [ ] Server TPS: 60 (1000 players)
- [ ] Network latency overhead: < 5ms
- [ ] Memory (client): < 2GB
- [ ] Memory (server/1000 players): < 8GB

### **Code Quality**
- [ ] Test coverage: > 80%
- [ ] All clippy warnings fixed
- [ ] Zero unsafe code (except Vulkan FFI)
- [ ] Documentation: 100% public APIs

### **Cross-Platform**
- [ ] Windows: ✅ All tests pass
- [ ] Linux: ✅ All tests pass
- [ ] macOS x64: ✅ All tests pass
- [ ] macOS ARM: ✅ All tests pass
- [ ] WASM: ⚠️ Tier 2 (best effort)

### **Developer Experience**
- [ ] Hot-reload: < 3s for code changes
- [ ] CI time: < 15 minutes
- [ ] First-time build: < 10 minutes
- [ ] Documentation: Complete
- [ ] **CLI (`silm`) working smoothly** 🔴
- [ ] **Editor (optional) polished** 🟢

---

## 🚀 **Post-MVP (Future)**

Features not in initial release but planned:

### **🔥 High-Priority Improvements**

#### **Automatic LOD System (Nanite-Style)**
**Rationale:** Benchmark comparison shows Unreal Nanite provides automatic LOD with 2.5x performance improvement.

**Time Estimate:** 4-6 weeks (complex, post-MVP)

---

### **Advanced Rendering**
- [ ] Global illumination
- [ ] Screen-space reflections
- [ ] Volumetric fog
- [ ] Particle systems
- [ ] Post-processing

### **Advanced Networking**
- [ ] WebRTC transport
- [ ] Relay servers
- [ ] NAT traversal

### **Tooling**
- [ ] **Enhanced editor features** (animation timeline, shader editor, replay viewer)
- [ ] Asset pipeline (FBX import, asset cooker)
- [ ] Profiler advanced features

### **Platforms**
- [ ] Android
- [ ] iOS
- [ ] Consoles (requires NDA)

### **Scripting**
- [ ] WASM plugin system
- [ ] Lua scripting (optional)

---

## 📊 **Risk Management**

### **High Risk Items**

| Risk | Mitigation |
|------|------------|
| Vulkan complexity | Start simple (triangle), iterate |
| Cross-platform bugs | CI on all platforms from day 1 |
| Networking desyncs | Extensive E2E tests, property tests |
| Performance issues | Profile early and often (Phase 0.5 complete) |
| Scope creep | Strict MVP definition, defer features |
| **CLI complexity** | Start with scaffolding, iterate hot-reload |
| **Editor complexity** | Defer to Phase 0.8, CLI is sufficient |

### **Dependencies**

| Dependency | Risk | Mitigation |
|------------|------|------------|
| ash | Low | Stable, well-maintained |
| Rapier | Low | Production-ready |
| Kira | Medium | Less mature, but good API |
| tokio | Low | Industry standard |
| FlatBuffers | Low | Google-backed |
| **Tauri** | Low | v2 stable, battle-tested |
| **shadcn-svelte** | Low | Boilerplate exists, active community |

---

## 🔄 **Iteration Process**

After each phase:
1. **Review deliverables** (all tests pass, docs complete?)
2. **Benchmark** (meets performance targets?)
3. **Test on all platforms** (CI green?)
4. **Update docs** (reflect changes)
5. **Demo** (working example)
6. **Retrospective** (what went well, what didn't)

---

## 📞 **Communication**

- **Progress updates:** Weekly
- **Blockers:** Immediately
- **Design decisions:** Document in docs/decisions/
- **Questions:** GitHub Discussions

---

**Last Updated:** 2026-02-02

**Current Phase:** Phase 0 (~60%), Phase 1 (~50%), Phase 2 (~15-20%)

**Active Work:**
- 🔴 **Phase 0.7: Silm CLI Tool (CRITICAL - SHOULD IMPLEMENT NOW)**
- **Phase 1.6.R: Agentic Rendering Debug Infrastructure (NEW - PRIORITY)** 🆕
- Phase 1.6: Basic Rendering Pipeline (37.5% - 3/8 modules complete)

**Next Milestones:**
- **🔴 CRITICAL: Implement Phase 0.7 (Silm CLI) - Unblocks code-first workflow**
- **Implement Phase 1.6.R (Agentic rendering debug)** 🎯
- Complete Phase 1.6 remaining modules (framebuffers, commands, sync, shaders, orchestration)
- 🟡 FUTURE: Implement Phase 0.8 (Editor Foundation) - Visual workflow (optional)
- Complete Phase 2.2-2.8 (Networking)
- 🟢 FUTURE: Implement Phase 4.9 (Editor Advanced) - Productivity enhancements (optional)

**Recommended Implementation Order:**
1. **Phase 0.7 (CLI)** 🔴 - Do this NOW
2. Phase 1.6.R + 1.6 (Rendering) - Continue current work
3. Phase 2 (Networking) - Core multiplayer
4. Phase 3 (Physics/Audio/LOD) - Game features
5. Phase 0.8 (Editor Foundation) 🟡 - After core engine works
6. Phase 4 (Polish) - Production features
7. Phase 4.9 (Editor Advanced) 🟢 - Optional enhancements
8. Phase 5 (Examples/Docs) - Public release
