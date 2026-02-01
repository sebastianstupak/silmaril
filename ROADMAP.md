# ROADMAP.md - Agent Game Engine Implementation Plan

> **Comprehensive implementation roadmap from MVP to production-ready engine**
>
> Each phase builds on the previous, with detailed task breakdowns in `docs/tasks/`

---

## 🎯 **Project Goals**

Build a fully automatable game engine optimized for AI agents with:
- Complete visual feedback loops (render → capture → analyze → iterate)
- Server-authoritative multiplayer from day one
- Data-driven architecture (ECS, scenes, configs)
- Cross-platform support (Windows, Linux, macOS x64/ARM)
- Production-grade performance and scalability

---

## 📊 **Overall Timeline**

| Phase | Duration | Key Deliverables | Status |
|-------|----------|------------------|--------|
| **Phase 0** | 2-3 weeks | Documentation, project structure, **profiling** | 🟡 ~80% Complete |
| **Phase 1** | 3-4 weeks | Core ECS + Basic Rendering | 🟡 ~65% Complete (ECS ✅, Rendering 37.5%) |
| **Phase 2** | 3-4 weeks | Networking + Client/Server | 🟡 ~15-20% (Foundation done, networking not started) |
| **Phase 3** | 3-4 weeks | Physics + Audio + LOD | ⚪ ~2% (Velocity component only) |
| **Phase 4** | 2-3 weeks | Polish + Production Features | ⚪ Not Started |
| **Phase 5** | 2-3 weeks | Examples + Documentation | ⚪ Not Started |

**Total Estimated Time:** 14-21 weeks (3.5-5 months)

**Key Change:** Profiling infrastructure moved from Phase 4 to Phase 0. This enables performance validation from day one.

---

## 📋 **Phase 0: Documentation & Foundation** (Weeks 1-3)

**Status:** 🟡 ~80% Complete (Profiling ✅, Repo Setup ✅, Docs Partial, CI Partial, Dev Tools Partial)

### **Goals**
- Complete technical documentation
- Set up repository structure
- Configure CI/CD for all platforms
- Establish development workflow
- **Implement profiling infrastructure for performance validation**

### **Tasks**

#### **0.1 Documentation** 🟡 **PARTIAL (8/14)** - [docs/tasks/phase0-documentation.md](docs/tasks/phase0-documentation.md)
- [x] CLAUDE.md (AI agent guide)
- [x] ROADMAP.md (this file)
- [x] docs/architecture.md ✅ (19KB comprehensive)
- [ ] docs/ecs.md ❌ MISSING
- [ ] docs/networking.md ❌ MISSING
- [ ] docs/rendering.md ❌ MISSING
- [ ] docs/physics.md ❌ MISSING
- [ ] docs/audio.md ❌ MISSING (not in original list)
- [ ] docs/lod.md ❌ MISSING (not in original list)
- [ ] docs/interest-management.md ❌ MISSING (not in original list)
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

#### **0.4 Development Tools** 🟡 **PARTIAL (3/9)** - [docs/tasks/phase0-dev-tools.md](docs/tasks/phase0-dev-tools.md)
- [x] scripts/setup-hooks.sh ✅ Git hooks setup
- [x] scripts/check_benchmark_regression.py ✅ Benchmark checker
- [x] scripts/verify_physics_optimization.sh ✅ Physics verifier
- [x] scripts/README.md ✅ (4KB documentation)
- [ ] scripts/dev.sh (start dev environment) ❌ MISSING
- [ ] scripts/test-all-platforms.sh ❌ MISSING
- [ ] docker/Dockerfile.base-client ❌ MISSING (client/server Dockerfiles exist in binaries/)
- [ ] docker/Dockerfile.base-server ❌ MISSING (client/server Dockerfiles exist in binaries/)
- [ ] docker-compose.dev.yml ❌ MISSING (example composes exist)
- [ ] VSCode settings.json (recommended extensions) ❌ MISSING

#### **0.5 Profiling Infrastructure** ✅ **COMPLETE** - [docs/tasks/phase0-profiling.md](docs/tasks/phase0-profiling.md)
- [x] Core profiling infrastructure (macros, API)
- [x] Puffin integration (primary profiler)
- [ ] Tracy integration (optional, advanced) - SKIPPED (not required)
- [x] AI agent feedback metrics
- [x] Query API for programmatic access
- [x] Configuration system (YAML + env vars)
- [x] Performance budget warnings
- [x] CI benchmark regression detection (benchmarks ready, CI workflow pending)
- [x] Integration with engine-core
- [x] Documentation and examples

