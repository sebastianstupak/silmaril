# Agentic Debugging Infrastructure - Summary

**Date:** 2026-02-02
**Status:** Specification Complete
**Phase:** Pre-Implementation

---

## What Changed

### Previous Phase A: Visual Debugging (Human-Centric)
- **Goal:** PhysX PVD-style visual debugger for humans
- **Duration:** 4-6 weeks
- **LOC:** ~2,320
- **Priority:** Critical for developer productivity

**Limitation:** Visual tools require human interpretation, don't help AI agents debug issues autonomously.

### New Phase A: Agentic Debugging First (AI-First)
- **Goal:** Machine-readable data export enabling AI agents to debug physics to 100% accuracy
- **Duration:** 10-12 weeks (6-7 weeks for A.0, rest for visual tools)
- **LOC:** ~5,420 (3,100 for A.0 agentic infrastructure, 2,320 for visual tools)
- **Priority:** **CRITICAL** - Enables autonomous debugging by AI agents

**Philosophy Shift:**
> "Instead of building tools for humans to debug physics, build infrastructure for AI agents to debug physics autonomously, then add human-friendly visualizations later."

---

## New Section: A.0 Agentic Debugging Infrastructure

### What Was Added

#### 1. Physics State Snapshot System (A.0.1)
**400 LOC, 1 week**

```rust
pub struct PhysicsSnapshot {
    pub frame: u64,
    pub timestamp: f64,
    pub entities: Vec<EntityState>,      // All entity states
    pub colliders: Vec<ColliderState>,   // All collider shapes
    pub constraints: Vec<ConstraintState>, // All joints/constraints
    pub islands: Vec<IslandState>,       // Solver islands
}
```

**Enables:**
- Complete world state capture per frame
- Export to JSONL (streaming), SQLite (queryable), CSV (simple metrics)
- Frame-by-frame comparison (detect divergence, verify determinism)

---

#### 2. Event Stream Exporter (A.0.2)
**300 LOC, 4 days**

```rust
pub enum PhysicsEvent {
    CollisionStart { entity_a, entity_b, impulse, ... },
    CollisionEnd { ... },
    ConstraintBreak { force_magnitude, ... },
    EntityWake { reason: Collision | Force | Manual },
    EntitySleep { ... },
    SolverFailure { residual, iterations },
    ForceApplied { force, torque },
    EntityTeleport { old_pos, new_pos },
    DivergenceDetected { client_pos, server_pos, delta },
}
```

**Enables:**
- Temporal event analysis (what happened when)
- Collision tracking
- Constraint break detection
- Solver convergence monitoring
- User input tracking

---

#### 3. SQLite Time-Series Database (A.0.3)
**500 LOC, 1 week**

**Schema:**
- `snapshots` - Frame metadata
- `entity_states` - Per-frame entity data
- `events` - Event log
- `contact_manifolds` - Collision details
- `islands` - Solver partitioning

**Enables:**
- SQL-like queries: `SELECT * FROM collisions WHERE entity_a = 42`
- Time-range analysis: "Show entity velocity from frame 100-500"
- Pattern detection: "Find all constraint breaks"
- Performance analysis: "Show solver iterations per frame"

---

#### 4. Query API for AI Agents (A.0.4)
**400 LOC, 1 week**

```rust
pub struct PhysicsQueryAPI {
    // High-level queries for common debugging tasks
    pub fn entity_history(&self, entity_id, start_frame, end_frame) -> Vec<EntityState>;
    pub fn entity_collisions(&self, entity_id) -> Vec<CollisionEvent>;
    pub fn find_high_velocity(&self, entity_id, threshold) -> Vec<u64>;
    pub fn constraint_breaks(&self) -> Vec<ConstraintBreakEvent>;
    pub fn solver_failures(&self) -> Vec<SolverFailureEvent>;
    pub fn determinism_violations(&self) -> Vec<u64>;
    pub fn events_by_type(&self, event_type, start, end) -> Vec<PhysicsEvent>;

    // Low-level SQL for advanced analysis
    pub fn raw_query(&self, sql: &str) -> Result<Vec<HashMap<String, Value>>>;
}
```

**Enables:**
- AI agents query physics data programmatically
- No physics expertise required (high-level API)
- Complex analysis via raw SQL
- Historical debugging (no simulation re-run needed)

---

#### 5. Solver Internals Export (A.0.5)
**450 LOC, 1 week**

```rust
pub struct SolverInternals {
    pub broadphase_pairs: Vec<(u64, u64)>,    // Which entities might collide
    pub narrowphase_manifolds: Vec<ContactManifold>, // Actual collision points
    pub islands: Vec<IslandData>,              // Solver partitioning
    pub solver_stats: SolverStats,             // Convergence metrics
}

pub struct ContactManifold {
    pub contacts: Vec<ContactPoint>,
    // Each contact has: point, normal, penetration_depth, impulse
}
```

