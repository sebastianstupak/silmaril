# Advanced Vehicle Physics Research Report (2024-2026)

**Date:** 2026-02-02
**Purpose:** Research advanced vehicle physics for game engine implementation
**Focus:** Racing/driving games with practical implementation guidance

---

## Executive Summary

This report synthesizes current state-of-the-art vehicle physics approaches for game engines, covering tire models, suspension systems, drivetrain simulation, and network prediction. The research spans arcade to simulation spectrum implementations, with focus on performance, testing, and practical integration.

**Key Findings:**
- **Tire Physics:** Pacejka Magic Formula remains industry standard; friction circle model essential for combined slip
- **Suspension:** Spring-damper with anti-roll bars is standard; 60Hz+ update rate required
- **Drivetrain:** PhysX-style torsional coupling provides good balance of accuracy and performance
- **Complexity:** 6/10 for arcade, 8/10 for simulation
- **Performance Target:** 100+ vehicles at 60Hz physics update
- **Network:** Client-side prediction with server reconciliation mandatory for multiplayer

---

## 1. Tire Physics Models

### 1.1 Overview

Tire physics is the foundation of realistic vehicle handling. Modern implementations balance accuracy with computational performance.

### 1.2 Pacejka Magic Formula

**Status:** Industry standard since 1990s, actively used in 2024-2025
**Complexity:** 7/10