**Time Estimate:** 8.5-9.5 days (~2 weeks)
**Actual Time:** ~8.5 days
**Status:** ✅ Complete
**Completion Report:** [PHASE_0_5_PROFILING_COMPLETE.md](PHASE_0_5_PROFILING_COMPLETE.md)

**Rationale:** Profiling infrastructure MUST be in Phase 0 to enable performance validation throughout all later phases. You can't fix what you don't measure. This enables:
- Validating Phase 1 ECS performance targets in real-time
- Profiling rendering performance as it's built
- Catching performance regressions immediately
- AI agent feedback loops from day one

**Deliverables:**
- ✅ Complete documentation structure
- ✅ CI/CD passing on all Tier 1 platforms
- ✅ Dev environment working locally
- ✅ Profiling infrastructure ready (zero overhead in release) ✅ **COMPLETE**

---

## 🏗️ **Phase 1: Core ECS + Basic Rendering** (Weeks 4-8)

**Status:** 🟡 In Progress (1.1, 1.2, 1.5 Complete | 1.6 Next)

**Prerequisites:** Phase 0 profiling infrastructure must be complete

### **Goals**
- ✅ Custom ECS with full query support (COMPLETE)
- 🟡 Basic Vulkan renderer (context done, pipeline in progress)
- ⚪ Cross-platform window management (Phase 1.6)
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
- [x] WorldState struct ✅ (in engine/core/src/serialization/world_state.rs)
- [x] ComponentData enum ✅ (in engine/core/src/serialization/component_data.rs)
- [x] WorldStateDelta ✅ (delta compression structure)
- [x] Error types ✅ (SerializationError with define_error!)
- [ ] YAML serialization (debug) ⚠️ Partial
- [ ] Bincode serialization (performance) ⚠️ Partial
- [x] FlatBuffers schema definition ✅ Structure defined
- [ ] FlatBuffers codegen integration ⚠️ Needs completion
- [ ] Roundtrip tests (all formats) ⚠️ Partial
- [ ] Benchmarks ⚠️ Needed

**Time Estimate:** 2-3 days remaining
**Tests:** Property-based roundtrip tests needed
**Performance Target:**
- Serialize 1000 entities < 5ms (bincode)

#### **1.4 Platform Abstraction Layer** 🟡 **PARTIAL (~70%)** - [docs/tasks/phase1-platform.md](docs/tasks/phase1-platform.md)
- [x] Platform abstraction traits ✅ (TimeBackend, FileSystemBackend, ThreadingBackend)
- [x] Error types ✅ (PlatformError enum)
- [x] Time abstraction ✅ Trait complete (Windows/Unix implementations partial)
- [x] Threading abstraction ✅ Trait complete (Windows/Unix implementations partial)
- [x] Filesystem abstraction ✅ Trait complete (native implementation partial)
- [x] Platform info ✅ Partial implementation
- [ ] Window trait definition ⚠️ Using winit directly (see Phase 1.6)
- [ ] Event handling abstraction ⚠️ Using winit events (see Phase 1.6)
- [ ] Input abstraction (keyboard, mouse) ❌ Not started
- [ ] Integration tests per platform ⚠️ Some tests exist

**Time Estimate:** 3-4 days remaining
**Tests:** Platform-specific integration tests needed
**CI:** Must pass on all platforms
**Note:** Window abstraction integrated into Phase 1.6 (winit 0.30)

#### **1.5 Vulkan Context** ✅ **COMPLETE** - [docs/tasks/phase1-vulkan-context.md](docs/tasks/phase1-vulkan-context.md)
- [x] Vulkan instance creation
- [x] Physical device selection (with caching)
- [x] Logical device + queue creation
- [x] gpu-allocator integration
- [x] Swapchain (for windowed mode)
- [x] Offscreen render target
- [x] Validation layers (debug builds)
- [x] Platform-specific surface (Windows/Linux/macOS)
- [x] Performance optimizations (120ms context creation - industry standard)

**Status:** ✅ Complete (See [docs/PHASE1.5-COMPLETE.md](docs/PHASE1.5-COMPLETE.md))
**Performance:** 120ms context creation (industry standard)
**Commit:** 1953867

