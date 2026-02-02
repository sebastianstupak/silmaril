# Advanced Physics Constraint Solvers Research Report (2024-2026)

**Research Focus**: Constraint solver algorithms for real-time game engines with 16ms frame budget (60fps)

**Date**: 2026-02-02

---

## Executive Summary

Modern game physics engines employ sophisticated constraint solvers to handle rigid body dynamics, joints, and contacts within strict real-time budgets. This report analyzes the state-of-the-art algorithms, their trade-offs, and implementation considerations for high-performance game engines.

**Key Finding**: The industry has converged on hybrid approaches combining iterative solvers (PGS/TGS) with direct solvers (LDL) and position-based methods (XPBD) to balance performance, stability, and determinism.

---

## 1. Solver Algorithms Overview

### 1.1 PGS (Projected Gauss-Seidel)

**Status**: Industry standard for two decades, still widely used

**Description**: Iterative method solving Linear Complementarity Problems (LCP) sequentially, one constraint at a time.

**Technical Details**:
- Solves constraints independently in sequence, converging to global solution over iterations
- "Projection" handles inequality constraints (e.g., contact separation, friction cones)
- Equivalent to Sequential Impulse method popularized by Erin Catto (Box2D)
- Operates on velocity-level constraints

**Convergence Rate**:
- **Poor to Moderate**: Typically requires 4-10 iterations for acceptable accuracy
- Convergence depends heavily on constraint ordering
- Struggles with high mass ratios (>10:1) and complex mechanisms
- Benefits significantly from warm starting (40-60% fewer iterations)

**Implementation Complexity**: **3/10**
- Simple to implement
- Minimal mathematical prerequisites
- Well-documented in game physics literature
- Easy to debug

**Performance Characteristics**:
- **Linear scaling** with constraint count: O(n) per iteration
- **Cache-friendly** sequential access pattern
- Typical cost: 0.5-2ms for 1000 constraints @ 6 iterations (single-threaded)
- Parallelization difficult due to sequential nature

**Industry Usage**:
- Box2D (2006-present)
- Bullet Physics (with modifications)
- PhysX 3.x (legacy)
- Unity built-in physics (legacy)

**Determinism**: ✅ Excellent
- Fully deterministic given fixed iteration count
- Same inputs always produce same outputs
- Constraint ordering affects convergence but not determinism

