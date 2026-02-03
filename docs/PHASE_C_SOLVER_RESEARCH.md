# Phase C: Advanced Constraint Solver - Research Findings

**Date:** 2026-02-03
**Rapier Version:** 0.18
**Status:** Research Complete, Scope Adjusted

---

## Executive Summary

**Original Goal:** Implement TGS (Total Gauss-Seidel) solver to replace Rapier's PGS (Projected Gauss-Seidel) solver for improved stability and convergence.

**Finding:** Rapier 0.18 does NOT expose a plugin API for replacing the core solver algorithm. TGS exists only in Dimforge's experimental GPU physics engine (wgrapier), with no CPU release timeline.

**Revised Approach:** Focus on solver **tuning, optimization, and enhancement** within Rapier's existing framework rather than replacing the core algorithm.

---

## Rapier 0.18 Solver Architecture

### Current Implementation

- **Base Algorithm:** Projected Gauss-Seidel (PGS) velocity solver
- **Position Stabilization:** Non-linear PGS for constraint correction
- **Variants:**
  - `switch_to_small_steps_pgs_solver()` - Modern default (better convergence, more stable joints)
  - `switch_to_standard_pgs_solver()` - Legacy v0.17 behavior (faster, less stable)

### Configurable Parameters

From [IntegrationParameters](https://docs.rs/rapier3d/0.18/rapier3d/dynamics/struct.IntegrationParameters.html):

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `num_solver_iterations` | 4 | Outer constraint solver iterations |
| `num_internal_pgs_iterations` | 1 | Inner PGS iterations per outer step |
| `num_additional_friction_iterations` | 4 | Extra friction resolution passes |
| `erp` | 0.8 | Error Reduction Parameter (0-1) |
| `damping_ratio` | 0.25 | Baumgarte stabilization damping |
| `joint_erp` | 1.0 | Joint-specific error reduction |
| `joint_damping_ratio` | 0.25 | Joint constraint regularization |
| `allowed_linear_error` | 0.001m | Penetration tolerance |
| `max_penetration_correction` | - | Per-step correction limit |

### PhysicsHooks Customization

Rapier provides [PhysicsHooks](https://docs.rs/rapier3d/latest/rapier3d/pipeline/trait.PhysicsHooks.html) for runtime behavior modification:

- `filter_contact_pair()` - Control which colliders generate contacts
- `filter_intersection_pair()` - Control intersection detection
- `modify_solver_contacts()` - Modify contact properties (friction, restitution, surface velocity)

**Limitation:** Hooks modify contact properties, NOT the solver algorithm itself.

---

## TGS Solver Status

### GPU Implementation (wgrapier)

From [Dimforge 2025 Review](https://dimforge.com/blog/2026/01/09/the-year-2025-in-dimforge/):

- **Status:** Experimental GPU physics engine using "modern Soft-TGS constraint solver"
- **Scale:** 93,000 bodies with 120,000 joints demonstrated
- **Platform:** WGSL-based, browser demos at wgmath.rs
- **Roadmap:** Transitioning to rust-gpu for code sharing with main Rapier

### CPU TGS

- **Status:** Not available in Rapier 0.18
- **Timeline:** No public commitment for CPU TGS release
- **2026 Priorities:** Robotics accuracy improvements, GPU rust-gpu migration

---

## Feasibility Analysis: Original Phase C Tasks

| Task | Feasible | Notes |
|------|----------|-------|
| C.2.1: TGS velocity solver core (800 LOC) | ❌ **NO** | Requires forking Rapier, modifying internal solver |
| C.2.2: Sub-stepping framework (300 LOC) | ✅ **YES** | Can implement at PhysicsWorld level |
| C.2.3: Warm starting (250 LOC) | ⚠️ **PARTIAL** | Rapier may already have this; verify |
| C.2.4: Constraint relaxation (200 LOC) | ✅ **YES** | Tune `erp`, `damping_ratio` parameters |
| C.2.5: Rapier integration (400 LOC) | ❌ **NO** | N/A without custom solver |
| C.3.1: LDL direct solver (600 LOC) | ❌ **NO** | Requires forking Rapier |
| C.3.2: Speculative contacts (300 LOC) | ✅ **YES** | Can implement via PhysicsHooks |
| C.3.3: SIMD/cache optimization (200 LOC) | ✅ **YES** | Optimize our integration layer |

**Verdict:** Cannot implement full TGS without forking Rapier. Adjust scope to feasible enhancements.

---

## Revised Phase C: Solver Tuning & Optimization

**New Goal:** Maximize Rapier 0.18 solver performance through configuration tuning, sub-stepping, and integration optimization.

**Expected Outcomes:**
- Improved stability through tuned parameters
- Reduced iteration counts via warm starting
- Better convergence with optimized sub-stepping
- Enhanced integration performance (SIMD, caching)

### Revised Task List

| Task | Description | LOC | Time | Priority |
|------|-------------|-----|------|----------|
| **C.1.1** | Study Rapier solver (DONE) | - | ✅ 1 week | Complete |
| **C.1.2** | Benchmark baseline performance | 200 | 2 days | High |
| **C.1.3** | Profile solver hotspots | - | 2 days | High |
| **C.2.1** | Implement adaptive sub-stepping | 300 | 1 week | High |
| **C.2.2** | Tune erp/damping for stability | 150 | 3 days | High |
| **C.2.3** | Verify warm-starting behavior | 100 | 2 days | Medium |
| **C.2.4** | Implement solver presets (Stable/Fast/Quality) | 200 | 3 days | Medium |
| **C.3.1** | Speculative contacts via hooks | 300 | 1 week | Medium |
| **C.3.2** | SIMD optimization for integration | 200 | 1 week | Medium |
| **C.3.3** | Constraint caching for static bodies | 250 | 4 days | Low |

**Total:** ~1,700 LOC, 5-6 weeks (vs 10-14 weeks original)

### Testing Strategy

**Unit Tests (15 tests):**
- Parameter tuning (erp, damping at various values)
- Sub-stepping accuracy (1/2/4/8 substeps)
- Warm starting verification (impulse persistence)
- Speculative contact triggering
- Solver preset comparison

**Integration Tests (12 tests):**
- Chain stability (20-body chain, joint separation < 0.001)
- Stack stability (10-box tower, collapse time)
- Mass ratio handling (10:1, 100:1)
- Ragdoll simulation (15 joints, no explosion)
- High-speed collision (speculative contacts)

**Benchmarks:**
- Baseline: 100/1000/5000 constraints
- Sub-stepping overhead measurement
- Iteration count reduction (tuned vs default)
- Solver time scaling
- SIMD speedup measurement

### Acceptance Criteria

- ✅ Reduce iteration count by 20-30% through tuning
- ✅ Sub-stepping improves stability for high-speed scenarios
- ✅ Solver presets provide quality/performance trade-offs
- ✅ All existing physics tests pass
- ✅ Determinism maintained (critical for networking)
- ✅ Performance overhead < 10% vs baseline

---

## Recommendation

**Proceed with Revised Phase C** focusing on:
1. Baseline benchmarking (C.1.2-C.1.3)
2. Sub-stepping implementation (C.2.1)
3. Parameter tuning (C.2.2-C.2.4)
4. Integration optimization (C.3.2-C.3.3)

**Defer TGS implementation** until:
- Dimforge releases CPU TGS in future Rapier version, OR
- Project requirements justify forking Rapier (7-10 weeks investment)

**Estimated Completion:** 5-6 weeks (reduced from 10-14 weeks)

---

## Sources

- [Rapier3D IntegrationParameters Documentation](https://docs.rs/rapier3d/0.18/rapier3d/dynamics/struct.IntegrationParameters.html)
- [PhysicsHooks Trait Documentation](https://docs.rs/rapier3d/latest/rapier3d/pipeline/trait.PhysicsHooks.html)
- [Dimforge 2025 Review & 2026 Goals](https://dimforge.com/blog/2026/01/09/the-year-2025-in-dimforge/)
- [Rapier GitHub Repository](https://github.com/dimforge/rapier)