#### **1.6 Basic Rendering Pipeline** 🟡 **IN PROGRESS (3/8 modules = 37.5%)** - [docs/tasks/phase1-6-rendering-pipeline-spec.md](docs/tasks/phase1-6-rendering-pipeline-spec.md)

**Research-Driven Specification:**
- Industry-validated tech stack (winit + ash-window + raw-window-handle)
- Production patterns from Bevy, Rerun, egui projects
- Build-time GLSL → SPIR-V compilation via shaderc
- Frames in flight synchronization pattern (2 frames)

**Completed Modules (3/8):**
- [x] **1.6.1 Window management** ✅ (winit 0.30)
  - [x] Create cross-platform window (252 lines, 5 tests passing)
  - [x] Handle events and resize
  - [x] Provide raw window/display handles
  - [x] Benchmarks: 56ms creation, ~50ns queries (OPTIMAL)

- [x] **1.6.2 Surface creation** ✅ (ash-window 0.13)
  - [x] Platform-specific Vulkan surface (166 lines, 1 test)
  - [x] Surface capability queries
  - [x] Present mode selection

- [x] **1.6.3 Render pass** ✅
  - [x] Color attachment configuration (191 lines, 1 test)
  - [x] Subpass dependencies
  - [x] Clear color operation
  - [x] Optional depth attachment support

**Pending Modules (5/8):**
- [ ] **1.6.4 Framebuffers** ⚠️ Stub (171 lines structure defined)
  - [ ] One per swapchain image
  - [ ] Automatic recreation on resize

- [ ] **1.6.5 Command pools & buffers** ❌ Not started
  - [ ] Allocate PRIMARY buffers
  - [ ] Record render commands
  - [ ] Submit to graphics queue

- [ ] **1.6.6 Synchronization** ❌ Not started
  - [ ] Image acquisition sync (semaphores)
  - [ ] Render completion sync (fences)
  - [ ] Frames in flight management (2 concurrent)

- [ ] **1.6.7 Shader module system** ❌ Not started
  - [ ] Build-time GLSL compilation (build.rs + shaderc)
  - [ ] SPIR-V module creation
  - [ ] Simple test shaders (hardcoded triangle)

- [ ] **1.6.8 Main renderer orchestration** ❌ Not started
  - [ ] Integrate all components
  - [ ] Render loop at 60 FPS
  - [ ] Handle window resize gracefully

**Time Estimate:** 4-5 days remaining (research complete, TDD implementation)
**Tests:** 14/14 passing for completed modules (100% success rate)
**Output:** Window with clear color at 60 FPS
**Dependencies:** winit, ash-window, raw-window-handle, shaderc (build)
**Checkpoint:** [docs/PHASE1.6-CHECKPOINT-DAY3.md](docs/PHASE1.6-CHECKPOINT-DAY3.md)

#### **1.7 Mesh Rendering** ⚠️ **MUST READ:** [docs/tasks/phase1-mesh-rendering.md](docs/tasks/phase1-mesh-rendering.md)
- [ ] Mesh struct (vertices, indices)
- [ ] OBJ file loader
- [ ] MeshRenderer component
- [ ] Transform component (position, rotation, scale)
- [ ] MVP matrix calculation
- [ ] Push constants for transforms
- [ ] Depth buffer
- [ ] Render system (World → Vulkan)

**Time Estimate:** 4-5 days
**Tests:** E2E test (render cube, check output)
**Output:** Rotating cube

#### **1.8 Frame Capture for Agents** ⚠️ **MUST READ:** [docs/tasks/phase1-frame-capture.md](docs/tasks/phase1-frame-capture.md)
- [ ] Offscreen image → buffer copy
- [ ] Image format conversion (RGBA8)
- [ ] RenderResult struct (color, depth, metrics)
- [ ] Performance metrics collection
- [ ] Opt-in capture fields
- [ ] Memory-efficient capture (zero-copy where possible)

**Time Estimate:** 2-3 days
**Tests:** Benchmark (capture overhead < 2ms)
**API:** `engine.render(capture=["color", "metrics"])`

**Phase 1 Deliverables:**
- ✅ Custom ECS with full query support
- ✅ Vulkan renderer (triangle, cube, mesh)
- ✅ Cross-platform window + input
- ✅ Frame capture working
- ✅ All tests passing on all platforms

**Total Phase 1 Time:** 3-4 weeks

---