**Sources**:
- [Box2D Solver2D](https://box2d.org/posts/2024/02/solver2d/)
- [Roblox Advanced Physics Solver](https://mizerski.medium.com/improving-simulation-and-performance-with-an-advanced-physics-solver-da071a98d35)
- [PGS vs Sequential Impulse Comparison](http://www.mft-spirit.nl/files/MTamis_PGS_SI_Comparison.pdf)

---

### 1.2 TGS (Temporal Gauss-Seidel)

**Status**: Modern improvement over PGS, introduced in PhysX 4.0 (2018)

**Description**: PGS with sub-stepping - each iteration is a smaller timestep with single constraint solve pass.

**Key Innovation**: "Small steps in physics simulation" - prefer sub-stepping over iteration count

**Technical Details**:
- Subdivides timestep into n substeps (typically 4-8)
- Each substep: single PGS iteration + integrate velocities
- Dynamically recomputes constraints between substeps based on relative motion
- No broad-phase update or contact regeneration between substeps (performance win)

**Convergence Rate**:
- **Good to Excellent**: 2-4x faster convergence than PGS
- Better energy conservation
- Improved stability for stiff systems
- Handles mass ratios up to 100:1 robustly

**Implementation Complexity**: **5/10**
- Moderate - requires careful timestep management
- Integration with existing PGS solver straightforward
- Sub-stepping logic adds complexity
- Must handle partial-timestep state correctly

**Performance Characteristics**:
- Similar per-iteration cost as PGS
- Achieves better results with fewer total iterations
- Typical speedup: 1.5-2.5x over PGS for same accuracy
- Sub-stepping enables higher soft constraint stiffness (cheaper than NGS)

**Trade-offs**:
- Higher temporal resolution improves stability but reduces parallelism opportunities
- Some developers reported initial stability issues during PhysX 3.x → 4.x migration
- Requires tuning substep count vs iteration count balance

**Industry Usage**:
- PhysX 4.0+ (NVIDIA, 2018-present)
- Unity PhysX integration
- Unreal Engine Chaos (uses TGS concepts)

**Determinism**: ✅ Good
- Deterministic with fixed substep count
- Sub-stepping must use consistent timesteps
- Floating-point differences still possible across platforms

**Sources**:
- [NVIDIA PhysX 4.0 Announcement](https://developer.nvidia.com/blog/announcing-physx-sdk-4-0-an-open-source-physics-engine/)
- [TGS Formulation Discussion](https://forums.developer.nvidia.com/t/formulation-of-the-temporal-gauss-seidel-tgs-solver/69391)
- [PGS vs TGS Differences](https://forums.developer.nvidia.com/t/differences-between-pgs-ad-tgs-solvers/277935)

---

### 1.3 XPBD (Extended Position-Based Dynamics)

**Status**: Cutting-edge research (2016-present), production-ready as of 2023

**Description**: Position-based solver with compliance parameters enabling soft constraints and implicit stiffness control.

**Key Innovation**: Decouples constraint stiffness from iteration count and timestep size

**Technical Details**:
- Operates on position-level constraints (vs velocity-level in PGS/TGS)
- Lagrange multipliers evolved as state variables
- Compliance parameter per constraint: α = 1/(k·Δt²) where k = stiffness
- Supports arbitrary elastic/dissipative energy potentials
- Damping controlled via α̃ = α/(1 + d·Δt) where d = damping ratio

**Convergence Rate**:
- **Excellent**: Often requires only 1-4 iterations for stable results
- Convergence independent of timestep (major advantage)
- Sub-stepping improves accuracy: n substeps with 1 iteration > 1 step with n iterations
- Multi-layer solver (2024) achieves 3-5x speedup via coarse-to-fine hierarchy

**Implementation Complexity**: **7/10**
- More complex than PGS - requires understanding of compliance formulation
- Lagrange multiplier state tracking adds memory overhead
- Constraint derivation more involved
- Tuning compliance parameters requires physical intuition

**Performance Characteristics**:
- **Competitive with PGS** at low iteration counts (1-4 iterations typical)
- Scales well to GPU (data-parallel friendly)
- Multi-grid acceleration achieves mesh-independent convergence
- Typical cost: 0.3-1.5ms for 1000 constraints @ 2 iterations

**Recent Developments (2024)**:
- **Multi-Layer Solver**: Exploits rigid/elastic part decomposition for faster convergence
- **DiffXPBD**: Fully differentiable for ML/robotics applications
- **GPU Acceleration**: 3-6x speedup over 4-core CPU

**Industry Usage**:
- Unreal Engine 5 Chaos (PBD-based, not XPBD yet)
- Research/academic simulators
- VFX/film industry (clothing, soft bodies)
- Emerging in game engines (2024-2026)

**Determinism**: ⚠️ Moderate
- Position-based methods less deterministic than velocity-based
- Timestep-independent behavior helps but not fully deterministic
- Iteration order affects results
- Requires careful implementation for cross-platform determinism

**Sources**:
- [XPBD Original Paper](https://matthias-research.github.io/pages/publications/XPBD.pdf)
- [Multi-Layer Solver for XPBD (2024)](https://onlinelibrary.wiley.com/doi/10.1111/cgf.15186)
- [XPBD Overview - Carmen's Graphics](https://carmencincotti.com/2022-08-08/xpbd-extended-position-based-dynamics/)
- [Physics-Based Animation - Multi-Layer XPBD](https://www.physicsbasedanimation.com/2024/08/26/a-multi-layer-solver-for-xpbd/)

---

### 1.4 NGS (Non-linear Gauss-Seidel)

**Status**: Specialized technique, less common in modern engines

**Description**: Position constraint solver that updates Jacobians and mass matrix per iteration to account for non-linearities.

**Technical Details**:
- Solves position constraints after velocity constraints
- Uses pseudo-velocities that don't affect kinetic energy
- Merges Newton outer loop with Gauss-Seidel inner loop
- Constantly updates mass, Jacobians, constraint error during iteration

**Convergence Rate**:
- **Good**: Better than Baumgarte stabilization for position drift
- Handles non-linear constraints well
- Requires fewer iterations than Baumgarte for equivalent accuracy

**Implementation Complexity**: **6/10**
- More complex than PGS
- Jacobian/mass recomputation per iteration expensive
- Conceptually straightforward once PGS understood

**Performance Characteristics**:
- **Much more expensive** than PGS per iteration (2-3x cost)
- Used sparingly, often as post-stabilization pass
- Sub-stepping (TGS) often preferred over NGS due to cost

**Industry Usage**:
- Box2D (early versions, now less emphasized)
- Specialized high-precision scenarios
- Often replaced by TGS in modern engines

**Determinism**: ✅ Good
- Deterministic given fixed iteration count
- Jacobian updates must be deterministic

**Sources**:
- [Box2D Solver2D](https://box2d.org/posts/2024/02/solver2d/)
- [Roblox Solver Improvements](https://mizerski.medium.com/improving-simulation-and-performance-with-an-advanced-physics-solver-da071a98d35)

---

### 1.5 Direct Solvers (LDL Decomposition)

**Status**: Hybrid use with iterative solvers for stability improvements

**Description**: Exact linear system solver using LDL matrix decomposition, typically for equality constraints only.

**Technical Details**:
- Solves Ax = b exactly using LDL^T factorization
- Applied to equality constraints (joints without limits)
- Inequality constraints (contacts, friction) handled by PGS
- Preconditioning step for iterative solver

**Convergence Rate**:
- **Perfect**: Single-pass exact solution for linear subsystem
- Dramatically improves stability of complex mechanisms
- Addresses high mass ratio problems (>100:1)

**Implementation Complexity**: **8/10**
- Requires sparse linear algebra library
- Matrix construction non-trivial
- Numerical stability concerns
- Integration with iterative solver requires care

**Performance Characteristics**:
- **O(n³) worst case**, but sparse methods reduce to O(n^1.5) - O(n²)
- Expensive for large constraint counts (>1000)
- Used selectively for "difficult" constraints
- Typical cost: 0.5-3ms for 100-500 equality constraints

**Hybrid Approach** (Roblox, 2018):
- LDL for equality constraints (joints)
- PGS for inequality constraints (contacts, limits)
- Improved stability with minimal performance hit
- Handles mass ratios up to 1000:1

**Industry Usage**:
- Roblox (hybrid LDL-PGS since 2018)
- Specialized applications requiring high precision
- CAD/engineering simulators

**Determinism**: ✅ Excellent
- Exact solution is fully deterministic
- Matrix operations must use consistent precision

**Sources**:
- [Roblox Advanced Physics Solver](https://mizerski.medium.com/improving-simulation-and-performance-with-an-advanced-physics-solver-da071a98d35)
- [Direct Solver vs Sequential Impulse](https://gdcvault.com/play/1026684/Math-In-Game-Development-Summit)

---

### 1.6 Featherstone Algorithm (Articulated Body)

**Status**: Specialized for kinematic chains (ragdolls, robots)

**Description**: Linear-time algorithm (O(n)) for articulated body dynamics using recursive formulation.

**Technical Details**:
- Tree-structured kinematic chains
- Recursive forward/backward passes
- Achieves zero joint separation (perfectly rigid joints)
- Exploits kinematic chain structure for efficiency

**Convergence Rate**:
- **N/A**: Direct method, not iterative
- Exact solution for tree structures

**Implementation Complexity**: **9/10**
- Complex recursive algorithm
- Requires understanding of spatial algebra
- Integration with contact constraints non-trivial

**Performance Characteristics**:
- **O(n) scaling** with joint count (vs O(n²) naive)
- Excellent for long chains (>10 joints)
- 4x speedup reported for 30-DOF quadruped
- DCA (divide-and-conquer) variant enables parallelism but 3x slower single-threaded

**Industry Usage**:
- Bullet Physics (hybrid with PGS)
- Robotics simulators
- High-DOF character animation
- Specialized for ragdolls

**Determinism**: ✅ Excellent
- Deterministic recursive algorithm

**Sources**:
- [Featherstone Algorithm Overview](https://www.thyrix.com/documentation/featherstone_method.php)
- [GDC Vault - MLCP and Featherstone](https://www.gdcvault.com/play/1020076/Physics-for-Game-Programmers-Exploring)

---

## 2. Comparative Analysis

### 2.1 Convergence Rate Comparison

| Solver | Iterations Required | Convergence Quality | Mass Ratio Limit |
|--------|---------------------|---------------------|------------------|
| PGS | 6-10 | Moderate | 10:1 |
| TGS | 3-5 (with 4-8 substeps) | Good | 100:1 |
| XPBD | 1-4 | Excellent | 50:1 |
| NGS | 4-8 | Good | 20:1 |
| LDL (direct) | 1 (exact) | Perfect | 1000:1+ |
| Featherstone | 1 (exact for tree) | Perfect | N/A |

**Key Insight**: Modern engines combine multiple solvers - direct/Featherstone for articulated bodies, TGS/XPBD for contacts.

---

### 2.2 Implementation Complexity Scale (1-10)

| Solver | Complexity | Development Time | Mathematical Background |
|--------|------------|------------------|-------------------------|
| PGS | 3 | 1-2 weeks | Basic linear algebra |
| TGS | 5 | 3-4 weeks | PGS + timestep integration |
| XPBD | 7 | 4-6 weeks | Lagrangian mechanics, compliance |
| NGS | 6 | 3-4 weeks | Non-linear systems |
| LDL (direct) | 8 | 6-8 weeks | Sparse linear algebra, numerical methods |
| Featherstone | 9 | 8-12 weeks | Spatial algebra, recursion |

**Recommendation**: Start with PGS for rapid prototyping, evolve to TGS or XPBD for production quality.

---

### 2.3 Performance Characteristics

#### Single-Threaded Performance (1000 constraints, typical game scenario)

| Solver | Time per Iteration | Typical Iterations | Total Time | Accuracy |
|--------|-------------------|-------------------|------------|----------|
| PGS | 0.25ms | 8 | 2.0ms | Moderate |
| TGS | 0.30ms | 4 | 1.2ms | Good |
| XPBD | 0.40ms | 2 | 0.8ms | Excellent |
| NGS | 0.70ms | 6 | 4.2ms | Good |
| LDL | 2.5ms | 1 | 2.5ms | Perfect |

**Note**: These are approximate benchmarks from literature and developer reports, not standardized tests.

#### Scalability with Constraint Count

| Solver | Complexity | 100 Constraints | 1000 Constraints | 10000 Constraints |
|--------|------------|----------------|------------------|-------------------|
| PGS | O(n) per iter | 0.02ms/iter | 0.25ms/iter | 2.5ms/iter |
| TGS | O(n) per iter | 0.03ms/iter | 0.30ms/iter | 3.0ms/iter |
| XPBD | O(n) per iter | 0.04ms/iter | 0.40ms/iter | 4.0ms/iter |
| LDL | O(n^1.5)-O(n²) | 0.2ms | 2.5ms | 50ms+ |

**Key Finding**: Iterative solvers scale linearly, making them suitable for large-scale simulations. Direct solvers limited to <1000 constraints in real-time.

---

### 2.4 Determinism Analysis

**Critical for**:
- Networked multiplayer (lockstep, rollback netcode)
- Replay systems
- Cross-platform parity
- Automated testing

#### Determinism Levels

| Solver | Same Machine | Cross-Platform | Networking Suitability |
|--------|--------------|----------------|------------------------|
| PGS | ✅ Perfect | ⚠️ FP differences | Good with fixed-point |
| TGS | ✅ Perfect | ⚠️ FP differences | Good with fixed-point |
| XPBD | ⚠️ Good | ⚠️ Moderate | Moderate |
| LDL | ✅ Perfect | ⚠️ FP differences | Good with careful impl |
| Featherstone | ✅ Perfect | ⚠️ FP differences | Excellent |

**Determinism Requirements**:
1. **Fixed timestep** (mandatory)
2. **Fixed iteration count** (no early-exit)
3. **Consistent constraint ordering** (no dynamic reordering)
4. **IEEE 754 floating-point compliance** (same precision across platforms)
5. **Deterministic collision detection** (Box2D 2024: fully deterministic)

**Box2D Determinism Levels (August 2024)**:
- **Algorithmic**: Same executable, same results
- **Multithreaded**: Deterministic even with threading
- **Cross-platform**: Windows/Linux/macOS identical results
- ⚠️ **No rollback support**: State snapshots not provided

**Sources**:
- [Box2D Determinism (August 2024)](https://box2d.org/posts/2024/08/determinism/)
- [Floating Point Determinism](https://gafferongames.com/post/floating_point_determinism/)
- [Unity DOTS Physics Determinism](https://discussions.unity.com/t/dots-physics-and-determinism/736199)

---

## 3. Advanced Techniques

### 3.1 Warm Starting

**Description**: Use previous frame's impulses/Lagrange multipliers as initial guess for current frame.

**Benefits**:
- **40-60% fewer iterations** for equivalent accuracy
- Exploits temporal coherence (physics changes slowly frame-to-frame)
- Critical for resting contacts (objects stacked/sleeping)

**Implementation Requirements**:
- Persistent contact manifolds (cannot clear contacts each frame)
- Contact tracking across frames (ID-based or spatial proximity)
- Lagrange multiplier storage (per-constraint state)

**Performance Impact**:
- Negligible memory overhead (~4 bytes per constraint)
- Amortizes convergence over multiple frames
- Enables stable simulation with fewer iterations

**Recommendation**: ✅ Always implement warm starting for production engines.

**Sources**:
- [Game Physics Stability - Warm Starting](https://allenchou.net/2014/01/game-physics-stability-warm-starting/)
- [Box2D Solver2D (2024)](https://box2d.org/posts/2024/02/solver2d/)

---

### 3.2 Constraint Relaxation & Damping

**Purpose**: Soft constraints for stability and realism

**Techniques**:

#### Soft Constraints (Harmonic Oscillator Model)
```
Parameters:
- Frequency (Hz): Natural oscillation frequency (1-30 Hz typical)
- Damping Ratio (ξ): 0 = undamped, 1 = critical damping, >1 = overdamped
- Stiffness: Derived from frequency and mass
```

**Tuning Guidelines**:
- **Stiff joints**: 20-30 Hz, ξ = 0.7-1.0
- **Soft joints**: 5-10 Hz, ξ = 0.3-0.5
- **Position stabilization**: 10-15 Hz, ξ = 1.0

#### Relaxation Parameters
- **SOR (Successive Over-Relaxation)**: ω ∈ [1.0, 1.5]
  - ω = 1.0: Standard Gauss-Seidel
  - ω > 1.0: Faster convergence, risk of instability
  - ω = 1.2-1.3: Sweet spot for games

**Unity DOTS Physics (2024 Update)**:
- Closed-form expression for regularization parameters (τ, damping)
- Constant-time complexity (independent of iteration count)
- Mass-independent soft constraint parameters (frequency, damping, timestep only)

**Sources**:
- [Soft Constraints - Erin Catto GDC 2011](https://box2d.org/files/ErinCatto_SoftConstraints_GDC2011.pdf)
- [Unity Physics 1.3.14 Changelog](https://docs.unity3d.com/Packages/com.unity.physics@1.3/changelog/CHANGELOG.html)

---

### 3.3 Sub-Stepping

**Description**: Subdivide physics timestep into smaller steps for stability.

**Benefits**:
- Improves CCD (continuous collision detection)
- Enables higher constraint stiffness
- Better energy conservation
- Prevents tunneling for fast-moving objects

**Implementation**:
```rust
// Pseudo-code
fn physics_update(dt: f32) {
    let substeps = 4;
    let sub_dt = dt / substeps as f32;

    for _ in 0..substeps {
        integrate_velocities(sub_dt);
        solve_constraints(sub_dt, 1); // Single iteration per substep
        integrate_positions(sub_dt);
    }
}
```

**Cost-Benefit Analysis**:
- `n` substeps × 1 iteration ≈ 1.2-1.5× cost of `n` iterations in 1 step
- **But**: Significantly better accuracy and stability
- TGS is essentially formalized sub-stepping

**Recommendation**:
- Use 4-8 substeps for high-precision requirements
- Combine with reduced iteration count (2-4 iterations per substep)

**Unreal Engine Approach**:
- Max substep size configurable (default: 33ms → 16ms)
- Async physics mode: Fixed timestep for full determinism

**Sources**:
- [Small Steps in Physics Simulation](https://www.semanticscholar.org/paper/Small-steps-in-physics-simulation-Macklin-Storey/7dd777c0c51d4d2682836fdb6420cc634b234664)
- [Unreal Engine Substepping](https://www.aclockworkberry.com/unreal-engine-substepping/)

---

### 3.4 Speculative Contacts

**Description**: Solve for contacts before they occur based on predicted motion.

**Benefits**:
- **Prevents tunneling** without expensive CCD
- Improves stability by eliminating contact flip-flop
- Distance-based resolution (no velocity dependence)
- Cheaper than sweep-based CCD

**How It Works**:
1. Inflate contact offset based on object velocity: `offset += velocity * dt`
2. Generate contacts at predicted future position
3. Solver prevents penetration before it occurs
4. Negative-depth contacts allowed (speculative)

**Stability Improvement**:
- Solves contact flip-flop problem (contact detected → resolved → lost → detected...)
- Provides "preview" of contact to solver
- Enables solver to see whole contact surface, preventing rotation into ground

**PhysX Implementation**:
- Part of discrete simulation (no separate CCD pass)
- Much cheaper than sweep-based CCD
- Less robust than sweep CCD (not suitable for bullets/projectiles)

**Sources**:
- [PhysX Advanced Collision Detection](https://nvidia-omniverse.github.io/PhysX/physx/5.3.0/docs/AdvancedCollisionDetection.html)
- [Unexpected Speculative Stability](https://jahej.com/alt/2011_04_11_unexpected-speculative-stability.html)

---

### 3.5 Contact Manifold Generation

**PCM (Persistent Contact Manifold)**:
- Generate full contact manifold on first contact
- Recycle and update existing contacts in subsequent frames
- Select subset of contacts to maximize manifold area

**Benefits**:
- Better stacking stability (more contacts = more stable)
- Warm starting works better with persistent contacts
- Reduces jitter from contact point changes

**Trade-offs**:
- Fewer contacts than discrete detection (4-8 vs 100+)
- May reduce stacking stability with insufficient solver iterations
- Requires careful contact tracking

**Box2D Approach (2024)**:
- 1-2 contacts per manifold (2D)
- Clipping algorithm for edge-edge contacts
- Deepest point + area maximization heuristic

**PhysX Approach**:
- Up to 4 contacts per manifold (3D)
- Face clipping for box-box, convex-convex
- Hybrid with discrete detection

**Sources**:
- [PhysX Persistent Contact Manifold](https://documentation.help/NVIDIA-PhysX-SDK-Guide/PersistentContactManifold.html)
- [Contact Manifold Generation - Valve](https://media.steampowered.com/apps/valve/2015/DirkGregorius_Contacts.pdf)

---

## 4. Testing Methodologies

### 4.1 Joint Stability Tests

**Purpose**: Validate solver handles complex constraint configurations.

#### Test Scenarios:

**1. Chain Test**
- 10-20 rigid bodies connected by hinges/ball-socket joints
- Apply gravity
- Measure: Joint separation, energy drift, frame time

**Success Criteria**:
- Zero joint separation (<0.001 units)
- Energy drift <5% over 10 seconds
- Consistent frame time (<16ms @ 60fps)

**2. Ragdoll Test**
- Humanoid with 15+ joints
- Drop from height, apply forces
- Measure: Joint limits respected, no explosions, realism

**3. High Mass Ratio Test**
- Small object (1kg) attached to large object (1000kg)
- Apply forces to small object
- Measure: Stability, convergence, joint integrity

**Expected Results by Solver**:
- PGS: Fails at 50:1 ratio
- TGS: Stable up to 100:1
- LDL-hybrid: Stable up to 1000:1

---

### 4.2 Constraint Violation Metrics

**Metrics to Track**:

**1. Position Error** (C_pos)
```
C_pos = |current_separation - target_separation|
Target: <0.01 units (1cm for meter-scale worlds)
```

**2. Velocity Error** (C_vel)
```
C_vel = |relative_velocity · constraint_normal|
Target: <0.1 units/sec
```

**3. Constraint Force Magnitude**
```
Track impulse magnitude per constraint
Sudden spikes indicate instability
```

**4. Energy Conservation**
```
E_total = E_kinetic + E_potential
Drift rate: <1% per second for conservative systems
```

---

### 4.3 Stacking Stability Tests

**Classic Test**: Tower of boxes

**Configuration**:
- 10-20 boxes stacked vertically
- Small initial perturbation
- Run for 30 seconds

**Metrics**:
- Time to collapse (longer = more stable)
- Final box count (# boxes still stacked)
- Solver iterations required

**Results by Solver** (Box2D Solver2D testing):
- PGS: 6-8 iterations for 10-box tower
- TGS: 4-5 iterations (with 4 substeps)
- XPBD: 2-3 iterations

**PCM Impact**:
- More contacts (4-8 per pair) → Better stability
- Fewer contacts with low iterations → Instability

---

### 4.4 Performance Benchmarking

**Standard Scenarios**:

**1. Simple Scene** (Baseline)
- 100 rigid bodies
- 50 contacts
- 10 joints
- Target: <1ms solver time

**2. Moderate Scene**
- 1000 rigid bodies
- 500 contacts
- 50 joints
- Target: <5ms solver time

**3. Complex Scene**
- 5000 rigid bodies
- 2000 contacts
- 200 joints
- Target: <12ms solver time (leaves 4ms for collision detection, integration)

**Metrics to Measure**:
- Solver time (isolated)
- Iteration count
- Convergence quality (position/velocity error)
- Frame-to-frame variance

**Profiling**:
- Profile per constraint type (contact, hinge, ball-socket)
- Identify bottlenecks (warm starting, Jacobian computation, impulse application)
- Compare single vs multi-threaded

---

### 4.5 Convergence Quality Tests

**Method**: Monitor constraint error over iterations

**Expected Behavior**:
- **PGS**: Linear convergence initially, plateaus
- **TGS**: Faster initial convergence, better asymptotic
- **XPBD**: Near-constant quality per iteration

**Test**:
```
For each iteration i:
    Measure sum of constraint violations: Σ|C_i|

Plot: Error vs Iteration Count

Good solver: Exponential decay
Poor solver: Slow linear decrease or oscillation
```

**Convergence Rate Formula**:
```
rate = log(error_i / error_0) / i
```
- Faster convergence → More negative rate

---

## 5. Benchmark Metrics Summary

### 5.1 Recommended Metrics

| Metric | Measurement | Target (60fps) | Critical Threshold |
|--------|-------------|----------------|--------------------|
| **Solver Time** | Per-frame average | <8ms | <12ms |
| **Iteration Count** | Average per frame | 4-6 | 10 max |
| **Position Error** | Max constraint violation | <0.01 units | <0.05 units |
| **Velocity Error** | Max relative velocity | <0.1 units/s | <0.5 units/s |
| **Energy Drift** | % change per second | <1% | <5% |
| **Joint Separation** | Max distance | 0.001 units | 0.01 units |
| **Mass Ratio** | Max stable ratio | 100:1 | 10:1 |

---

### 5.2 Performance Budget (16ms total @ 60fps)

**Typical Breakdown**:
- **Broad Phase**: 1-2ms
- **Narrow Phase** (collision detection): 2-4ms
- **Constraint Solver**: 6-10ms
- **Integration & Updates**: 1-2ms
- **Misc (sleeping, events)**: 1-2ms

**Solver Budget Details** (8ms target):
- Constraint setup (Jacobians, mass): 2ms
- Iteration loop: 5ms (4-6 iterations)
- Warm starting: 0.5ms
- Post-stabilization: 0.5ms

---

## 6. Recommendations for Agent Game Engine

### 6.1 Primary Solver Choice

**Recommendation**: **TGS (Temporal Gauss-Seidel)** with LDL preconditioning for equality constraints

**Rationale**:
1. **Proven in AAA engines** (PhysX 4.0+, Unreal Chaos)
2. **Performance/stability balance** optimal for games
3. **Mass ratio handling** (100:1) sufficient for most scenarios
4. **Implementation complexity** moderate (5/10)
5. **Determinism** achievable with fixed timesteps

**Implementation Phases**:
1. **Phase 1**: Implement PGS (2-3 weeks) - Foundation
2. **Phase 2**: Add sub-stepping → TGS (1-2 weeks) - Performance
3. **Phase 3**: LDL preconditioning (3-4 weeks) - Stability for complex mechanisms
4. **Phase 4**: Warm starting (1 week) - Optimization

**Total Estimated Time**: 7-10 weeks for production-ready solver

---

### 6.2 Alternative: XPBD for Innovation

**If targeting cutting-edge physics**:
- XPBD offers timestep-independent stiffness
- Multi-layer solver (2024) shows significant promise
- Better suited for soft bodies, cloth (future features)

**Trade-offs**:
- Less industry validation (newer)
- Determinism more challenging
- Requires expertise in Lagrangian mechanics

**Recommendation**: Consider XPBD for Phase 2+ if soft body physics planned.

---

### 6.3 Determinism Strategy

**For multiplayer/replay support**:

1. **Fixed Timestep**: 60 Hz (16.67ms) or 120 Hz (8.33ms)
2. **Fixed Iteration Count**: No early-exit convergence checks
3. **Consistent Ordering**: Deterministic constraint ordering (entity ID-based)
4. **IEEE 754 Compliance**: Use same floating-point precision (f32) everywhere
5. **Platform Testing**: Validate Windows/Linux/macOS produce identical results

**Box2D Model** (2024):
- Cross-platform determinism achieved
- Multithreaded determinism via island-based parallelism
- Consider replicating their approach

---

### 6.4 Testing Infrastructure

**Essential Tests**:

**Unit Tests**:
- Single constraint solve accuracy
- Jacobian computation correctness
- Impulse application validation

**Integration Tests**:
- Chain test (10-20 bodies)
- Ragdoll test (humanoid)
- Stacking test (10-box tower)
- High mass ratio test (100:1, 1000:1 with LDL)

**Performance Tests**:
- 100, 1000, 5000 body scenarios
- Solver time profiling
- Iteration count tracking
- Convergence quality measurement

**Regression Tests**:
- Record simulation state every 10 frames
- Compare against golden master
- Detect determinism breaks

**Benchmark Suite**:
- Compare against Box2D, Bullet, PhysX (if possible)
- Publish results for transparency

---

### 6.5 Profiling Integration

**Per CLAUDE.md requirements**:

```rust
use engine_profiling::{profile_scope, ProfileCategory};

#[profile(category = "Physics")]
fn solve_constraints(&mut self, dt: f32) {
    profile_scope!("constraint_solver");

    {
        profile_scope!("warm_start");
        self.warm_start();
    }

    for i in 0..self.iteration_count {
        profile_scope!("solver_iteration");
        self.solve_iteration(dt);
    }

    {
        profile_scope!("store_impulses");
        self.store_impulses();
    }
}
```

**Metrics to Expose**:
- Solver time per frame (avg, p50, p95, p99)
- Iteration count distribution
- Constraint count breakdown (contacts, joints)
- Convergence quality (position/velocity error)

---

## 7. Industry Examples (2024-2026)

### 7.1 Unity DOTS Physics (2024)

**Solver**: Impulse-based (PGS variant) with TGS concepts

**Recent Improvements**:
- Parallel constraint solver: 16 → 64 phases (major parallelism boost)
- Impulse event thresholds configurable
- Motor constraints: Max impulse limits added
- Regularization formula optimized: Closed-form, O(1) computation

**Performance**:
- Scales well to multi-core (parallelism improvements)
- Handles 1000s of constraints efficiently

**Sources**:
- [Unity Physics 1.3.14 Changelog](https://docs.unity3d.com/Packages/com.unity.physics@1.3/changelog/CHANGELOG.html)

---

### 7.2 Unreal Engine 5 Chaos (2024)

**Solver**: PBD (Position-Based Dynamics) with custom extensions

**Architecture**:
- FPBDRigidsSolver (core)
- "GBF" integration (Guendelman, Bridson, Fedkiw for collisions)
- Server-authoritative support
- Large world coordinate support

**Tools** (Unreal Fest 2024):
- Chaos Visual Debugger (Beta in UE 5.4)
- Debug Draw system

**Philosophy**: Full control over physics (vs licensed PhysX), enabling custom optimizations for Unreal-specific features.

**Sources**:
- [Exploring Unreal's Physics Framework](https://itscai.us/blog/post/ue-physics-framework/)
- [Chaos Scene Queries and Rigid Body Engine](https://www.unrealengine.com/en-US/tech-blog/chaos-scene-queries-and-rigid-body-engine-in-ue5)

---

### 7.3 Box2D Solver2D (February 2024)

**Research Platform**: 8 different Gauss-Seidel based solvers

**Goal**: Compare solver behavior, stability, performance

**Key Insights**:
- Sub-stepping (TGS) reduces iteration count significantly
- Warm starting critical for performance (all solvers benefit)
- NGS more expensive than TGS for equivalent accuracy
- Lower iteration counts possible with sub-stepping (2-4 vs 6-10)

**Impact**: Influences future Box2D releases and broader community understanding.

**Sources**:
- [Box2D Solver2D](https://box2d.org/posts/2024/02/solver2d/)

---

### 7.4 Roblox Hybrid Solver (2018-2024)

**Approach**: LDL + PGS hybrid

**Innovation**: Direct solver for equality constraints, iterative for inequalities

**Results**:
- Improved stability for complex mechanisms (gears, linkages)
- Handles mass ratios >1000:1
- Minimal performance overhead (<10% vs pure PGS)

**Lessons**:
- Hybrid approaches offer best of both worlds
- Selective use of expensive solvers viable
- Preconditioning dramatically improves convergence

**Sources**:
- [Roblox Advanced Physics Solver](https://mizerski.medium.com/improving-simulation-and-performance-with-an-advanced-physics-solver-da071a98d35)

---

## 8. Open Research Questions (2024-2026)

### 8.1 GPU Acceleration

**Challenge**: Constraint solving is inherently sequential (Gauss-Seidel)

**Approaches**:
- Jacobi solver (fully parallel but worse convergence)
- Graph coloring for parallel PGS (Box2D investigated)
- XPBD with GPU multi-grid solvers (research, 3-6x speedup)

**Determinism Concern**: Parallel solvers often non-deterministic

---

### 8.2 Machine Learning Integration

**DiffXPBD (2024)**: Fully differentiable physics for gradient-based optimization

**Applications**:
- Parameter tuning (stiffness, damping)
- Control policy learning (robotics)
- Inverse physics problems

**Game Engine Relevance**: Limited currently, but promising for AI-driven content.

---

### 8.3 Soft Constraint Simplifications

**Mass-Independent Parameters** (Bepu, 2024):
- Simplify soft constraints to 3 parameters: frequency, damping ratio, timestep
- Automatic mass adjustment
- Easier for designers to tune

**Adoption**: Emerging in modern engines, worth considering.

---

## 9. Glossary

**PGS**: Projected Gauss-Seidel - Iterative constraint solver
**TGS**: Temporal Gauss-Seidel - PGS with sub-stepping
**XPBD**: Extended Position-Based Dynamics - Position solver with compliance
**NGS**: Non-linear Gauss-Seidel - Position solver updating Jacobians per iteration
**LDL**: LDL^T matrix decomposition - Direct linear system solver
**LCP**: Linear Complementarity Problem - Mathematical formulation for inequality constraints
**Lagrange Multiplier**: State variable representing constraint force/impulse
**Compliance**: Inverse stiffness (α = 1/k), enables soft constraints
**Warm Starting**: Using previous solution as initial guess
**Baumgarte Stabilization**: Position error correction via biased impulses
**Speculative Contacts**: Solving contacts before penetration occurs
**PCM**: Persistent Contact Manifold - Reusable contact point sets
**Articulated Body**: Kinematic chain (e.g., ragdoll, robot arm)

---

## 10. References

### Key Papers
- "Small Steps in Physics Simulation" (Macklin, Storey) - TGS foundation
- "XPBD: Position-Based Simulation of Compliant Constrained Dynamics" (Macklin et al., 2016)
- "A Multi-layer Solver for XPBD" (Mercier-Aubin et al., 2024)
- "Soft Constraints" (Catto, GDC 2011) - Harmonic oscillator model

### Industry Sources
- [Box2D Solver2D (2024)](https://box2d.org/posts/2024/02/solver2d/)
- [Box2D Determinism (2024)](https://box2d.org/posts/2024/08/determinism/)
- [NVIDIA PhysX 4.0 SDK](https://developer.nvidia.com/blog/announcing-physx-sdk-4-0-an-open-source-physics-engine/)
- [Roblox Advanced Physics](https://mizerski.medium.com/improving-simulation-and-performance-with-an-advanced-physics-solver-da071a98d35)
- [Unity DOTS Physics Changelog](https://docs.unity3d.com/Packages/com.unity.physics@1.3/changelog/CHANGELOG.html)
- [Unreal Chaos Physics](https://itscai.us/blog/post/ue-physics-framework/)

### Educational Resources
- [Allen Chou - Game Physics Series](https://allenchou.net/2013/12/game-physics-constraints-sequential-impulse/)
- [Toptal - Constrained Rigid Body Simulation](https://www.toptal.com/game/video-game-physics-part-iii-constrained-rigid-body-simulation)
- [Gaffer On Games - Floating Point Determinism](https://gafferongames.com/post/floating_point_determinism/)

### Research Papers
- [DiffXPBD (arXiv:2301.01396)](https://arxiv.org/abs/2301.01396)
- [Survey of Rigid Body Simulation with Extended Position Based Dynamics (arXiv:2311.09327)](https://arxiv.org/pdf/2311.09327)

---

## Appendix A: Quick Reference Table

| Need | Recommended Solver | Complexity | Est. Time | Notes |
|------|-------------------|------------|-----------|-------|
| Rapid prototyping | PGS | 3/10 | 2 weeks | Start here |
| Production physics | TGS | 5/10 | 4 weeks | Industry standard |
| High stability | TGS + LDL hybrid | 8/10 | 8 weeks | Complex mechanisms |
| Soft bodies | XPBD | 7/10 | 6 weeks | Cutting-edge |
| Ragdolls | Featherstone + TGS | 9/10 | 10 weeks | Specialized |
| Determinism | TGS (fixed timestep) | 5/10 | 5 weeks | Networking |

---

## Appendix B: Implementation Checklist

**Phase 1: PGS Foundation** (2-3 weeks)
- [ ] Contact constraint (normal impulse, friction)
- [ ] Joint constraint base class
- [ ] Hinge joint
- [ ] Ball-socket joint
- [ ] PGS iteration loop
- [ ] Basic tests (chain, stack)

**Phase 2: TGS Enhancement** (2-3 weeks)
- [ ] Sub-stepping infrastructure
- [ ] TGS iteration integration
- [ ] Timestep subdivision tuning
- [ ] Performance comparison vs PGS

**Phase 3: Warm Starting** (1 week)
- [ ] Impulse storage
- [ ] Contact tracking/persistence
- [ ] Warm start application
- [ ] Measure iteration reduction

**Phase 4: LDL Hybrid** (3-4 weeks)
- [ ] Sparse matrix construction
- [ ] LDL decomposition (use library: nalgebra-sparse, faer)
- [ ] Equality constraint detection
- [ ] Hybrid solver integration
- [ ] High mass ratio tests

**Phase 5: Advanced Features** (2-3 weeks)
- [ ] Soft constraints (frequency, damping)
- [ ] Speculative contacts
- [ ] PCM (persistent contact manifolds)
- [ ] Sleeping optimization

**Total**: 10-14 weeks for full-featured solver

---

## Conclusion

Modern constraint solvers represent a balance between **performance** (16ms budget), **stability** (complex constraints, high mass ratios), and **determinism** (networking, replays). The industry has converged on **iterative solvers with enhancements**:

- **TGS** as the primary workhorse (PhysX, Unity, emerging standard)
- **Direct solvers** (LDL) for stability in difficult scenarios
- **XPBD** as the cutting-edge alternative for next-gen features

For **agent-game-engine**, the recommended path is **TGS with LDL preconditioning**, providing AAA-quality physics while maintaining reasonable implementation complexity. Warm starting and sub-stepping are non-negotiable for production quality.

The 2024-2026 research landscape shows continued refinement of these techniques, with improvements in **parallelism** (Unity's 64-phase solver), **determinism** (Box2D cross-platform), and **innovation** (XPBD multi-layer solver). These advances make it an excellent time to implement a modern constraint solver.

**Final Recommendation**: Allocate 10-14 weeks for physics solver implementation, prioritizing **correctness** (tests) → **performance** (profiling) → **determinism** (fixed timestep) → **stability** (LDL hybrid) in that order.

---

**Document Status**: Research Complete
**Next Steps**: Implementation planning, task breakdown, proof-of-concept prototyping
**Estimated Reading Time**: 45-60 minutes
