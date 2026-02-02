# Agentic Physics Debugging - Practical Example

## Scenario: Entity Explosion Bug

**User Report:** "Entity #42 suddenly flew off the screen at frame ~500 during multiplayer game"

**Traditional Debugging (Human):**
1. Add print statements
2. Re-run simulation and watch visually
3. Try to catch the moment it breaks
4. Guess at root cause
5. Repeat 10+ times
6. **Time: Hours to days**

**Agentic Debugging (AI Agent):**
1. Query exported data
2. Analyze patterns
3. Identify root cause
4. Suggest fix
5. **Time: Seconds to minutes**

---

## Step-by-Step: AI Agent Workflow

### Step 1: Agent Receives Bug Report

```
User: "Entity 42 exploded around frame 500"
```

### Step 2: Agent Loads Physics Database

```rust
// Agent code
let db = PhysicsQueryAPI::open("physics_recording.db")?;
```

### Step 3: Agent Identifies Explosion Frame

```rust
// Query: Find frames where entity 42 had abnormally high velocity
let high_vel_frames = db.find_high_velocity(42, 100.0)?;

// Result:
// [498, 499, 500, 501, 502, ...]
```

**Agent Output:**
> "Detected velocity spike starting at frame 498. Investigating pre-spike state..."

### Step 4: Agent Examines State History

```rust
// Get entity state history leading up to explosion
let states = db.entity_history(42, 450, 505)?;

// Analyze velocity progression
for (i, state) in states.iter().enumerate() {
    let frame = 450 + i;
    let vel_mag = state.linear_velocity.length();

    if vel_mag > 10.0 {
        println!("Frame {}: vel = {:.2} m/s", frame, vel_mag);
    }
}
```

**Output:**
```
Frame 450-497: vel = 0.2-2.5 m/s (normal)
Frame 498: vel = 15.3 m/s  ⚠️ SPIKE START
Frame 499: vel = 87.2 m/s  ⚠️ EXPLOSION
Frame 500: vel = 245.8 m/s ⚠️ RUNAWAY
```

**Agent Analysis:**
> "Velocity jumped from 2.5 m/s to 15.3 m/s at frame 498. This is the triggering frame."

### Step 5: Agent Investigates Frame 498 Events

```rust
// Query: What events happened at frame 498?
let events = db.events_by_type("*", 498, 498)?;

// Result:
// [
//   CollisionStart { entity_a: 42, entity_b: 13, impulse: 5234.2, ... }
//   EntityWake { entity_id: 42, reason: Collision }
// ]
```

**Agent Insight:**
> "Collision with entity 13 at frame 498 generated impulse of **5234.2 N·s** (abnormally high). Investigating entity 13..."

### Step 6: Agent Examines Collision Partner

```rust
// Get entity 13's state at frame 498
let entity_13_state = db.entity_history(13, 498, 498)?[0];

println!("Entity 13 at frame 498:");
println!("  Mass: {}", entity_13_state.mass);
println!("  Velocity: {:?}", entity_13_state.linear_velocity);
```

**Output:**
```
Entity 13 at frame 498:
  Mass: 0.00001 kg  ⚠️ EXTREMELY LOW
  Velocity: [2.3, -1.2, 0.5] m/s
```

**Agent Analysis:**
> "Entity 13 has mass of **0.00001 kg** (0.01 grams). This is causing numerical instability in collision solver."

### Step 7: Agent Examines Solver Internals

```rust
// Get solver internals for frame 498
let solver_data = db.get_solver_internals(498)?;

// Find contact manifold for entities 42 & 13
let manifold = solver_data.narrowphase_manifolds.iter()
    .find(|m| (m.entity_a == 42 && m.entity_b == 13) ||
              (m.entity_a == 13 && m.entity_b == 42))
    .unwrap();

println!("Contact manifold:");
for contact in &manifold.contacts {
    println!("  Impulse: {:.2} N·s", contact.impulse);
    println!("  Penetration: {:.4} m", contact.penetration_depth);
}

// Check solver convergence
println!("Solver stats:");
println!("  Iterations: {}", solver_data.solver_stats.total_iterations);
println!("  Converged: {}", solver_data.solver_stats.convergence_achieved);
println!("  Max residual: {:.6}", solver_data.solver_stats.max_residual);
```

**Output:**
```
Contact manifold:
  Impulse: 5234.18 N·s  ⚠️ EXTREMELY HIGH
  Penetration: 0.0023 m (normal)

Solver stats:
  Iterations: 8
  Converged: false  ⚠️ DID NOT CONVERGE
  Max residual: 12.3456  ⚠️ HIGH ERROR
```

**Agent Diagnosis:**
> "Solver failed to converge due to mass ratio issue. Entity 42 (mass: 1.0 kg) colliding with entity 13 (mass: 0.00001 kg) creates mass ratio of **100,000:1**, exceeding solver stability limits."

### Step 8: Agent Checks for Pattern

```rust
// Query: Find all extremely low-mass entities
let all_entities = db.raw_query("
    SELECT DISTINCT entity_id, mass
    FROM entity_states
    WHERE mass < 0.001
    ORDER BY mass ASC
")?;

println!("Extremely low-mass entities:");
for row in all_entities {
    println!("  Entity {}: mass = {}", row["entity_id"], row["mass"]);
}
```

**Output:**
```
Extremely low-mass entities:
  Entity 13: mass = 0.00001 kg
  Entity 94: mass = 0.00002 kg
  Entity 127: mass = 0.00001 kg
```

**Agent Analysis:**
> "Found 3 entities with mass < 0.001 kg. This is a systemic issue, not isolated to entity 13."

### Step 9: Agent Provides Root Cause & Fix

**Agent Report:**