## 🌐 **Phase 2: Networking + Client/Server** (Weeks 6-9)

**Status:** ⚪ Not Started

### **Goals**
- Client/server architecture with compile-time enforcement
- Feature flags with flexible client/server/shared patterns
- TCP + UDP dual-channel networking
- Full state + delta compression with property-based validation
- Client-side prediction + server reconciliation
- Comprehensive metrics and monitoring (built-in)
- Basic interest management
- Docker-based development environment
- Production-ready containers

### **Architecture Decisions** (Phase 2 Planning Session)

#### **1. Feature Flags & Code Splitting**
- **Pattern:** `#[client_only]`, `#[server_only]`, `#[shared]`, `#[server_authoritative]`
- **Flexibility:** Code can be shared between client/server with different implementations
- **Validation:** Property-based tests ensure client/server parity
- **Build System:** Separate binaries with feature flags (`cargo build --bin client/server`)

#### **2. Module Structure** (Future Phases)
Three new optional modules for scaling:
- `engine/persistence` - Database, Redis, caching (Phase 3-4)
- `engine/infrastructure` - Config, secrets, service discovery, health checks (Phase 3-4)
- `engine/scaling` - Sharding, load balancing, replication (Phase 4-5)

#### **3. Testing Strategy**
- **Comprehensive:** Unit + Integration + Property + Benchmarks (like Phase 1.4)
- **Validation:** Property tests for client/server prediction parity
- **Performance:** Benchmark every critical path (< 1ms per network operation)
- **Coverage:** AAA quality requires >80% test coverage

#### **4. Metrics & Monitoring** (Day One)
- **Built-in:** Prometheus endpoint on port 8080 (optional, can disable)
- **AI-Friendly:** Comprehensive metrics for debugging (TPS, latency, entity counts, errors)
- **Console:** Basic telnet admin console (localhost-only, no auth for Phase 2)
- **Dashboard:** Advanced web UI deferred to Phase 4

#### **5. Container Infrastructure**
- **Development:** docker-compose with hot-reload support
- **Production:** Multi-stage Dockerfiles (small images ~50MB)
- **Registry:** Local-only for Phase 2 (Docker Desktop)
- **Orchestration:** Kubernetes deferred to Phase 4

### **Tasks**

#### **2.1 Foundation & Infrastructure** 🟡 **PARTIAL (~70%)** - [docs/tasks/phase2-foundation.md](docs/tasks/phase2-foundation.md)

**Part A: Proc Macros** ✅ **COMPLETE (Commit: dded124)**
- [x] #[client_only] attribute macro ✅
- [x] #[server_only] attribute macro ✅
- [x] #[shared] attribute macro ✅
- [x] #[server_authoritative] attribute macro ✅
- [x] define_error! macro ✅ (error handling)
- [x] Compile-time enforcement ✅
- [x] Unit tests for all macros ✅ PASSING
- [x] Documentation with examples ✅

**Part B: Build Infrastructure** ✅ **COMPLETE (Commit: 7b996e9)**
- [x] Separate client/server binaries in `engine/binaries/` ✅
- [x] Feature flag setup (client, server, networking, all) ✅
- [x] Separate build profiles ✅
- [ ] CI matrix for both builds ⚠️ Needs update
- [ ] Cross-compilation verification ⚠️ Partial

**Part C: Docker Infrastructure** ✅ **COMPLETE**
- [x] Production Dockerfiles (multi-stage) ✅ client/Dockerfile, server/Dockerfile
- [x] Docker networking (client ↔ server communication) ✅
- [x] Development docker-compose files ✅ (in examples/mmorpg, examples/moba)
- [ ] One-command dev environment (`./scripts/dev.sh`) ❌ Not created yet
- [ ] Hot-reload support (cargo-watch integration) ⚠️ Partial

**Part D: Metrics & Observability** ✅ **SUBSTANTIAL IMPLEMENTATION**
- [x] Profiler struct with scope-based timing ✅ (engine/observability/)
- [x] BudgetTracker for performance budgets ✅
- [x] ScopeGuard RAII pattern ✅
- [x] Budget violation detection ✅
- [x] Configuration via ProfilerConfig ✅
- [x] Tests: 8 comprehensive unit tests ✅ PASSING
- [ ] Prometheus metrics endpoint ⚠️ Framework ready, endpoint not exposed
- [ ] Core metrics collection ⚠️ Partial (budgets tracked, not exposed)
- [ ] Network metrics ❌ Waiting for Phase 2.2+
- [ ] Basic admin console ❌ Not started