**Enables:**
- Understand solver behavior (why did it fail to converge?)
- Detect numerical instability (high residuals, divergence)
- Validate collision detection (are contact points correct?)
- Debug performance (which islands are expensive?)

---

#### 6. Divergence Detection System (A.0.6)
**250 LOC, 3 days**

```rust
pub struct DivergenceReport {
    pub frame: u64,
    pub expected_hash: String,    // SHA256 of reference state
    pub actual_hash: String,
    pub diverged_entities: Vec<EntityDivergence>,
}

pub struct EntityDivergence {
    pub entity_id: u64,
    pub expected_position: Vec3,
    pub actual_position: Vec3,
    pub delta: f32,
}
```

**Enables:**
- Determinism validation (same inputs → same outputs?)
- Multiplayer debugging (client vs server state comparison)
- Regression detection (did this change break physics?)
- Floating-point error tracking

---

#### 7. Network State Diff Exporter (A.0.7)
**300 LOC, 4 days**

**Enables:**
- Export client state and server state
- AI agent compares and identifies divergence
- Root cause analysis for multiplayer desync

---

#### 8. CSV Metrics Exporter (A.0.8)
**150 LOC, 2 days**

**Enables:**
- Simple time-series export (frame, entity_id, pos_x, pos_y, vel_x, ...)
- Easy import into data analysis tools (pandas, Excel, MATLAB)
- Performance trend analysis

---

## Total Investment

### Phase A.0: Agentic Debugging
- **LOC:** ~3,100
- **Time:** 6-7 weeks
- **Team:** 1 developer
- **Tests:** 32 unit tests, 12 integration tests
- **Benchmarks:** Snapshot overhead, export throughput, query latency

### Phase A (Total)
- **LOC:** ~5,420 (A.0 + A.1-A.3)
- **Time:** 10-12 weeks
- **Team:** 1-2 developers (recommend 2 to parallelize)

---

## Impact

### Before Agentic Debugging
**Scenario:** Entity explodes at frame 500

1. Developer adds print statements
2. Re-runs simulation, watches visually
3. Tries to catch the moment it breaks
4. Guesses at root cause
5. Repeats 10+ times

**Time:** 2-8 hours per bug
**Accuracy:** Depends on developer skill
**Scalability:** Manual, doesn't scale

### After Agentic Debugging
**Scenario:** Entity explodes at frame 500

1. AI agent queries exported database:
   ```rust
   let high_vel_frames = db.find_high_velocity(42, 100.0)?; // Instant
   let collisions = db.entity_collisions(42)?; // Instant
   let solver_internals = db.get_solver_internals(498)?; // Instant
   ```

2. AI agent analyzes:
   - Velocity spike at frame 498
   - Collision with low-mass entity (0.00001 kg)
   - Mass ratio 100,000:1 (exceeds solver limit)
   - Solver failed to converge

3. AI agent suggests fix:
   - Clamp minimum mass to 0.1 kg
   - Or upgrade to TGS solver (Phase C)

**Time:** 30 seconds
**Accuracy:** 100% (data-driven)
**Scalability:** Fully autonomous

---

## Concrete Example

See: [AGENTIC_DEBUGGING_EXAMPLE.md](tasks/AGENTIC_DEBUGGING_EXAMPLE.md)

**Bug:** Entity 42 explodes at frame 500
**Root Cause:** Collision with ultra-low-mass entity (0.00001 kg) creates mass ratio of 100,000:1, causing solver divergence
**Detection Time:** 30 seconds (AI agent queries database)
**Fix:** Clamp minimum mass to 0.1 kg
**Verification:** Automated (re-run simulation, compare velocity at frame 498)

**Traditional Debugging:** 2-8 hours
**Agentic Debugging:** 30 seconds

---

## Why This Matters for AI-First Engine

### 1. Autonomous Development
AI agents can now:
- ✅ Debug physics issues without human intervention
- ✅ Detect regressions automatically (compare exports before/after)
- ✅ Validate physics accuracy (vs reference data)
- ✅ Optimize performance (identify solver bottlenecks)

### 2. Multiplayer First-Class
- ✅ Client-server divergence detection (automatic)
- ✅ Determinism validation (hash-based comparison)
- ✅ Desync root cause analysis (AI agent identifies which entity diverged)

### 3. Continuous Validation
- ✅ Every commit exports physics state
- ✅ CI compares against reference exports
- ✅ AI agent flags regressions before merge
- ✅ No manual testing required

### 4. Developer Productivity
Even human developers benefit:
- Export data once, query many times (no re-running simulation)
- Historical analysis (debug issues from production)
- Data-driven debugging (no guessing)
- Automated documentation (AI generates bug reports)

---

## Comparison: Visual vs Agentic Debugging