```markdown
## Root Cause Analysis: Entity 42 Explosion Bug

### Summary
Entity 42 exploded at frame 498 due to collision with extremely low-mass entity 13 (0.00001 kg), creating a mass ratio of 100,000:1 that caused solver divergence and generated unrealistic impulse.

### Evidence
1. **Frame 498:** Velocity spike from 2.5 m/s → 15.3 m/s
2. **Collision Event:** Entity 42 ↔ Entity 13, impulse = 5234.2 N·s (abnormal)
3. **Mass Ratio:** 1.0 kg / 0.00001 kg = 100,000:1 (exceeds solver limit ~1000:1)
4. **Solver Failure:** Did not converge (8 iterations, residual = 12.3)
5. **Systemic Issue:** 3 entities with mass < 0.001 kg detected

### Recommended Fixes

**Option 1: Clamp Minimum Mass (Immediate Fix)**
```rust
// In entity spawn code:
let mass = user_mass.max(0.1); // Clamp to 100g minimum
world.add_rigidbody(entity, &RigidBody::dynamic(mass), ...);
```

**Option 2: Add Mass Validation (Prevention)**
```rust
impl RigidBody {
    pub fn dynamic(mass: f32) -> Self {
        assert!(mass >= 0.1, "Mass must be >= 0.1 kg for numerical stability");
        // ...
    }
}
```

**Option 3: Upgrade Constraint Solver (Long-term Fix)**
Implement TGS solver (Phase C) which handles mass ratios up to 10,000:1.

### Verification Steps
1. Apply fix (clamp mass to 0.1 kg minimum)
2. Re-run simulation from frame 0 with same inputs
3. Verify entity 42 velocity remains < 10 m/s at frame 498
4. Check solver convergence rate (should be 100%)
```

### Step 10: Agent Verifies Fix

```rust
// After applying fix, agent re-runs simulation
let db_after_fix = PhysicsQueryAPI::open("physics_recording_after_fix.db")?;

// Check entity 42 velocity at frame 498
let states_after = db_after_fix.entity_history(42, 498, 502)?;

println!("After fix:");
for state in states_after {
    let vel_mag = state.linear_velocity.length();
    println!("  Frame {}: vel = {:.2} m/s", state.frame, vel_mag);
}

// Check solver convergence
let solver_after = db_after_fix.get_solver_internals(498)?;
println!("  Solver converged: {}", solver_after.solver_stats.convergence_achieved);
```

**Output:**
```
After fix:
  Frame 498: vel = 2.7 m/s  ✅ NORMAL
  Frame 499: vel = 2.5 m/s  ✅ NORMAL
  Frame 500: vel = 2.3 m/s  ✅ NORMAL
  Solver converged: true    ✅ FIXED
```

**Agent Confirmation:**
> "Fix verified. Entity 42 no longer explodes. Solver converges successfully. Bug resolved."

---

## Comparison: Traditional vs Agentic Debugging

| Aspect | Traditional (Human) | Agentic (AI) |
|--------|---------------------|--------------|
| **Time to Root Cause** | 2-8 hours | 30 seconds |
| **Accuracy** | Depends on developer skill | 100% (data-driven) |
| **Reproducibility** | Requires re-running simulation | Query historical data |
| **Pattern Detection** | Manual inspection | Automated (find all similar bugs) |
| **Fix Verification** | Manual testing | Automated comparison |
| **Documentation** | Often incomplete | Automatically generated |

---

## Data Requirements (What Made This Possible)

This debugging workflow required the following data exports:

### ✅ From A.0.1: Physics State Snapshots
- Entity positions, velocities, masses (frames 450-505)

### ✅ From A.0.2: Event Stream
- Collision events at frame 498
- Entity wake events

### ✅ From A.0.3: SQLite Database
- Queryable time-series data
- Cross-frame analysis

### ✅ From A.0.4: Query API
- `find_high_velocity(entity, threshold)`
- `entity_history(entity, start_frame, end_frame)`
- `events_by_type(type, start, end)`

### ✅ From A.0.5: Solver Internals
- Contact manifolds with impulses
- Solver convergence stats
- Island partitioning

Without agentic debugging infrastructure, this bug would require:
- Manual reproduction attempts
- Adding print statements
- Visual inspection
- Educated guessing
- **Total time: 2-8 hours**

With agentic debugging infrastructure:
- Single query to exported database
- Automated pattern detection
- Data-driven root cause analysis
- **Total time: 30 seconds**

---

## Next Steps

### Implementing A.0 Enables:
1. **Autonomous Bug Fixing** - AI agents can detect, diagnose, and fix physics bugs
2. **Regression Detection** - Compare exports before/after changes
3. **Performance Analysis** - Find performance bottlenecks from solver data
4. **Multiplayer Debugging** - Detect client-server divergence automatically
5. **Quality Assurance** - AI agents validate physics accuracy vs reference data

### Future Enhancements (Post-A.0):
- **A.0.10:** Real-time anomaly detection (flag potential bugs during simulation)
- **A.0.11:** Automatic fix suggestion engine (AI proposes code fixes)
- **A.0.12:** Regression test generation (AI creates unit tests from bug reports)
- **A.0.13:** Performance optimization suggestions (AI identifies solver bottlenecks)

---

## Conclusion

**Agentic debugging transforms physics development from:**
- ❌ Slow, manual, error-prone debugging
- ❌ Requires expert physics knowledge
- ❌ Time-consuming reproduction
- ❌ Incomplete documentation

**To:**
- ✅ Instant, automated, data-driven debugging
- ✅ Accessible to AI agents (no physics expertise needed)
- ✅ Historical analysis (no re-running required)
- ✅ Complete audit trail

**This is the AI-first game engine advantage.**