**Part E: Client/Server Main Binaries** 🟡 **STUB IMPLEMENTATION**
- [x] Client binary structure ✅ Compiles with logging setup
- [x] Server binary structure ✅ Compiles with async/tokio + Ctrl+C handling
- [ ] Client game loop implementation ❌ Commented out
- [ ] Server tick loop implementation ❌ Commented out
- [ ] Networking integration ❌ Waiting for Phase 2.2+

**Time Estimate:** 2-3 days remaining (complete binaries + Prometheus endpoint)
**Deliverables:**
- ✅ Macros enforce client/server separation (DONE)
- ✅ `cargo build --bin client/server` works (DONE)
- ⚠️ `./scripts/dev.sh` starts complete environment (script not created)
- ⚠️ Metrics visible at `http://localhost:8080/metrics` (framework ready, endpoint not exposed)

#### **2.2 Network Protocol** ⚪ **NOT STARTED** - [docs/tasks/phase2-network-protocol.md](docs/tasks/phase2-network-protocol.md)
- [ ] Message enum (ClientMessage, ServerMessage) ❌
- [ ] FlatBuffers schema for messages ❌
- [ ] Packet framing (length prefix) ❌
- [ ] Serialization/deserialization ❌
- [ ] Protocol versioning ❌
- [ ] Tests (roundtrip) ❌

**Status:** Placeholder only (engine/networking/src/lib.rs = 336 bytes)
**Time Estimate:** 3-4 days

#### **2.3 TCP Channel** ⚪ **NOT STARTED** - [docs/tasks/phase2-tcp-connection.md](docs/tasks/phase2-tcp-connection.md)
- [ ] Async TCP server (tokio) ❌
- [ ] Async TCP client ❌
- [ ] Connection management ❌
- [ ] Reliable message delivery ❌
- [ ] Heartbeat/keepalive ❌
- [ ] Graceful disconnect ❌
- [ ] Integration tests ❌

**Status:** Not implemented (networking crate is stub)
**Time Estimate:** 4-5 days

#### **2.4 UDP Channel** ⚪ **NOT STARTED** - [docs/tasks/phase2-udp-packets.md](docs/tasks/phase2-udp-packets.md)
- [ ] UDP socket (unreliable) ❌
- [ ] Packet sequence numbers ❌
- [ ] Duplicate detection ❌
- [ ] Out-of-order handling ❌
- [ ] Packet loss handling ❌
- [ ] Tests ❌

**Status:** Not implemented (networking crate is stub)
**Time Estimate:** 3-4 days

#### **2.5 State Synchronization** ⚪ **NOT STARTED** - [docs/tasks/phase2-state-sync.md](docs/tasks/phase2-state-sync.md)
- [ ] StateUpdate enum (Full, Delta) ❌
- [ ] Snapshot history (server-side) ❌
- [ ] Delta diff computation ❌
- [ ] Delta application (client-side) ❌
- [ ] Adaptive full/delta switching ❌
- [ ] Client ack tracking ❌
- [ ] Tests (verify correctness) ❌

**Status:** Not implemented (networking crate is stub)
**Time Estimate:** 5-7 days
**Tests:** Property tests (delta application = full state)

#### **2.6 Client-Side Prediction** ⚪ **NOT STARTED** - [docs/tasks/phase2-client-prediction.md](docs/tasks/phase2-client-prediction.md)
- [ ] Input sequence numbering ❌
- [ ] Input buffering ❌
- [ ] Predicted world state ❌
- [ ] Server reconciliation ❌
- [ ] Replay unacknowledged inputs ❌
- [ ] Smoothing/interpolation ❌
- [ ] Tests ❌

**Status:** Not implemented (networking crate is stub)
**Time Estimate:** 5-6 days
**Difficulty:** High (subtle bugs)

#### **2.7 Server Authoritative Logic** ⚪ **NOT STARTED** - [docs/tasks/phase2-server-tick.md](docs/tasks/phase2-server-tick.md)
- [ ] Input validation (anti-cheat) ❌
- [ ] Server tick loop (60 TPS) ❌
- [ ] State broadcast ❌
- [ ] Per-client state tracking ❌
- [ ] Connection handling ❌
- [ ] Tests ❌

**Status:** Not implemented (networking crate is stub)
**Time Estimate:** 4-5 days