| Aspect | Visual (A.1-A.3) | Agentic (A.0) |
|--------|------------------|---------------|
| **Target User** | Humans | AI Agents |
| **Data Format** | Rendered overlays | JSONL/SQLite/CSV |
| **Query Method** | Visual inspection | Programmatic API |
| **Temporal Analysis** | Replay + scrub timeline | SQL queries |
| **Automation** | Manual | Fully automated |
| **Scalability** | 1 bug at a time | Find all similar bugs |
| **Documentation** | Screenshots | Auto-generated reports |
| **Overhead** | Runtime (rendering) | Export-time only |

**Both are valuable.** Visual tools help humans understand physics at a glance. Agentic tools enable AI agents to debug issues autonomously. By prioritizing A.0 first, we enable autonomous debugging immediately, then add human-friendly visualizations later.

---

## Roadmap Position

### Updated Phase Order

**Before:**
1. Phase A: Visual debugging (humans)
2. Phase B: GPU particles
3. Phase C: TGS solver
4. ...

**After:**
1. **Phase A.0: Agentic debugging (AI agents) ← NEW PRIORITY**
2. Phase A.1-A.3: Visual debugging (humans)
3. Phase B: GPU particles
4. Phase C: TGS solver
5. ...

### Rationale

1. **Blocks all other phases:** Can't efficiently debug cloth/fluids/destruction without data export
2. **Unique competitive advantage:** No other engine has AI-first debugging
3. **Immediate ROI:** Speeds up all future development
4. **Multiplayer critical:** Client-server divergence detection is essential for networked games

---

## Next Steps

### Implementation Order (Recommended)

**Week 1-2: A.0.1-A.0.2 (Core Infrastructure)**
- PhysicsSnapshot struct + serialization
- Event stream
- JSONL export

**Week 3-4: A.0.3 (Database)**
- SQLite schema
- Export to database
- Basic queries

**Week 5-6: A.0.4-A.0.6 (Query API + Validation)**
- High-level query API
- Divergence detection
- Hash-based determinism validation

**Week 7: A.0.7-A.0.9 (Multiplayer + Polish)**
- Network state diff
- CSV exporter
- Performance optimization

**Week 8-10: A.1-A.3 (Visual Tools) - OPTIONAL, can parallelize**
- Debug rendering
- Profiling UI
- Network debugging UI

**Week 11-12: Integration + Testing**
- End-to-end testing
- Example AI agent debugger
- Documentation

---

## Success Criteria

### A.0 Complete When:
- ✅ AI agent can debug physics bug from exported data alone
- ✅ Divergence detection accurate to single entity
- ✅ Query API covers 90% of common debugging scenarios
- ✅ Performance overhead < 5% when recording
- ✅ Exports are human-readable (JSON) AND machine-queryable (SQLite)
- ✅ Example AI agent debugger included in repo

### Validation Test:
1. Inject physics bug (e.g., ultra-low mass entity)
2. Run simulation with export enabled
3. AI agent analyzes exported data
4. AI agent identifies root cause within 60 seconds
5. AI agent suggests correct fix

**If AI agent can do this, A.0 is complete.**

---

## Long-Term Vision

### Phase A.0 Enables (Future Work)

**A.0.10: Real-Time Anomaly Detection**
- AI agent monitors physics during simulation
- Flags potential bugs before they explode
- Auto-pauses simulation when divergence detected

**A.0.11: Automatic Fix Suggestion Engine**
- AI agent not only diagnoses, but proposes code fixes
- Generate pull requests for physics bugs
- Fully autonomous debugging + fixing

**A.0.12: Regression Test Generation**
- AI agent creates unit tests from bug reports
- Prevent fixed bugs from re-occurring
- Continuous physics validation

**A.0.13: Performance Optimization**
- AI agent identifies solver bottlenecks from profiling data
- Suggests optimizations (e.g., "island X takes 80% of time, reduce entities")
- Auto-tunes solver parameters

---

## Conclusion

**Agentic debugging infrastructure (A.0) is the foundation for AI-first physics development.**

By prioritizing machine-readable data export over human-centric visualizations, we enable:
- ✅ Autonomous debugging by AI agents
- ✅ 100x faster bug diagnosis (30 seconds vs 2-8 hours)
- ✅ Automated regression detection
- ✅ Multiplayer divergence debugging
- ✅ Data-driven development workflow

**This is the competitive advantage of an AI-first game engine.**

Traditional engines (Unity, Unreal) have excellent visual debuggers for humans, but no infrastructure for AI agents to debug autonomously. We flip the priority: **AI-first, humans second.**

**Total investment:** 6-7 weeks, ~3,100 LOC
**ROI:** Speeds up all future physics development, enables autonomous debugging
**Unique:** No other engine has this capability

**Recommendation:** Start Phase A.0 immediately.