The [Pacejka Magic Formula](https://medium.com/@remvoorhuis/how-to-program-realistic-vehicle-physics-for-realtime-environments-games-part-i-simple-b4c2375dc7fa) is an empirical formula describing tire friction forces as a function of slip. Developed by Hans B. Pacejka and funded by Volvo and Audi VAG, it remains the "cheap to compute and battle-tested" choice.

**Key Characteristics:**
- Empirical (curve-fitted from real tire data)
- Separate curves for longitudinal and lateral forces
- Non-linear response to slip angle and slip ratio
- Computationally efficient

**Parameters:**
```rust
struct PacejkaParams {
    peak_friction: f32,      // Peak friction coefficient (μ)
    shape_factor: f32,       // Controls curve sharpness
    stiffness_factor: f32,   // Initial slope
    curvature_factor: f32,   // Peak position
}
```

**Formula (simplified):**
```
F = D * sin(C * atan(B * slip - E * (B * slip - atan(B * slip))))

Where:
  D = peak force
  C = shape factor
  B = stiffness factor
  E = curvature factor
```

**Pros:**
- Fast computation (single trigonometric function)
- Well-documented with extensive real-world data
- Predictable behavior for tuning

**Cons:**
- Requires empirical data for each tire type
- Less intuitive than physical models
- Doesn't capture all edge cases (extreme deformation)

### 1.3 Brush Tire Model

**Status:** Academic/advanced simulations, less common in games
**Complexity:** 8/10

The brush model simulates tire deformation as distributed bristles contacting the road surface. More physically accurate but computationally expensive.

**Key Characteristics:**
- Physics-based (models actual tire deformation)
- Contact patch simulation
- Better at extreme conditions
- Higher computational cost

**Use Cases:**
- High-fidelity simulation (BeamNG.drive)
- Validation/tuning tools
- Special effects (tire deformation visualization)

**Implementation Note:** Most games use simplified brush models or hybrid Pacejka-brush approaches.

### 1.4 Friction Circle and Combined Slip

**Status:** Essential for realistic handling
**Complexity:** 6/10

The [friction circle concept](https://help.functionbay.com/2025/RecurDynHelp/Tire/Tire_ch02_s02_00_index.html) recognizes that a tire has a maximum total grip force, shared between longitudinal (acceleration/braking) and lateral (cornering) forces.

**Mathematical Representation:**
```
(Fx / Fx_max)² + (Fy / Fy_max)² ≤ 1

Where:
  Fx = longitudinal force
  Fy = lateral force
  Fx_max = maximum longitudinal grip
  Fy_max = maximum lateral grip
```

**Implementation Approaches:**

1. **Simple Scaling:**
```rust
let total_force = sqrt(fx * fx + fy * fy);
if total_force > max_grip {
    let scale = max_grip / total_force;
    fx *= scale;
    fy *= scale;
}
```

2. **MFTire 5.2 Weighting:**
Uses empirical magic formula weighting functions with curve-fitted constants to reduce pure Fx and Fy independently.

3. **Peak Grip Scaling:**
Scale inputs by a factor of the peak grip circle, effectively limiting forces around maximum slip.

**Critical Implementation Detail:**
Never simply add longitudinal and lateral forces! A tire can only develop so much total grip at any one time.

### 1.5 Slip Angle and Slip Ratio

**Slip Angle:** Angle difference between where tire is pointed and where it's actually going
**Slip Ratio:** (wheel_spin_speed / vehicle_speed) - 1

```rust
struct TireSlip {
    slip_angle: f32,      // Lateral slip (radians)
    slip_ratio: f32,      // Longitudinal slip (0-1 typically)
    combined_slip: f32,   // Magnitude for friction circle
}

fn calculate_slip_angle(wheel_velocity: Vec3, wheel_forward: Vec3) -> f32 {
    let lateral_velocity = wheel_velocity - wheel_forward * wheel_velocity.dot(wheel_forward);
    let forward_velocity = wheel_velocity.dot(wheel_forward);
    lateral_velocity.magnitude().atan2(forward_velocity.abs())
}

fn calculate_slip_ratio(wheel_angular_velocity: f32, wheel_radius: f32, vehicle_speed: f32) -> f32 {
    let wheel_linear_speed = wheel_angular_velocity * wheel_radius;
    if vehicle_speed.abs() < 0.1 {
        return 0.0; // Avoid division by zero at standstill
    }
    (wheel_linear_speed - vehicle_speed) / vehicle_speed.abs()
}
```

### 1.6 Recommended Tire Implementation

**For Arcade Games (Complexity 3/10):**
```rust
// Simple friction with max force cap
let lateral_force = slip_angle * lateral_stiffness;
let longitudinal_force = slip_ratio * longitudinal_stiffness;
let max_force = max_grip * normal_force;

// Apply friction circle
let total = sqrt(lateral_force² + longitudinal_force²);
if total > max_force {
    lateral_force *= max_force / total;
    longitudinal_force *= max_force / total;
}
```

**For Simulation Games (Complexity 7/10):**
```rust
// Pacejka with combined slip
let fx_pure = pacejka_longitudinal(slip_ratio, normal_force);
let fy_pure = pacejka_lateral(slip_angle, normal_force);

// Combined slip magnitude
let slip_combined = sqrt(slip_ratio² + tan(slip_angle)²);

// Weighting factors (MFTire approach)
let weight_x = slip_ratio / slip_combined;
let weight_y = tan(slip_angle) / slip_combined;

// Final forces with friction circle
let fx = fx_pure * combined_slip_factor(slip_combined) * weight_x;
let fy = fy_pure * combined_slip_factor(slip_combined) * weight_y;
```

---

## 2. Suspension Systems

### 2.1 Overview

Vehicle suspension manages vertical forces, absorbs bumps, and maintains tire contact. Modern game implementations use spring-damper models with anti-roll bars.

### 2.2 Spring-Damper Model

**Status:** Industry standard
**Complexity:** 5/10

[Spring-damper systems](https://vehiclephysics.com/components/vehicle-suspension/) are the foundation of vehicle suspension simulation.

**Physics Equation:**
```
F = k * x + c * dx/dt

Where:
  F = suspension force
  k = spring constant (stiffness) [N/m]
  c = damping constant [N·s/m]
  x = compression distance from equilibrium
  dx/dt = compression velocity
```

**Implementation:**
```rust
struct Suspension {
    spring_stiffness: f32,      // k (N/m) - higher = stiffer
    damping_coefficient: f32,   // c (N·s/m) - higher = more damping
    rest_length: f32,           // Equilibrium length
    max_compression: f32,       // Travel limit (compression)
    max_extension: f32,         // Travel limit (droop)
}

fn calculate_suspension_force(
    suspension: &Suspension,
    current_length: f32,
    compression_velocity: f32,
) -> f32 {
    let compression = suspension.rest_length - current_length;

    // Clamp to travel limits
    let compression = compression.clamp(
        -suspension.max_extension,
        suspension.max_compression
    );

    // Spring force (Hooke's law)
    let spring_force = suspension.spring_stiffness * compression;

    // Damping force
    let damping_force = suspension.damping_coefficient * compression_velocity;

    spring_force + damping_force
}
```

### 2.3 Damping Ratios and Natural Frequency

**Key Tuning Parameters:**

According to [Vehicle Physics Pro documentation](https://vehiclephysics.com/components/vehicle-dynamics/), racing cars typically use:

- **Damping Ratio:** 0.6 - 0.85
  - < 0.6: Underdamped (bouncy)
  - 0.6-0.85: Optimal for racing
  - > 1.0: Overdamped (sluggish)

- **Natural Frequency:**
  - Low frequency (< 2.0 Hz): Soft suspension, more mechanical grip
  - Racing cars: 2.0 - 4.0 Hz
  - F1/high downforce: > 4.0 Hz (very stiff for aero stability)

**Calculation:**
```rust
// Natural frequency (Hz)
let natural_frequency = sqrt(spring_stiffness / vehicle_mass) / (2.0 * PI);

// Damping coefficient from damping ratio
let critical_damping = 2.0 * sqrt(spring_stiffness * vehicle_mass);
let damping_coefficient = damping_ratio * critical_damping;
```

### 2.4 Anti-Roll Bars (Stabilizer Bars)

**Status:** Essential for realistic handling
**Complexity:** 6/10

[Anti-roll bars](https://vehiclephysics.com/components/vehicle-suspension/) connect the two wheels of the same axle, allowing limited freedom between their suspensions. When one wheel compresses, the bar transfers a portion of that force to the other wheel.

**Purpose:**
- Reduce body roll in corners
- Adjust handling balance (understeer/oversteer)
- Maintain more level chassis

**Implementation:**
```rust
struct AntiRollBar {
    stiffness: f32,  // Torsional stiffness [N·m/rad]
}

fn calculate_anti_roll_force(
    arb: &AntiRollBar,
    left_compression: f32,
    right_compression: f32,
    wheel_track: f32,  // Distance between wheels
) -> (f32, f32) {  // (left_force, right_force)
    let compression_diff = left_compression - right_compression;
    let roll_angle = compression_diff / wheel_track;
    let arb_force = arb.stiffness * roll_angle;

    // Force opposes relative compression
    (-arb_force, arb_force)
}
```

**Tuning Guidelines:**
- **Stiffer Front ARB:** Reduces front grip → increases understeer
- **Stiffer Rear ARB:** Reduces rear grip → increases oversteer
- **No ARB:** Maximum mechanical grip but more body roll

### 2.5 Raycast vs. Geometry-Based Suspension

**Raycast (Common in Games):**
```rust
fn raycast_suspension(
    world: &PhysicsWorld,
    wheel_position: Vec3,
    suspension_direction: Vec3,
    max_length: f32,
) -> Option<SuspensionHit> {
    world.raycast(
        wheel_position,
        suspension_direction,
        max_length
    ).map(|hit| SuspensionHit {
        distance: hit.distance,
        normal: hit.normal,
        surface_type: hit.surface_type,
    })
}
```

**Pros:** Fast, simple, works with any collision geometry
**Cons:** Single contact point, can miss complex geometry

**Geometry-Based (BeamNG.drive style):**
- Full wheel collision shape
- Multiple contact points
- Accurate tire deformation
- **Much** more expensive (suitable for soft-body physics)

**Recommendation:** Raycast for most games, geometry-based only for simulation focus.

### 2.6 Recommended Suspension Implementation

**For Arcade Games (Complexity 4/10):**
```rust
// Simple spring-damper, no ARB
let force = spring_stiffness * compression + damping * velocity;
```

**For Simulation Games (Complexity 6/10):**
```rust
// Full spring-damper + ARB
let spring_force = spring_stiffness * compression;
let damper_force = damping_coefficient * velocity;
let arb_force = calculate_anti_roll_force(arb, left_comp, right_comp, track);

let total_force = spring_force + damper_force + arb_force;
```

---

## 3. Drivetrain Simulation

### 3.1 Overview

Drivetrain simulation connects engine power to wheels through gearbox and differential. Modern implementations use torsional coupling for realistic power delivery.

### 3.2 Engine Torque Curve

**Status:** Essential for realistic acceleration
**Complexity:** 5/10

[Engine torque curves](https://vehiclephysics.com/blocks/engine/) define power delivery across RPM range.

**Implementation:**
```rust
struct Engine {
    idle_rpm: f32,          // Minimum stable RPM (800-1000)
    peak_torque_rpm: f32,   // RPM at peak torque (4000-6000)
    max_rpm: f32,           // Redline (6000-9000)
    peak_torque: f32,       // Maximum torque [N·m]
    curve_bias: f32,        // Shape parameter (0.5-2.0)
    inertia: f32,           // Rotational inertia [kg·m²]
    friction_coefficient: f32, // Engine friction
}

fn calculate_engine_torque(engine: &Engine, rpm: f32, throttle: f32) -> f32 {
    if rpm < engine.idle_rpm || rpm > engine.max_rpm {
        return 0.0;
    }

    // Normalized RPM (0-1)
    let rpm_normalized = (rpm - engine.idle_rpm) / (engine.max_rpm - engine.idle_rpm);
    let peak_normalized = (engine.peak_torque_rpm - engine.idle_rpm) / (engine.max_rpm - engine.idle_rpm);

    // Torque curve shape (simplified)
    let torque_factor = if rpm_normalized < peak_normalized {
        // Rising to peak
        (rpm_normalized / peak_normalized).powf(engine.curve_bias)
    } else {
        // Falling from peak
        1.0 - ((rpm_normalized - peak_normalized) / (1.0 - peak_normalized)).powf(engine.curve_bias) * 0.3
    };

    // Apply throttle
    let base_torque = engine.peak_torque * torque_factor;
    let engine_friction = engine.friction_coefficient * rpm;

    (base_torque * throttle - engine_friction).max(0.0)
}
```

**Electric Motor Variant:**

Electric motors have [constant torque from 0 RPM](https://x-engineer.org/electric-vehicle-motor-torque-power-curves/) up to base speed, then constant power (torque decreases).

```rust
fn calculate_electric_motor_torque(motor: &ElectricMotor, rpm: f32, throttle: f32) -> f32 {
    if rpm < motor.base_speed_rpm {
        // Constant torque region
        motor.max_torque * throttle
    } else if rpm < motor.max_rpm {
        // Constant power region (torque decreases)
        let power = motor.max_torque * motor.base_speed_rpm;
        (power / rpm) * throttle
    } else {
        0.0
    }
}
```

### 3.3 Gearbox and Gear Ratios

**Status:** Standard in all vehicle games
**Complexity:** 4/10

```rust
struct Gearbox {
    gear_ratios: Vec<f32>,     // Ratio for each gear (higher = more torque, less speed)
    final_drive_ratio: f32,    // Differential ratio
    shift_time: f32,           // Time to change gears (seconds)
    current_gear: i32,         // 0 = neutral, -1 = reverse, 1+ = forward
    clutch_engagement: f32,    // 0-1 (0 = disengaged, 1 = engaged)
}

fn calculate_drive_force(
    engine_torque: f32,
    gearbox: &Gearbox,
    wheel_radius: f32,
    transmission_efficiency: f32,  // 0.85-0.95 typically
) -> f32 {
    if gearbox.current_gear == 0 {
        return 0.0; // Neutral
    }

    let gear_ratio = gearbox.gear_ratios[gearbox.current_gear.abs() as usize - 1];
    let total_ratio = gear_ratio * gearbox.final_drive_ratio;

    // F = (T * ratio * efficiency) / wheel_radius
    let force = (engine_torque * total_ratio * transmission_efficiency) / wheel_radius;

    // Apply clutch engagement
    force * gearbox.clutch_engagement * gearbox.current_gear.signum() as f32
}
```

**Automatic Shifting:**
```rust
fn auto_shift_logic(engine: &Engine, gearbox: &mut Gearbox, rpm: f32, throttle: f32) {
    const SHIFT_UP_RPM_RATIO: f32 = 0.85;    // Shift up at 85% of max RPM
    const SHIFT_DOWN_RPM_RATIO: f32 = 0.40;  // Shift down at 40% of max RPM

    if throttle > 0.5 && rpm > engine.max_rpm * SHIFT_UP_RPM_RATIO {
        shift_up(gearbox);
    } else if throttle < 0.3 && rpm < engine.max_rpm * SHIFT_DOWN_RPM_RATIO {
        shift_down(gearbox);
    }
}
```

### 3.4 Differential Types

**Status:** Critical for handling characteristics
**Complexity:** 6/10

The [differential type](https://gamedev.net/forums/topic/695333-limited-slip-differential-and-friends/5394027/) has a great influence on vehicle handling.

#### Open Differential

**Behavior:** Sends equal torque to both wheels; torque limited by wheel with least grip
**Use Case:** Standard road cars

```rust
fn open_differential(total_torque: f32) -> (f32, f32) {
    // Split torque equally
    let torque_per_wheel = total_torque / 2.0;
    (torque_per_wheel, torque_per_wheel)
}
```

**Problem:** When one wheel slips, both receive minimal torque (torque = torque needed to spin slipping wheel).

#### Locked Differential

**Behavior:** Both wheels spin at same rate
**Use Case:** Off-road, drifting

```rust
fn locked_differential(total_torque: f32) -> (f32, f32) {
    // Force equal wheel speeds, torque distributes based on resistance
    // In practice, implemented by constraining wheel angular velocities
    (total_torque / 2.0, total_torque / 2.0)  // Simplified
}
```

**Problem:** Makes steering harder (wheels fight each other in turns), causes tire scrubbing.

#### Limited-Slip Differential (LSD)

**Behavior:** Acts like open diff normally, transfers torque when slip detected
**Use Case:** Performance/racing cars

```rust
struct LimitedSlipDiff {
    bias_ratio: f32,  // 1.0 = open, 5.0 = aggressive LSD, inf = locked
    preload: f32,     // Constant friction torque
}

fn limited_slip_differential(
    lsd: &LimitedSlipDiff,
    total_torque: f32,
    left_wheel_speed: f32,
    right_wheel_speed: f32,
) -> (f32, f32) {
    let speed_diff = (left_wheel_speed - right_wheel_speed).abs();

    // Calculate torque split based on bias ratio
    let base_split = total_torque / 2.0;

    if speed_diff < 0.1 {
        // No slip, act like open diff
        return (base_split, base_split);
    }

    // Determine which wheel is slipping
    let (faster_wheel_torque, slower_wheel_torque) = if left_wheel_speed > right_wheel_speed {
        let slower = base_split * lsd.bias_ratio;
        let faster = total_torque - slower;
        (faster, slower)
    } else {
        let slower = base_split * lsd.bias_ratio;
        let faster = total_torque - slower;
        (slower, faster)
    };

    if left_wheel_speed > right_wheel_speed {
        (faster_wheel_torque, slower_wheel_torque)
    } else {
        (slower_wheel_torque, faster_wheel_torque)
    }
}
```

**Bias Ratio Guide:**
- 1.0 = Open differential
- 2.0-3.0 = Street LSD
- 4.0-6.0 = Racing LSD
- >10.0 = Nearly locked

**Configuration in Games:**
According to racing game implementations, `POWER=0.0` represents open diff, `0.0-1.0` represents LSD strength, and `1.0` is solid (locked).

### 3.5 PhysX-Style Torsional Drivetrain

**Status:** Used in PhysX, Unreal Chaos
**Complexity:** 7/10

The [NVIDIA PhysX approach](https://nvidia-omniverse.github.io/PhysX/physx/5.1.0/docs/Vehicles.html) models drivetrain as torsional springs connecting components.

**Architecture:**
```
Engine ←→ Clutch ←→ Gearbox ←→ Differential ←→ Wheels
  (inertia)  (spring)  (ratio)    (split)      (inertia)
```

**Torsional Spring Model:**
```rust
struct TorsionalCoupling {
    stiffness: f32,        // Spring constant [N·m/rad]
    damping: f32,          // Damping [N·m·s/rad]
}

fn calculate_coupling_torque(
    coupling: &TorsionalCoupling,
    angular_vel_1: f32,
    angular_vel_2: f32,
    gear_ratio: f32,
) -> f32 {
    let relative_vel = angular_vel_1 - angular_vel_2 * gear_ratio;
    coupling.stiffness * relative_vel + coupling.damping * relative_vel
}
```

**Update Loop:**
```rust
fn update_drivetrain(dt: f32) {
    // 1. Calculate clutch torque (torsional spring between engine and gearbox)
    let clutch_torque = calculate_coupling_torque(
        clutch,
        engine.angular_velocity,
        gearbox_input_shaft.angular_velocity,
        1.0
    ) * clutch_engagement;

    // 2. Apply torques to engine
    let engine_torque = calculate_engine_torque(engine, engine.rpm, throttle);
    let net_engine_torque = engine_torque - clutch_torque - engine_friction;
    engine.angular_velocity += (net_engine_torque / engine.inertia) * dt;

    // 3. Propagate through gearbox
    let gearbox_output_torque = clutch_torque * current_gear_ratio;

    // 4. Split through differential
    let (left_torque, right_torque) = differential_split(gearbox_output_torque);

    // 5. Apply to wheels (wheels also receive ground reaction torques)
    left_wheel.angular_velocity += (left_torque - left_ground_torque) / wheel.inertia * dt;
    right_wheel.angular_velocity += (right_torque - right_ground_torque) / wheel.inertia * dt;
}
```

**Advantages:**
- Realistic power delivery with gear chatter
- Automatic stall behavior
- Wheel spin affects engine (engine braking)
- Supports manual clutch control

**Disadvantages:**
- More complex than direct force application
- Requires careful tuning of spring stiffness
- Can be numerically unstable with very stiff couplings

### 3.6 Recommended Drivetrain Implementation

**For Arcade Games (Complexity 4/10):**
```rust
// Direct force application, no drivetrain dynamics
let drive_force = throttle * max_force * gear_ratio;
apply_force_to_wheels(drive_force);
```

**For Simulation Games (Complexity 7/10):**
```rust
// Full PhysX-style torsional drivetrain
update_engine_dynamics(dt);
update_clutch_coupling(dt);
update_gearbox(dt);
update_differential(dt);
update_wheel_dynamics(dt);
```

---

## 4. Racing Game Reference Implementations

### 4.1 BeamNG.drive

**Physics Approach:** Soft-body physics simulation
**Complexity:** 10/10

[BeamNG.drive](https://en.wikipedia.org/wiki/BeamNG.drive) uses a unique soft-body physics engine where vehicles are simulated as networks of interconnected nodes and beams forming an invisible skeleton with realistic weights and masses.

**Key Features:**
- Real-time soft-body deformation
- Realistic crash physics
- Advanced tire model (brush-type)
- Full drivetrain simulation
- Suspension geometry dynamics

**Performance Characteristics:**
- Very CPU intensive (limits to ~10-20 active vehicles)
- Each node/beam adds computational cost
- Requires careful tuning to prevent instability

**Limitations:**
- High downforce cars can shake themselves apart
- Each simulated node needs certain weight for stability
- Not suitable for large multiplayer

**Takeaway for Game Engines:**
Soft-body physics is feasible for small-scale simulations (1-4 vehicles) but impractical for massively multiplayer or large AI fleets. Use for special destruction effects only.

### 4.2 Forza Motorsport

**Physics Approach:** Rigid-body with advanced tire model
**Complexity:** 8/10

[Forza Motorsport](https://traxion.gg/new-forza-motorsport-gameplay-footage-showcases-impressive-driving-physics/) features detailed chassis flex simulation, suspension geometry characteristics, and intricate tire physics with heat transfer and wear.

**Key Features:**
- Tire thermal model (carcass and surface temperature)
- Suspension geometry (camber, caster, toe changes)
- Aerodynamic load simulation
- 360Hz physics update rate (4x visual framerate at 90fps)

**Performance Optimization:**
- Rigid body core (not soft-body like BeamNG)
- Tire model simplified for real-time (not full FEA)
- LOD system for AI vehicles

**Takeaway for Game Engines:**
High-frequency physics updates (120-360Hz) are key to stability and accuracy. Use simpler models updated more frequently rather than complex models at low rates.

### 4.3 Gran Turismo

**Physics Approach:** Simulation-focused rigid-body
**Complexity:** 8/10

Gran Turismo emphasizes accurate vehicle dynamics with real-world data from manufacturers.

**Key Features:**
- Real car data (weight distribution, power curves, suspension geometry)
- Advanced tire model with temperature and wear
- Suspension modeling with accurate geometry
- Aerodynamic effects

**Performance Characteristics:**
- 60Hz physics for player vehicle
- Simplified physics for distant AI
- Predictive models for networked opponents

**Takeaway for Game Engines:**
LOD system for physics: full simulation for player + nearby vehicles, simplified models for distant vehicles.

### 4.4 rFactor 2

**Physics Approach:** Professional simulation platform
**Complexity:** 9/10

[rFactor 2](https://simracingcockpit.gg/which-sims-offer-the-best-car-physics/) is considered the simulator that comes closest to delivering a match for real-world vehicle dynamics.

**Key Features:**
- Chassis flex simulation
- Suspension geometry with accurate kinematics
- Real Road technology (track surface changes with tire rubber)
- Advanced tire thermal model
- Wet weather physics with puddle formation

**Performance Characteristics:**
- 400Hz tire physics update
- 200Hz suspension/chassis update
- Complex real-time calculations limit AI count

**Takeaway for Game Engines:**
For professional simulation, invest in multi-rate integration (different update rates for different subsystems).

### 4.5 iRacing

**Physics Approach:** Professional simulation with focus on multiplayer
**Complexity:** 9/10

iRacing uses LIDAR-scanned tracks and cars for extreme accuracy, with a sophisticated wet weather model added in 2024.

**Key Features:**
- LIDAR-accurate track surfaces
- Dynamic puddle formation
- Real-time tire model
- Network-optimized physics

**Performance Characteristics:**
- Client-side prediction for local vehicle
- Server-authoritative for validation
- Interpolation for remote vehicles

**Takeaway for Game Engines:**
For multiplayer, client-side prediction is mandatory. Server validates critical events (collisions, lap times) but doesn't simulate all vehicles fully.

---

## 5. Modern Physics Engine Implementations

### 5.1 NVIDIA PhysX Vehicles

**Status:** Legacy (deprecated in favor of custom solutions)
**Complexity:** 7/10

PhysX 5.1 provides a [complete vehicle SDK](https://nvidia-omniverse.github.io/PhysX/physx/5.1.0/docs/Vehicles.html) with engine, clutch, differential, and gearing.

**Architecture:**
- Torsional clutch at center of drive model
- Couples wheels and engine via rotational speed differences
- Supports any torque split (not just equal split)
- Limited slip on up to 4 wheels

**Advantages:**
- Battle-tested, used in many shipping games
- Good documentation
- Handles edge cases (stalling, wheel spin)

**Disadvantages:**
- Legacy API (PhysX moving away from built-in vehicles)
- Limited customization
- Performance not optimized for 100+ vehicles

**Current Status (2024-2025):**
PhysX vehicle SDK still available but not actively developed. Most new projects use custom implementations.

### 5.2 Unreal Engine Chaos Vehicles

**Status:** Current (default in UE5+)
**Complexity:** 7/10

[Chaos Vehicles](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-vehicles) replaced PhysX as the default vehicle system in Unreal Engine 5.

**Key Improvements Over PhysX:**
- More deterministic (better for multiplayer)
- Network-friendly (predictable across frame rates)
- More stable at high speeds
- Better performance scaling

**Architecture:**
- Component-based (ChaosWheeledVehicleComponent)
- Blueprint-friendly
- Custom wheel classes
- Animation blueprint integration

**Migration Path:**
Unreal provides [conversion tools](https://dev.epicgames.com/documentation/en-us/unreal-engine/how-to-convert-physx-vehicles-to-chaos-in-unreal-engine) from PhysX vehicles:
1. Create ChaosVehicleWheel classes
2. Replace PhysX component with ChaosWheeledVehicleComponent
3. Update animation blueprint (WheelHandler → WheelController)
4. Update Blueprint references

**Advantages:**
- First-party support in UE5
- Better multiplayer support
- Actively maintained

**Disadvantages:**
- Still marked "experimental" in some UE versions
- Less mature than PhysX
- Breaking changes possible

**Takeaway for Game Engines:**
Determinism and network-friendliness should be primary goals. Frame-rate independence is critical for multiplayer.

### 5.3 Vehicle Physics Pro (Unity Asset)

**Status:** Active commercial product (2024-2025)
**Complexity:** 8/10

[Vehicle Physics Pro](https://vehiclephysics.com/about/features/) is a professional Unity vehicle simulation kit used in automotive engineering and game development.

**Key Features:**
- Complete vehicle dynamics model
- High degree of customization
- Supports passenger cars, racing cars, heavy equipment
- Realistic torque curve, inertia, friction
- Engine ignition, stall, fuel consumption
- Multiple tire friction models (Flat, Linear, Smooth, Parametric, Pacejka)

**Performance:**
- Optimized for real-time (60Hz+)
- Supports multiple vehicles
- LOD system for distant vehicles

**Tuning Tools:**
- Visual torque curve editor
- Suspension frequency calculator
- Real-time telemetry
- Parameter sweeps

**Takeaway for Game Engines:**
Invest in tuning tools early. Visual editors for torque curves, suspension, and tire parameters save massive development time.

---

## 6. Implementation Complexity Scale

### 6.1 Arcade Implementation (Complexity 3/10)

**Time Estimate:** 2-4 weeks for basic implementation
**Team Size:** 1 programmer

**Components:**
- Simple friction model (linear + cap)
- Direct force application (no drivetrain dynamics)
- Basic spring-damper suspension (no ARB)
- Raycast collision
- No tire temperature/wear

**Code Estimate:** ~1,500 lines

**Example (simplified):**
```rust
struct ArcadeVehicle {
    mass: f32,
    max_speed: f32,
    acceleration: f32,
    turn_rate: f32,
    suspension_stiffness: f32,
}

fn update_arcade_vehicle(vehicle: &mut ArcadeVehicle, input: &Input, dt: f32) {
    // Direct velocity control
    let forward_input = input.throttle - input.brake;
    vehicle.velocity += vehicle.acceleration * forward_input * dt;
    vehicle.velocity = vehicle.velocity.clamp(0.0, vehicle.max_speed);

    // Direct rotation
    vehicle.rotation += vehicle.turn_rate * input.steering * dt;

    // Simple suspension
    let ground_height = raycast_ground(vehicle.position);
    let compression = (ground_height - vehicle.position.y).max(0.0);
    let suspension_force = compression * vehicle.suspension_stiffness;
    vehicle.velocity.y += (suspension_force / vehicle.mass) * dt;
}
```

**Suitable For:**
- Mobile games
- Casual racing games
- Non-racing games with vehicles (GTA-style)

### 6.2 Simcade Implementation (Complexity 6/10)

**Time Estimate:** 2-3 months for full implementation
**Team Size:** 1-2 programmers

**Components:**
- Pacejka tire model (simplified)
- Friction circle
- Spring-damper + ARB suspension
- Basic drivetrain (engine torque curve, gearbox, open diff)
- Raycast collision
- Basic aerodynamics (drag + downforce)

**Code Estimate:** ~5,000 lines

**Example:**
```rust
struct SimcadeVehicle {
    // Physics
    rigidbody: RigidBody,
    wheels: [Wheel; 4],
    suspension: [Suspension; 4],

    // Drivetrain
    engine: Engine,
    gearbox: Gearbox,
    differential: OpenDifferential,

    // Tuning
    tire_friction: PacejkaParams,
    aero_drag: f32,
    aero_downforce: f32,
}

fn update_simcade_vehicle(vehicle: &mut SimcadeVehicle, input: &Input, dt: f32) {
    // 1. Update engine
    let engine_torque = calculate_engine_torque(&vehicle.engine, input.throttle);

    // 2. Apply through drivetrain
    let wheel_torque = engine_torque * vehicle.gearbox.current_ratio;
    let (left_torque, right_torque) = vehicle.differential.split(wheel_torque);

    // 3. Update each wheel
    for wheel in &mut vehicle.wheels {
        update_wheel_physics(wheel, &vehicle.tire_friction, dt);
    }

    // 4. Update suspension
    for suspension in &mut vehicle.suspension {
        let force = calculate_suspension_force(suspension, dt);
        vehicle.rigidbody.add_force(force);
    }

    // 5. Aerodynamics
    let aero_force = calculate_aero_forces(vehicle.rigidbody.velocity, vehicle.aero_drag, vehicle.aero_downforce);
    vehicle.rigidbody.add_force(aero_force);
}
```

**Suitable For:**
- Racing games (Forza Horizon, Need for Speed style)
- Vehicle combat games
- Open-world games with driving focus

### 6.3 Simulation Implementation (Complexity 8/10)

**Time Estimate:** 6-12 months for full implementation
**Team Size:** 2-4 programmers + vehicle dynamics expert

**Components:**
- Full Pacejka tire model with combined slip
- Tire thermal model (temperature affects grip)
- Tire wear simulation
- Spring-damper + ARB + geometry changes (camber, caster, toe)
- Full drivetrain with torsional coupling
- Limited-slip differential
- Aerodynamic load map (downforce varies by ride height, pitch, yaw)
- Chassis flex (simplified)

**Code Estimate:** ~15,000-20,000 lines

**Additional Requirements:**
- Real vehicle data (if simulating real cars)
- Tuning tools (visual editors, telemetry)
- Validation against real-world data
- Multi-rate integration (different update rates for subsystems)

**Suitable For:**
- Professional simulators (iRacing, rFactor 2)
- Automotive engineering tools
- High-fidelity racing games (Gran Turismo, Assetto Corsa)

### 6.4 Soft-Body Implementation (Complexity 10/10)

**Time Estimate:** 1-2+ years
**Team Size:** 4-8 programmers + physics specialists

**Components:**
- Everything from Simulation level
- Soft-body deformation (node-beam network)
- Fracture/breakage simulation
- Fluid simulation (for fluids like oil, fuel)
- Advanced collision detection for deformable bodies

**Code Estimate:** ~50,000+ lines

**Suitable For:**
- BeamNG.drive style simulation
- Destruction-focused games
- Engineering validation tools

**Performance Warning:**
Soft-body physics scales poorly. Limit to 1-4 vehicles for real-time.

---

## 7. Performance Requirements and Optimization

### 7.1 Target Performance Metrics

Based on industry standards and 2024-2025 racing games:

| Metric | Arcade | Simcade | Simulation |
|--------|--------|---------|------------|
| **Physics Update Rate** | 60 Hz | 60-120 Hz | 120-400 Hz |
| **Vehicles Per Frame (Player Platform)** | 50+ | 30+ | 10-20 |
| **Vehicles Per Frame (Server)** | 200+ | 100+ | 50+ |
| **Frame Time Budget (Physics)** | < 2ms | < 4ms | < 8ms |
| **Memory Per Vehicle** | < 50 KB | < 200 KB | < 1 MB |
| **LOD Distances** | Simple always | 2-3 LODs | 3-4 LODs |

### 7.2 Update Rate Guidelines

**60 Hz (16.67ms):**
- Minimum for stable vehicle simulation
- Suitable for arcade games
- Tire forces can be less accurate

**120 Hz (8.33ms):**
- Recommended for simcade
- Better tire contact stability
- Smoother suspension response

**240+ Hz (< 4.17ms):**
- Professional simulation
- Very accurate tire model
- Suspension kinematics stability
- Required for high-speed vehicles (>300 km/h)

**Multi-Rate Integration:**
```rust
fn update_vehicle_multi_rate(vehicle: &mut Vehicle, dt: f32) {
    const TIRE_SUBSTEPS: u32 = 4;
    const SUSPENSION_SUBSTEPS: u32 = 2;

    let tire_dt = dt / TIRE_SUBSTEPS as f32;
    let suspension_dt = dt / SUSPENSION_SUBSTEPS as f32;

    // High-frequency tire updates
    for _ in 0..TIRE_SUBSTEPS {
        update_tire_forces(vehicle, tire_dt);
    }

    // Medium-frequency suspension
    for _ in 0..SUSPENSION_SUBSTEPS {
        update_suspension(vehicle, suspension_dt);
    }

    // Low-frequency drivetrain
    update_drivetrain(vehicle, dt);
    update_aerodynamics(vehicle, dt);
}
```

### 7.3 Performance Optimization Strategies

#### 7.3.1 LOD System for Physics

**Concept:** Reduce physics complexity based on distance from player

```rust
enum VehiclePhysicsLOD {
    Full,      // Player vehicle + close AI
    Medium,    // Nearby AI (simplified tire model)
    Simple,    // Distant AI (direct force, no suspension)
    Kinematic, // Very far (interpolated position, no physics)
}

fn get_physics_lod(distance_to_player: f32) -> VehiclePhysicsLOD {
    match distance_to_player {
        d if d < 50.0  => VehiclePhysicsLOD::Full,
        d if d < 200.0 => VehiclePhysicsLOD::Medium,
        d if d < 500.0 => VehiclePhysicsLOD::Simple,
        _              => VehiclePhysicsLOD::Kinematic,
    }
}
```

**LOD Implementation:**
- **Full:** Complete physics (tire model, suspension, drivetrain)
- **Medium:** Simplified tire (linear instead of Pacejka), no drivetrain dynamics
- **Simple:** Direct force application, raycast suspension only
- **Kinematic:** No physics, interpolate position from AI waypoints

**Performance Gain:** 3-5x more vehicles at same cost

#### 7.3.2 SIMD Optimization

Modern vehicle physics benefits significantly from SIMD:

```rust
// Example: Update 4 wheels simultaneously with SIMD
use std::simd::f32x4;

fn update_wheels_simd(wheels: &mut [Wheel; 4], dt: f32) {
    // Load all wheel data into SIMD registers
    let compressions = f32x4::from_array([
        wheels[0].suspension_compression,
        wheels[1].suspension_compression,
        wheels[2].suspension_compression,
        wheels[3].suspension_compression,
    ]);

    let velocities = f32x4::from_array([
        wheels[0].suspension_velocity,
        wheels[1].suspension_velocity,
        wheels[2].suspension_velocity,
        wheels[3].suspension_velocity,
    ]);

    // Calculate all suspension forces in parallel
    let spring_forces = compressions * f32x4::splat(SPRING_STIFFNESS);
    let damper_forces = velocities * f32x4::splat(DAMPING_COEFF);
    let total_forces = spring_forces + damper_forces;

    // Store results
    let forces = total_forces.to_array();
    for i in 0..4 {
        wheels[i].apply_force(forces[i]);
    }
}
```

**Performance Gain:** 2-4x for vectorizable operations

#### 7.3.3 Spatial Partitioning

Use spatial partitioning (grid, octree) to:
- Quickly find nearby vehicles for LOD decisions
- Optimize collision detection
- Cull vehicles outside interest area

```rust
struct VehicleSpatialGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<VehicleId>>,
}

impl VehicleSpatialGrid {
    fn get_nearby_vehicles(&self, position: Vec3, radius: f32) -> Vec<VehicleId> {
        let cell_x = (position.x / self.cell_size) as i32;
        let cell_y = (position.z / self.cell_size) as i32;
        let cell_radius = (radius / self.cell_size).ceil() as i32;

        let mut nearby = Vec::new();
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                if let Some(cell) = self.cells.get(&(cell_x + dx, cell_y + dy)) {
                    nearby.extend_from_slice(cell);
                }
            }
        }
        nearby
    }
}
```

#### 7.3.4 Parallelization

Vehicle physics is embarrassingly parallel:

```rust
use rayon::prelude::*;

fn update_all_vehicles_parallel(vehicles: &mut [Vehicle], dt: f32) {
    vehicles.par_iter_mut().for_each(|vehicle| {
        update_vehicle_physics(vehicle, dt);
    });
}
```

**Performance Gain:** Near-linear scaling with CPU cores (3-8x on modern CPUs)

**Caveat:** Collisions between vehicles require synchronization. Use:
- Broad-phase parallel (find potential collisions)
- Narrow-phase sequential (resolve collisions)

### 7.4 Profiling and Benchmarking

**Key Metrics to Track:**

1. **Per-Vehicle Cost:**
   - Time to update one vehicle (µs)
   - Memory per vehicle (KB)

2. **Total Physics Cost:**
   - Total physics time per frame (ms)
   - Percentage of frame budget

3. **Subsystem Breakdown:**
   - Tire physics (%)
   - Suspension (%)
   - Drivetrain (%)
   - Collision (%)

**Benchmarking Setup:**
```rust
fn benchmark_vehicle_update() {
    let mut vehicle = create_test_vehicle();
    let iterations = 10000;

    let start = std::time::Instant::now();
    for _ in 0..iterations {
        update_vehicle_physics(&mut vehicle, 1.0 / 60.0);
    }
    let elapsed = start.elapsed();

    let us_per_update = elapsed.as_micros() / iterations;
    println!("Vehicle update: {} µs", us_per_update);
    println!("Max vehicles at 60Hz (16ms budget): {}", 16000 / us_per_update);
}
```

**Target Performance (2024-2025 Hardware):**

| Implementation | Update Time (µs) | Vehicles @ 60Hz (16ms budget) |
|----------------|------------------|-------------------------------|
| Arcade         | 20-50            | 300-800                       |
| Simcade        | 100-200          | 80-160                        |
| Simulation     | 500-1000         | 16-32                         |

### 7.5 Memory Optimization

**Typical Memory Layout:**

```rust
// Bad: Scattered data (cache misses)
struct VehicleBad {
    position: Vec3,          // 12 bytes
    velocity: Vec3,          // 12 bytes
    rotation: Quat,          // 16 bytes
    wheels: Box<[Wheel; 4]>, // Heap allocation! Cache miss!
    engine: Box<Engine>,     // Another heap allocation!
}

// Good: Contiguous data (cache friendly)
struct VehicleGood {
    position: Vec3,          // 12 bytes
    velocity: Vec3,          // 12 bytes
    rotation: Quat,          // 16 bytes
    wheels: [Wheel; 4],      // Inline, 4 * sizeof(Wheel)
    engine: Engine,          // Inline, sizeof(Engine)
}
```

**Structure-of-Arrays (SoA) for Massive Parallelism:**

```rust
// Array-of-Structures (AoS) - traditional
struct VehicleArrayAoS {
    vehicles: Vec<Vehicle>,  // [V1, V2, V3, ...]
}

// Structure-of-Arrays (SoA) - better for SIMD
struct VehicleArraySoA {
    positions: Vec<Vec3>,    // [P1, P2, P3, ...]
    velocities: Vec<Vec3>,   // [V1, V2, V3, ...]
    rotations: Vec<Quat>,    // [R1, R2, R3, ...]
    // ... etc
}
```

**Performance Gain:** 20-40% improvement for large vehicle counts (100+) when using SIMD

---

## 8. Testing and Validation

### 8.1 Unit Testing

**Key Tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pacejka_zero_slip() {
        let params = PacejkaParams::default();
        let force = pacejka_force(&params, 0.0, 1000.0);
        assert!(force.abs() < 0.1, "Zero slip should produce near-zero force");
    }

    #[test]
    fn test_friction_circle() {
        let fx = 5000.0;
        let fy = 5000.0;
        let max_grip = 6000.0;

        let (fx_limited, fy_limited) = apply_friction_circle(fx, fy, max_grip);
        let total = (fx_limited * fx_limited + fy_limited * fy_limited).sqrt();

        assert!((total - max_grip).abs() < 0.1, "Total force should equal max grip");
    }

    #[test]
    fn test_suspension_equilibrium() {
        let suspension = Suspension {
            spring_stiffness: 20000.0,
            damping_coefficient: 2000.0,
            rest_length: 0.3,
            max_compression: 0.15,
            max_extension: 0.15,
        };

        // At rest length with zero velocity, force should be zero
        let force = calculate_suspension_force(&suspension, 0.3, 0.0);
        assert!(force.abs() < 0.1);
    }

    #[test]
    fn test_engine_torque_curve() {
        let engine = Engine::default();

        // Below idle RPM
        assert_eq!(calculate_engine_torque(&engine, 500.0, 1.0), 0.0);

        // At peak RPM
        let peak_torque = calculate_engine_torque(&engine, engine.peak_torque_rpm, 1.0);
        assert!(peak_torque >= engine.peak_torque * 0.95);

        // Above max RPM
        assert_eq!(calculate_engine_torque(&engine, engine.max_rpm + 100.0, 1.0), 0.0);
    }
}
```

### 8.2 Integration Testing

**Test Scenarios:**

1. **Straight-Line Acceleration:**
   - Verify vehicle reaches expected top speed
   - Check acceleration curve matches engine power
   - Validate gear shifts occur at correct RPM

2. **Cornering Stability:**
   - Ensure vehicle doesn't flip at reasonable speeds
   - Verify understeer/oversteer balance
   - Check friction circle limits are respected

3. **Suspension Response:**
   - Test bump absorption
   - Verify no suspension "pogoing" (oscillation)
   - Check bottoming out behavior

4. **Edge Cases:**
   - High-speed collision recovery
   - Wheel completely off ground
   - Extreme steering inputs
   - Rapid throttle changes

```rust
#[test]
fn test_straight_line_acceleration() {
    let mut vehicle = create_test_vehicle();
    let dt = 1.0 / 60.0;
    let input = Input { throttle: 1.0, brake: 0.0, steering: 0.0 };

    let mut velocity_log = Vec::new();

    // Simulate 10 seconds
    for _ in 0..(10.0 / dt) as usize {
        update_vehicle_physics(&mut vehicle, &input, dt);
        velocity_log.push(vehicle.velocity.length());
    }

    // Check acceleration curve is monotonic (always increasing or stable)
    for i in 1..velocity_log.len() {
        assert!(velocity_log[i] >= velocity_log[i-1] - 0.1,
                "Velocity should not decrease during acceleration");
    }

    // Check reaches reasonable top speed (within 10% of theoretical)
    let final_velocity = velocity_log.last().unwrap();
    let expected_top_speed = calculate_theoretical_top_speed(&vehicle);
    assert!((final_velocity - expected_top_speed).abs() < expected_top_speed * 0.1);
}
```

### 8.3 Property-Based Testing

Use property-based testing for invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_friction_circle_never_exceeds_max(
        fx in -10000.0..10000.0f32,
        fy in -10000.0..10000.0f32,
        max_grip in 1.0..20000.0f32,
    ) {
        let (fx_limited, fy_limited) = apply_friction_circle(fx, fy, max_grip);
        let total = (fx_limited * fx_limited + fy_limited * fy_limited).sqrt();
        prop_assert!(total <= max_grip + 0.01); // Allow small numerical error
    }

    #[test]
    fn test_suspension_force_continuous(
        compression in -0.3..0.3f32,
        velocity in -10.0..10.0f32,
    ) {
        let suspension = Suspension::default();
        let force1 = calculate_suspension_force(&suspension, compression, velocity);
        let force2 = calculate_suspension_force(&suspension, compression + 0.001, velocity);

        // Force should be continuous (no sudden jumps)
        prop_assert!((force1 - force2).abs() < 100.0);
    }
}
```

### 8.4 Real-World Validation

For simulation-grade physics:

1. **Benchmark Against Real Data:**
   - 0-60 mph time
   - Top speed
   - Braking distance (60-0)
   - Lateral G in corners (skidpad test)
   - Lap times (if track data available)

2. **Telemetry Comparison:**
   - Compare simulated telemetry (speed, throttle, RPM) with real race data
   - Validate tire temperatures match real-world patterns
   - Check suspension travel aligns with real vehicle

3. **Professional Driver Feedback:**
   - For high-fidelity sims, professional driver validation is critical
   - Subjective feel often reveals issues metrics miss

**Example Validation Test:**
```rust
#[test]
fn test_porsche_911_gt3_acceleration() {
    let mut vehicle = create_porsche_911_gt3();
    let measured_0_to_60 = measure_0_to_60_time(&mut vehicle);

    // Real Porsche 911 GT3: 3.2 seconds (0-60 mph)
    // Allow 5% tolerance for simulation differences
    const REAL_WORLD_TIME: f32 = 3.2;
    const TOLERANCE: f32 = 0.16; // 5%

    assert!(
        (measured_0_to_60 - REAL_WORLD_TIME).abs() < TOLERANCE,
        "0-60 time: {:.2}s, expected: {:.2}s ± {:.2}s",
        measured_0_to_60, REAL_WORLD_TIME, TOLERANCE
    );
}
```

### 8.5 Stress Testing

**Performance Stress Tests:**

```rust
#[test]
fn stress_test_100_vehicles() {
    let mut vehicles: Vec<Vehicle> = (0..100).map(|_| create_test_vehicle()).collect();
    let dt = 1.0 / 60.0;

    let start = std::time::Instant::now();
    let iterations = 600; // 10 seconds of simulation

    for _ in 0..iterations {
        for vehicle in &mut vehicles {
            update_vehicle_physics(vehicle, &Input::default(), dt);
        }
    }

    let elapsed = start.elapsed();
    let avg_frame_time = elapsed.as_secs_f32() / iterations as f32;

    println!("Average frame time (100 vehicles): {:.2}ms", avg_frame_time * 1000.0);
    assert!(avg_frame_time < 0.016, "Should maintain 60 FPS with 100 vehicles");
}
```

**Stability Stress Tests:**

```rust
#[test]
fn stability_test_extreme_inputs() {
    let mut vehicle = create_test_vehicle();
    let dt = 1.0 / 60.0;

    // Apply random extreme inputs for 60 seconds
    let mut rng = rand::thread_rng();
    for _ in 0..(60.0 / dt) as usize {
        let input = Input {
            throttle: rng.gen_range(-1.0..1.0),
            brake: rng.gen_range(0.0..1.0),
            steering: rng.gen_range(-1.0..1.0),
        };

        update_vehicle_physics(&mut vehicle, &input, dt);

        // Check for NaN/Inf (indicates instability)
        assert!(vehicle.position.is_finite(), "Position became NaN/Inf");
        assert!(vehicle.velocity.is_finite(), "Velocity became NaN/Inf");
        assert!(vehicle.rotation.is_finite(), "Rotation became NaN/Inf");
    }
}
```

---

## 9. Network Prediction for Multiplayer

### 9.1 Overview

Vehicle physics in multiplayer requires special handling due to latency. Modern racing games use **client-side prediction** with **server reconciliation**.

### 9.2 Client-Side Prediction

**Concept:** Client simulates local vehicle immediately (zero perceived latency) while server validates.

```rust
struct ClientVehicle {
    // Local simulation state
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,

    // Prediction history
    input_history: VecDeque<InputFrame>,
    state_history: VecDeque<StateFrame>,

    // Server reconciliation
    last_server_state: Option<ServerState>,
    last_server_tick: u32,
}

struct InputFrame {
    tick: u32,
    input: Input,
}

struct StateFrame {
    tick: u32,
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
}

fn client_update(vehicle: &mut ClientVehicle, input: Input, dt: f32, current_tick: u32) {
    // 1. Store input for later reconciliation
    vehicle.input_history.push_back(InputFrame {
        tick: current_tick,
        input: input.clone(),
    });

    // 2. Simulate locally (instant feedback)
    update_vehicle_physics(vehicle, &input, dt);

    // 3. Store predicted state
    vehicle.state_history.push_back(StateFrame {
        tick: current_tick,
        position: vehicle.position,
        velocity: vehicle.velocity,
        rotation: vehicle.rotation,
    });

    // 4. Send input to server
    send_input_to_server(current_tick, input);

    // Keep history bounded (last 1 second)
    const MAX_HISTORY: usize = 60;
    while vehicle.input_history.len() > MAX_HISTORY {
        vehicle.input_history.pop_front();
        vehicle.state_history.pop_front();
    }
}
```

### 9.3 Server Reconciliation

**Concept:** When server state arrives, check if prediction was correct. If not, rewind and replay.

```rust
fn client_reconcile(vehicle: &mut ClientVehicle, server_state: ServerState) {
    // 1. Find the state we predicted for this server tick
    let predicted_state = vehicle.state_history.iter()
        .find(|s| s.tick == server_state.tick);

    let Some(predicted) = predicted_state else {
        // Too old, discard
        return;
    };

    // 2. Check prediction error
    let position_error = (predicted.position - server_state.position).length();
    let velocity_error = (predicted.velocity - server_state.velocity).length();

    const ERROR_THRESHOLD: f32 = 0.1; // 10cm

    if position_error < ERROR_THRESHOLD && velocity_error < ERROR_THRESHOLD {
        // Prediction was good, no correction needed
        return;
    }

    // 3. Prediction was wrong, rewind to server state
    vehicle.position = server_state.position;
    vehicle.velocity = server_state.velocity;
    vehicle.rotation = server_state.rotation;

    // 4. Replay all inputs since server tick
    for input_frame in &vehicle.input_history {
        if input_frame.tick > server_state.tick {
            update_vehicle_physics(vehicle, &input_frame.input, 1.0 / 60.0);
        }
    }

    // 5. Update state history with corrected prediction
    for state_frame in &mut vehicle.state_history {
        if state_frame.tick > server_state.tick {
            // Re-record corrected states
            state_frame.position = vehicle.position;
            state_frame.velocity = vehicle.velocity;
            state_frame.rotation = vehicle.rotation;
        }
    }
}
```

### 9.4 Remote Vehicle Interpolation

**Concept:** Remote vehicles (other players) are displayed with interpolation, not prediction.

```rust
struct RemoteVehicle {
    // Display state (interpolated)
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,

    // Network state buffer
    state_buffer: VecDeque<ServerState>,

    // Interpolation settings
    interpolation_delay: f32, // 100ms typical
}

fn update_remote_vehicle(vehicle: &mut RemoteVehicle, dt: f32, current_time: f32) {
    // 1. Calculate interpolation time (current time - delay)
    let interpolation_time = current_time - vehicle.interpolation_delay;

    // 2. Find two states to interpolate between
    let mut from_state = None;
    let mut to_state = None;

    for i in 0..vehicle.state_buffer.len() - 1 {
        if vehicle.state_buffer[i].timestamp <= interpolation_time &&
           vehicle.state_buffer[i + 1].timestamp >= interpolation_time {
            from_state = Some(&vehicle.state_buffer[i]);
            to_state = Some(&vehicle.state_buffer[i + 1]);
            break;
        }
    }

    // 3. Interpolate
    if let (Some(from), Some(to)) = (from_state, to_state) {
        let time_range = to.timestamp - from.timestamp;
        let t = (interpolation_time - from.timestamp) / time_range;

        vehicle.position = from.position.lerp(to.position, t);
        vehicle.velocity = from.velocity.lerp(to.velocity, t);
        vehicle.rotation = from.rotation.slerp(to.rotation, t);
    }

    // 4. Clean old states
    vehicle.state_buffer.retain(|state| {
        state.timestamp > current_time - 1.0 // Keep last second
    });
}
```

### 9.5 Extrapolation (Advanced)

For vehicles moving at high speed, interpolation can lag behind. **Extrapolation** predicts future position.

```rust
fn extrapolate_vehicle(vehicle: &RemoteVehicle, extrapolation_time: f32) -> Vec3 {
    // Simple linear extrapolation
    let extrapolated_position = vehicle.position + vehicle.velocity * extrapolation_time;

    // Problem: Doesn't account for turning!
    // Solution: Dead reckoning with physics
    let mut temp_vehicle = vehicle.clone();
    for _ in 0..(extrapolation_time / 0.016) as usize {
        // Simulate with last known input (guess)
        update_vehicle_physics(&mut temp_vehicle, &temp_vehicle.last_input, 0.016);
    }

    temp_vehicle.position
}
```

**Challenges with Extrapolation:**
- At high speeds, 100ms latency can mean 5-10 meters discrepancy
- Turning prediction is difficult (extrapolated vehicles "overshoot" corners)
- Works better for straight-line high-speed racing

**Best Practice:**
Use **interpolation** for most remote vehicles, **extrapolation** only for vehicles moving in straight lines at high speed.

### 9.6 Lag Compensation for Collisions

**Problem:** Client and server may disagree on collision timing.

**Solution:** Server authoritative with rewind.

```rust
fn server_check_collision(
    vehicle_a: &Vehicle,
    vehicle_b: &Vehicle,
    client_timestamp: f32,
    current_server_time: f32,
) -> bool {
    // 1. Calculate client latency
    let latency = current_server_time - client_timestamp;

    // 2. Rewind vehicle_b to client's perspective
    let vehicle_b_rewound = rewind_vehicle(vehicle_b, latency);

    // 3. Check collision at client's time
    check_collision(vehicle_a, &vehicle_b_rewound)
}

fn rewind_vehicle(vehicle: &Vehicle, time: f32) -> Vehicle {
    // Use velocity to rewind position
    let mut rewound = vehicle.clone();
    rewound.position -= rewound.velocity * time;
    rewound
}
```

### 9.7 Network Bandwidth Optimization

**Problem:** Sending full vehicle state (position, velocity, rotation, wheel states, etc.) at 60Hz is expensive.

**Solutions:**

1. **Lower Update Rate:**
   - Send player vehicle at 20-30 Hz
   - Interpolate between updates on receiving clients

2. **Delta Compression:**
   - Only send changed values
   - Use last acknowledged state as baseline

3. **Quantization:**
   ```rust
   fn quantize_position(pos: Vec3) -> (i16, i16, i16) {
       const SCALE: f32 = 100.0; // 1cm precision
       (
           (pos.x * SCALE) as i16,
           (pos.y * SCALE) as i16,
           (pos.z * SCALE) as i16,
       )
   }
   ```

4. **Interest Management:**
   - Only send updates for vehicles near player
   - Reduce update rate for distant vehicles

**Typical Bandwidth:**
- Full state: ~100 bytes
- Delta compressed: ~20-40 bytes
- Quantized: ~15-25 bytes
- At 20 Hz: ~400 bytes/sec per vehicle

---

## 10. Arcade vs. Simulation Spectrum

### 10.1 Spectrum Overview

Vehicle physics exists on a spectrum from pure arcade to pure simulation. Understanding this spectrum helps choose appropriate implementation complexity.

```
Arcade <----------------------------------------> Simulation
  1       2       3       4       5       6       7       8       9       10

Mario    GTA    NFS    Forza   Gran    iRacing  rFactor BeamNG
Kart            Heat   Horizon Turismo              2
```

### 10.2 Detailed Comparison

| Aspect | Arcade (1-3) | Simcade (4-6) | Simulation (7-10) |
|--------|-------------|---------------|-------------------|
| **Tire Model** | Linear friction | Simplified Pacejka | Full Pacejka + thermal |
| **Suspension** | None or basic spring | Spring-damper | Spring-damper + geometry |
| **Drivetrain** | Direct force | Torque curve + gearbox | Full torsional model |
| **Differential** | N/A | Open or locked | LSD with tuning |
| **Physics Rate** | 30-60 Hz | 60-120 Hz | 120-400 Hz |
| **Assists** | Always on, invisible | Optional, subtle | Minimal or none |
| **Tuning** | None | Basic (tires, gears) | Full (suspension, aero, diff) |
| **Learning Curve** | Minutes | Hours | Days to weeks |
| **Development Time** | Weeks | Months | 6+ months |
| **Code Complexity** | 3/10 | 6/10 | 8-10/10 |

### 10.3 Design Philosophy Differences

**Arcade Philosophy:**
> "What experience do I want to convey?"

- Focus on fun, accessibility
- Exaggerated physics for excitement
- Hidden assists make player feel skilled
- Prioritize responsiveness over realism

**Example:** Mario Kart uses:
- Instant acceleration (no engine simulation)
- Magnetic road adhesion (can't fall off edges easily)
- Auto-drift (steering automatically initiates drift)

**Implementation:**
```rust
fn arcade_steering(input: f32, speed: f32) -> f32 {
    // Simple, instant response
    let turn_rate = 2.0; // Much faster than realistic
    let speed_factor = (1.0 - speed / max_speed * 0.5); // Easier at high speed!
    input * turn_rate * speed_factor
}
```

**Simulation Philosophy:**
> "How does this work in reality?"

- Prioritize accuracy over fun
- Emergent gameplay from realistic systems
- Punish mistakes (spin if you brake while turning)
- Reward skill and knowledge

**Example:** iRacing requires:
- Smooth throttle/brake inputs (abrupt changes cause spin)
- Trail braking technique (real driving skill)
- Setup knowledge (understanding suspension geometry)

**Implementation:**
```rust
fn simulation_steering(input: f32, speed: f32, tire_grip: f32) -> f32 {
    // Realistic response with understeer
    let slip_angle = calculate_slip_angle(input, speed);
    let lateral_force = pacejka_lateral_force(slip_angle, tire_grip);
    let turn_rate = lateral_force / (vehicle_mass * speed);

    // Understeer at high speed if grip lost
    if slip_angle > MAX_GRIP_ANGLE {
        turn_rate * 0.5 // Reduced steering effectiveness
    } else {
        turn_rate
    }
}
```

### 10.4 Hybrid Approaches (Simcade)

**Best of Both Worlds:**
Realistic physics with hidden assists for accessibility.

**Forza Horizon Example:**
- Uses full physics simulation (Pacejka tires, suspension, drivetrain)
- BUT: Hidden stability control prevents most spins
- AND: Throttle/brake have smoothing filters
- PLUS: Steering has automatic counter-steer assist

**Implementation:**
```rust
fn simcade_update(vehicle: &mut Vehicle, raw_input: Input, dt: f32) {
    // 1. Apply smoothing to raw input (hidden assist)
    let smoothed_input = smooth_input(raw_input, dt);

    // 2. Run full physics simulation
    update_realistic_physics(vehicle, &smoothed_input, dt);

    // 3. Apply stability control (hidden assist)
    if detect_instability(vehicle) {
        apply_stability_control(vehicle);
    }

    // 4. Auto-counter-steer if spinning (hidden assist)
    if vehicle.angular_velocity.y.abs() > SPIN_THRESHOLD {
        apply_counter_steer(vehicle);
    }
}

fn smooth_input(raw: Input, dt: f32) -> Input {
    const SMOOTHING: f32 = 0.8;
    Input {
        throttle: lerp(LAST_INPUT.throttle, raw.throttle, SMOOTHING),
        brake: lerp(LAST_INPUT.brake, raw.brake, SMOOTHING),
        steering: lerp(LAST_INPUT.steering, raw.steering, SMOOTHING),
    }
}
```

### 10.5 Choosing Your Position on the Spectrum

**Questions to Ask:**

1. **Target Audience:**
   - Casual players? → Arcade (1-4)
   - Racing fans? → Simcade (5-6)
   - Racing enthusiasts? → Simulation (7-10)

2. **Platform:**
   - Mobile/Switch? → Arcade (1-3)
   - Console? → Simcade (4-6)
   - PC with wheels? → Simulation (7-10)

3. **Game Type:**
   - Open world? → Arcade-Simcade (3-5)
   - Track racing? → Simcade-Simulation (5-9)
   - Vehicle combat? → Arcade (2-4)

4. **Development Resources:**
   - Solo dev? → Arcade (1-4)
   - Small team? → Simcade (4-6)
   - Large team with expert? → Simulation (7-10)

**Recommendation for Agent Game Engine:**
Start at **Simcade (5-6)** with:
- Pacejka tire model (simplified)
- Spring-damper suspension + ARB
- Engine torque curve + gearbox
- Open differential
- 60-120 Hz physics
- Hidden assists configurable

This provides:
- Good enough for racing games
- Can scale down to arcade (disable features)
- Can scale up to simulation (add complexity)
- Reasonable development time (2-3 months)

---

## 11. Implementation Roadmap

### 11.1 Phase 1: Foundation (Week 1-2)

**Goal:** Basic drivable vehicle

**Components:**
- Rigid body integration (position, velocity, rotation)
- Simple tire model (linear friction)
- Raycast suspension
- Direct force application (no drivetrain)
- Basic input handling

**Deliverable:** Vehicle that can drive around, turn, and stop.

**Code Estimate:** ~1,000 lines

### 11.2 Phase 2: Tire Physics (Week 3-4)

**Goal:** Realistic tire behavior

**Components:**
- Pacejka tire model
- Slip angle calculation
- Slip ratio calculation
- Friction circle implementation
- Load sensitivity

**Deliverable:** Vehicle with realistic drifting and cornering limits.

**Code Estimate:** ~1,500 lines

### 11.3 Phase 3: Suspension (Week 5-6)

**Goal:** Proper suspension dynamics

**Components:**
- Spring-damper model
- Anti-roll bars
- Suspension geometry (camber, caster changes)
- Weight transfer simulation
- Bottoming out / topping out

**Deliverable:** Vehicle with realistic body roll, pitch, weight transfer.

**Code Estimate:** ~1,000 lines

### 11.4 Phase 4: Drivetrain (Week 7-9)

**Goal:** Realistic power delivery

**Components:**
- Engine torque curve
- Gearbox with ratios
- Clutch simulation (optional: auto-clutch)
- Differential (open, locked, LSD)
- Engine braking
- Stalling

**Deliverable:** Vehicle with realistic acceleration, gear shifts, power delivery.

**Code Estimate:** ~2,000 lines

### 11.5 Phase 5: Aerodynamics (Week 10)

**Goal:** High-speed stability

**Components:**
- Drag force
- Downforce
- Load map (downforce varies by speed, ride height)

**Deliverable:** Realistic top speed, high-speed stability.

**Code Estimate:** ~500 lines

### 11.6 Phase 6: Optimization (Week 11-12)

**Goal:** Performance targets met

**Components:**
- LOD system (physics detail levels)
- SIMD optimization
- Parallelization
- Profiling and benchmarking

**Deliverable:** 100+ vehicles at 60 FPS.

**Code Estimate:** ~1,000 lines

### 11.7 Phase 7: Networking (Week 13-15)

**Goal:** Multiplayer support

**Components:**
- Client-side prediction
- Server reconciliation
- Remote vehicle interpolation
- Lag compensation
- Bandwidth optimization

**Deliverable:** Smooth multiplayer with up to 32 players.

**Code Estimate:** ~2,000 lines

### 11.8 Phase 8: Tuning Tools (Week 16-18)

**Goal:** Easy tuning and validation

**Components:**
- Visual torque curve editor
- Suspension tuning UI
- Real-time telemetry display
- Test scenarios (acceleration, cornering, braking)
- Parameter export/import

**Deliverable:** Designer-friendly tuning tools.

**Code Estimate:** ~3,000 lines

**Total Time:** ~4.5 months
**Total Code:** ~12,000-15,000 lines for simcade implementation

---

## 12. Key Takeaways and Recommendations

### 12.1 Essential Implementations

**Must-Have for Any Racing Game:**
1. **Pacejka Tire Model** (or simplified version)
2. **Friction Circle** for combined slip
3. **Spring-Damper Suspension**
4. **Engine Torque Curve**
5. **Multi-rate physics update** (120+ Hz)

### 12.2 Optional but Recommended

**Simcade and Above:**
- Anti-roll bars
- Limited-slip differential
- Tire thermal model
- Suspension geometry changes

**Simulation Only:**
- Full torsional drivetrain
- Tire wear
- Chassis flex (simplified)
- Aerodynamic load maps

### 12.3 Performance Targets Summary

| Vehicle Count | Target Frame Time | Required Optimizations |
|---------------|-------------------|------------------------|
| 1-10          | < 1ms             | None (basic implementation) |
| 10-30         | < 4ms             | SIMD, basic LOD |
| 30-100        | < 8ms             | SIMD, LOD, parallelization |
| 100+          | < 12ms            | All optimizations + SoA layout |

### 12.4 Testing Priorities

**Critical Tests:**
1. Friction circle never exceeded
2. No NaN/Inf in physics (stability)
3. Energy conservation (vehicle doesn't gain speed without input)
4. Suspension equilibrium (stable at rest)

**Important Tests:**
5. Real-world acceleration/braking validation
6. Cornering G-force limits
7. Network prediction error bounds
8. Performance benchmarks (vehicles per frame)

### 12.5 Common Pitfalls to Avoid

1. **Adding Forces Instead of Combining:** Never add longitudinal and lateral tire forces directly. Use friction circle.

2. **Too-Stiff Torsional Springs:** In drivetrain simulation, excessively stiff springs cause numerical instability. Use implicit integration or limit stiffness.

3. **Forgetting Load Transfer:** Tire grip depends on normal force. Weight transfer in braking/acceleration is critical.

4. **Over-Engineering:** Don't implement soft-body physics unless it's your core feature. Rigid-body + good tire model is sufficient.

5. **Neglecting Network Early:** Retrofitting multiplayer is painful. Design physics with networking in mind from day one.

### 12.6 Recommended Resources

**Books:**
- "Race Car Vehicle Dynamics" by Milliken & Milliken (bible of vehicle dynamics)
- "Pacejka Tire Model" by Hans Pacejka (definitive tire model reference)

**Online:**
- [Vehicle Physics Pro Documentation](https://vehiclephysics.com/) - Excellent practical guidance
- [Car Physics for Games](https://www.asawicki.info/Mirror/Car%20Physics%20for%20Games/Car%20Physics%20for%20Games.html) - Classic article
- [Programming Vehicles in Games](https://wassimulator.com/blog/programming/programming_vehicles_in_games.html) - Modern overview

**Code Examples:**
- [GitHub: car-physics-pacejka](https://github.com/svenlr/car-physics-pacejka) - Pacejka implementation
- PhysX Vehicle SDK (for reference architecture)
- Unreal Chaos Vehicles (open source)

---

## 13. Conclusion

Modern vehicle physics for games has matured significantly by 2024-2025, with clear best practices:

- **Tire Physics:** Pacejka Magic Formula is the industry standard, balancing accuracy and performance
- **Suspension:** Spring-damper with anti-roll bars provides realistic handling at reasonable cost
- **Drivetrain:** Torsional coupling (PhysX-style) offers best realism, but direct torque application works for simcade
- **Networking:** Client-side prediction with server reconciliation is mandatory for multiplayer
- **Performance:** 120Hz+ physics update, SIMD optimization, LOD systems enable 100+ vehicles

The key to successful implementation is **choosing the right position on the arcade-simulation spectrum** based on your audience, platform, and resources. A well-executed simcade implementation (complexity 6/10) satisfies most racing games, with room to scale down (arcade) or up (simulation) as needed.

For the Agent Game Engine, I recommend:
1. Start with **Phase 1-4** (foundation through drivetrain) for a solid simcade base
2. Implement **Phase 6** (optimization) early to validate performance targets
3. Add **Phase 7** (networking) before public testing
4. Build **Phase 8** (tuning tools) to empower designers and reduce programmer bottlenecks

**Estimated Implementation:**
- **Complexity:** 6/10 (simcade)
- **Time:** 4-5 months (1-2 programmers)
- **Lines of Code:** ~12,000-15,000
- **Performance:** 100+ vehicles @ 60 FPS

---

## Sources

- [Vehicle Physics Pro - Tires](https://vehiclephysics.com/blocks/tires/)
- [Medium: How to program realistic vehicle physics](https://medium.com/@remvoorhuis/how-to-program-realistic-vehicle-physics-for-realtime-environments-games-part-i-simple-b4c2375dc7fa)
- [GitHub: car-physics-pacejka](https://github.com/svenlr/car-physics-pacejka)
- [BeamNG.drive - Wikipedia](https://en.wikipedia.org/wiki/BeamNG.drive)
- [Traxion: New Forza Motorsport gameplay](https://traxion.gg/new-forza-motorsport-gameplay-footage-showcases-impressive-driving-physics/)
- [Unreal Engine: Chaos Vehicles](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-vehicles)
- [Unreal Engine: How to Convert PhysX Vehicles to Chaos](https://dev.epicgames.com/documentation/en-us/unreal-engine/how-to-convert-physx-vehicles-to-chaos-in-unreal-engine)
- [Vehicle Physics Pro - Suspension](https://vehiclephysics.com/components/vehicle-suspension/)
- [Vehicle Physics Pro - Dynamics](https://vehiclephysics.com/components/vehicle-dynamics/)
- [NVIDIA PhysX Vehicles Documentation](https://nvidia-omniverse.github.io/PhysX/physx/5.1.0/docs/Vehicles.html)
- [VDS: Drivetrain Dynamics](https://bamason2.github.io/ttc066-module/notes/Section_7.html)
- [Vehicle Physics Pro - Engine](https://vehiclephysics.com/blocks/engine/)
- [x-engineer.org: Electric Vehicle Motor Torque and Power Curves](https://x-engineer.org/electric-vehicle-motor-torque-power-curves/)
- [GameDev.net: Limited-Slip Differential](https://gamedev.net/forums/topic/695333-limited-slip-differential-and-friends/5394027/)
- [NeurIPS 2024: Simulation Benchmark for Autonomous Racing](https://neurips.cc/virtual/2024/poster/97499)
- [Applied Intuition: VehicleSim](https://www.appliedintuition.com/products/vehiclesim)
- [Simracingcockpit: Which Sims Offer the Best Car Physics?](https://simracingcockpit.gg/which-sims-offer-the-best-car-physics/)
- [Nick Chavez: Client-side Prediction Multiplayer](https://nicolaschavez.com/projects/xrpg/)
- [Game Networking Demystified, Part V: Interpolation and Rollback](https://ruoyusun.com/2019/09/21/game-networking-5.html)
- [Superheroes in Racecars: Implementing Racing Games](https://superheroesinracecars.com/2016/08/11/implementing-racing-games-an-intro-to-different-approaches-and-their-game-design-trade-offs/)
- [Programming Vehicles in Games](https://wassimulator.com/blog/programming/programming_vehicles_in_games.html)
- [RecurDyn Help: Slip Ratio & Friction](https://help.functionbay.com/2025/RecurDynHelp/Tire/Tire_ch02_s02_00_index.html)
- [Car Physics for Games (Classic Article)](https://www.asawicki.info/Mirror/Car%20Physics%20for%20Games/Car%20Physics%20for%20Games.html)