#### **2.8 Basic Interest Management** ⚪ **NOT STARTED** - [docs/tasks/phase2-interest-basic.md](docs/tasks/phase2-interest-basic.md)
- [ ] Spatial grid for proximity queries ❌
- [ ] Distance-based culling ❌
- [ ] Per-client visibility sets ❌
- [ ] Filter state updates by visibility ❌
- [ ] Tests (verify culling works) ❌

**Status:** Not implemented (interest crate is placeholder)
**Time Estimate:** 3-4 days
**Performance:** < 1ms per client

**Phase 2 Deliverables:**
- ✅ Client + server binaries compile separately with feature flags (DONE - 2.1B)
- ✅ Macros enforce client/server code separation (DONE - 2.1A)
- 🟡 Docker development environment working (Dockerfiles done, compose partial)
- 🟡 Metrics framework ready (observability crate done, endpoint not exposed)
- ❌ Basic admin console operational (NOT STARTED)
- ❌ TCP + UDP channels working (NOT STARTED - 2.3, 2.4)
- ❌ State sync (full + delta) with property tests (NOT STARTED - 2.5)
- ❌ Client prediction with < 10% error rate (NOT STARTED - 2.6)
- ❌ Basic multiplayer demo (2+ clients, <50ms latency) (NOT STARTED)
- ❌ Comprehensive test suite (>80% coverage) (Macros tested, rest pending)
- ❌ Benchmarks for all network operations (<1ms target) (NOT STARTED)

**Phase 2 Status:** ~15-20% complete (2.1 foundation mostly done)
**Total Phase 2 Time:** 4-5 weeks (added infrastructure week)
**Time Remaining:** ~3-4 weeks

---

## ⚙️ **Phase 3: Physics + Audio + LOD** (Weeks 10-13)

**Status:** 🟡 Minimal Start (Physics component only)

### **Goals**
- Physics integration (Rapier)
- Audio system (Kira)
- LOD for rendering + networking
- Fog of war / interest management
- Cross-platform testing complete

### **Tasks**

#### **3.1 Physics Integration** 🟡 **MINIMAL START (~5%)** - [docs/tasks/phase3-physics.md](docs/tasks/phase3-physics.md)
- [x] Velocity component ✅ (engine/physics/src/components.rs with tests)
- [ ] PhysicsBackend trait ❌
- [ ] Rapier backend implementation ❌ (dependency exists but unused)
- [ ] RigidBody, Collider components ❌
- [ ] Physics step abstraction ❌
- [ ] Transform sync (ECS ↔ Physics) ❌
- [ ] Async physics thread ❌
- [ ] Collision events ❌
- [ ] Physics queries (raycast, shapecast) ❌
- [ ] Integration tests ❌
- [ ] Benchmarks ❌

**Status:** Only basic component structure started
**Time Estimate:** 4-5 days remaining

#### **3.2 Audio System** ⚪ **NOT STARTED** - [docs/tasks/phase3-audio.md](docs/tasks/phase3-audio.md)
- [ ] Kira integration ❌
- [ ] AudioSource component ❌
- [ ] 3D spatial audio ❌
- [ ] Audio asset loading ❌
- [ ] Tests ❌

**Status:** Placeholder only (engine/audio/src/lib.rs = 339 bytes)
**Time Estimate:** 3-4 days

#### **3.3 Rendering LOD System** ⚪ **NOT STARTED** - [docs/tasks/phase3-lod-rendering.md](docs/tasks/phase3-lod-rendering.md)
- [ ] LodLevels component ❌
- [ ] Distance-based LOD switching ❌
- [ ] Mesh LOD (multiple meshes) ❌
- [ ] Texture LOD (mipmap selection) ❌
- [ ] Tests ❌

**Status:** Placeholder only (engine/lod/src/lib.rs = 344 bytes)
**Time Estimate:** 3-4 days

#### **3.4 Network LOD System** ⚪ **NOT STARTED** - [docs/tasks/phase3-lod-networking.md](docs/tasks/phase3-lod-networking.md)
- [ ] Network LOD (update rates) ❌
- [ ] Component mask filtering ❌
- [ ] Distance-based update frequencies ❌
- [ ] Tests ❌

**Status:** Will be integrated into networking crate
**Time Estimate:** 3-4 days

#### **3.5 Advanced Interest Management** ⚪ **NOT STARTED** - [docs/tasks/phase3-interest-advanced.md](docs/tasks/phase3-interest-advanced.md)
- [ ] Fog of war component ❌
- [ ] Team-based visibility ❌
- [ ] Occlusion culling (line-of-sight) ❌
- [ ] PVS (potentially visible sets) ❌
- [ ] Tests ❌
- [ ] Benchmarks (< 2% server time) ❌

**Status:** Placeholder only (engine/interest/src/lib.rs = 418 bytes)
**Time Estimate:** 5-7 days
**Performance:** Match VALORANT's <2% server time

#### **3.6 Cross-Platform Verification** ⚪ **NOT STARTED** - [docs/tasks/phase3-cross-platform-verify.md](docs/tasks/phase3-cross-platform-verify.md)
- [ ] Test suite on Windows ⚠️ CI exists but needs verification
- [ ] Test suite on Linux ⚠️ CI exists but needs verification
- [ ] Test suite on macOS x64 ⚠️ CI exists but needs verification
- [ ] Test suite on macOS ARM ⚠️ CI exists but needs verification
- [ ] Fix platform-specific bugs ❌
- [ ] Document platform quirks ❌

**Status:** CI infrastructure exists but comprehensive testing not done
**Time Estimate:** 3-5 days
**Goal:** 100% pass rate on all platforms

**Phase 3 Deliverables:**
- ❌ Physics working (Velocity component only, ~5%)
- ❌ Audio working (Placeholder only)
- ❌ LOD reducing network bandwidth by 80%+ (Placeholder only)
- ❌ Fog of war preventing wallhacks (Placeholder only)
- ⚠️ All platforms tested (CI exists, comprehensive testing needed)

**Phase 3 Status:** ~2% complete (minimal physics component only)
**Total Phase 3 Time:** 3-4 weeks
**Time Remaining:** ~3-4 weeks (full phase)

---

## 🎨 **Phase 4: Polish + Production Features** (Weeks 14-16)

**Status:** ⚪ Not Started

### **Goals**
- Auto-update system
- Advanced rendering (PBR, lighting, shadows)
- **Advanced profiling features** (GPU profiling, graphical UI)
- Save/load system
- Hot-reload for dev

**Note:** Basic profiling infrastructure completed in Phase 0. This phase adds GPU profiling and graphical profiling UI.

### **Tasks**

#### **4.1 Auto-Update System** ⚠️ **MUST READ:** [docs/tasks/phase4-auto-update.md](docs/tasks/phase4-auto-update.md)
- [ ] Version struct + comparison
- [ ] Update server API
- [ ] Delta patching (xdelta)
- [ ] Download + apply
- [ ] Customizable UI
- [ ] Tests

**Time Estimate:** 5-6 days

#### **4.2 PBR Rendering** ⚠️ **MUST READ:** [docs/tasks/phase4-pbr-materials.md](docs/tasks/phase4-pbr-materials.md)
- [ ] PBR shader (metallic/roughness)
- [ ] Texture support (albedo, normal, metallic, roughness)
- [ ] Image loading (PNG, JPG)
- [ ] Material system
- [ ] Tests

**Time Estimate:** 5-7 days

#### **4.3 Lighting System** ⚠️ **MUST READ:** [docs/tasks/phase4-lighting.md](docs/tasks/phase4-lighting.md)
- [ ] Point lights
- [ ] Directional lights
- [ ] Spot lights
- [ ] Shadow mapping (directional)
- [ ] Tests

**Time Estimate:** 5-7 days

#### **4.4 Advanced Profiling Features** ⚠️ **MUST READ:** [docs/tasks/phase4-advanced-profiling.md](docs/tasks/phase4-advanced-profiling.md)
- [ ] GPU profiling (Vulkan timestamp queries)
- [ ] GPU memory profiling
- [ ] Graphical profiling UI (in-engine timeline view)
- [ ] Flamegraph visualization
- [ ] Thread visualization timeline
- [ ] Integration with existing Phase 0 profiling

**Time Estimate:** 4-5 days

**Note:** CPU profiling infrastructure completed in Phase 0. This adds GPU-specific profiling and visual UI.

#### **4.5 Hot-Reload Dev Environment** ⚠️ **MUST READ:** [docs/tasks/phase4-hot-reload.md](docs/tasks/phase4-hot-reload.md)
- [ ] File watcher (notify crate)
- [ ] Client/server restart orchestration
- [ ] Docker Compose dev setup
- [ ] Documentation

**Time Estimate:** 3-4 days

#### **4.6 Save/Load System** ⚠️ **MUST READ:** [docs/tasks/phase4-save-load.md](docs/tasks/phase4-save-load.md)
- [ ] SaveSystem trait
- [ ] Reuse WorldState serialization
- [ ] Versioning
- [ ] Migration support
- [ ] Tests

**Time Estimate:** 3-4 days

**Phase 4 Deliverables:**
- ✅ Auto-update working
- ✅ Production-quality graphics
- ✅ Profiling integrated
- ✅ Dev environment smooth
- ✅ Save/load working

**Total Phase 4 Time:** 2-3 weeks

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
- [ ] Simple 3D game (platformer or shooter)
- [ ] No networking (pure local)
- [ ] Demonstrates: ECS, rendering, physics, audio
- [ ] Documented code
- [ ] README.md

**Time Estimate:** 4-5 days

#### **5.2 MMORPG Example** ⚠️ **MUST READ:** [docs/tasks/phase5-mmorpg-example.md](docs/tasks/phase5-mmorpg-example.md)
- [ ] Client + server architecture
- [ ] Persistent world
- [ ] Player movement + combat
- [ ] Fog of war
- [ ] LOD in action
- [ ] Dockerfile + docker-compose
- [ ] README.md

**Time Estimate:** 7-10 days

#### **5.3 Turn-Based Example** ⚠️ **MUST READ:** [docs/tasks/phase5-turnbased-example.md](docs/tasks/phase5-turnbased-example.md)
- [ ] Turn-based strategy game
- [ ] State-based gameplay
- [ ] Demonstrates: deterministic logic, save/load
- [ ] README.md

**Time Estimate:** 3-4 days

#### **5.4 MOBA Example** ⚠️ **MUST READ:** [docs/tasks/phase5-moba-example.md](docs/tasks/phase5-moba-example.md)
- [ ] 5v5 arena
- [ ] Real-time combat
- [ ] Team-based fog of war
- [ ] Demonstrates: high player count, prediction
- [ ] README.md

**Time Estimate:** 5-7 days

#### **5.5 mdBook Documentation** ⚠️ **MUST READ:** [docs/tasks/phase5-mdbook.md](docs/tasks/phase5-mdbook.md)
- [ ] Set up mdBook
- [ ] Getting Started guide
- [ ] Architecture overview
- [ ] API reference (from rustdoc)
- [ ] Tutorials
- [ ] Deploy to GitHub Pages

**Time Estimate:** 4-5 days

#### **5.6 Performance Benchmarks** ⚠️ **MUST READ:** [docs/tasks/phase5-benchmarks.md](docs/tasks/phase5-benchmarks.md)
- [ ] Criterion benchmarks for all critical paths
- [ ] Flamegraph generation
- [ ] Performance regression tests in CI
- [ ] Benchmark results in README

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

---

## 🚀 **Post-MVP (Future)**

Features not in initial release but planned:

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
- [ ] Dedicated server hosting

### **Tooling**
- [ ] Visual scene editor
- [ ] Asset pipeline (FBX, GLTF import)
- [ ] Shader editor
- [ ] Replay viewer

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
| Performance issues | Profile early and often (Tracy) |
| Scope creep | Strict MVP definition, defer features |

### **Dependencies**

| Dependency | Risk | Mitigation |
|------------|------|------------|
| ash | Low | Stable, well-maintained |
| Rapier | Low | Production-ready |
| Kira | Medium | Less mature, but good API |
| tokio | Low | Industry standard |
| FlatBuffers | Low | Google-backed |

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

**Last Updated:** 2026-02-01
**Current Phase:** Phase 0 (~80%), Phase 1 (~65%), Phase 2 (~15-20%)
**Active Work:**
- Phase 1.6: Basic Rendering Pipeline (37.5% - 3/8 modules complete)
- Phase 2.1: Foundation complete, networking implementation pending
- ECS Optimization: Ongoing performance improvements
**Next Milestones:**
- Complete Phase 1.6 remaining modules (framebuffers, commands, sync, shaders, orchestration)
- Complete Phase 2.2-2.8 (TCP/UDP, state sync, client prediction, server tick, interest mgmt)
- Fill in missing Phase 0 docs (ECS, networking, rendering, physics, audio, LOD, interest-management)
