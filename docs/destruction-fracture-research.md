# Destruction and Fracture Simulation Research (2024-2026)

> **Research Date:** 2026-02-02
> **Target:** Multiplayer-ready destruction systems for silmaril
> **Focus:** Practical implementation with network synchronization

---

## Executive Summary

This document provides a comprehensive analysis of destruction and fracture simulation techniques for modern game engines, with emphasis on multiplayer-ready implementations. Research covers techniques from AAA games (Unreal's Chaos Destruction, The Finals, Teardown, Red Faction, Battlefield) and academic state-of-the-art approaches.

**Key Findings:**
- **Server-side destruction** is the new gold standard for multiplayer (The Finals, 2024-2025)
- **Pre-fractured assets** remain most performance-reliable for predictable frame rates
- **Voronoi fracture** is the industry-standard algorithm for runtime shattering
- **Convex decomposition** is critical for performance optimization (collision efficiency)
- **Object pooling** is mandatory for managing debris memory without fragmentation
- **Deterministic physics** enables lockstep networking with minimal bandwidth

---

## 1. Fracture Algorithms

### 1.1 Voronoi Shattering

**Description:**
Voronoi diagrams partition 3D space into convex cells based on seed points. When applied to mesh fracturing, each cell becomes a separate fragment.

**Implementation Approach:**
- Generate random seed points within or around mesh bounds
- Compute 3D Voronoi diagram
- Use Boolean operations to clip mesh geometry against Voronoi cells
- Generate convex fragments with proper collision geometry

**Performance Characteristics:**
- **Pre-computation:** Can fracture a cuboid into 32 completed rigid body meshed shards in <1/60th of a second ([Bullet Physics Forum](https://pybullet.org/Bullet/phpBB3/viewtopic.php?t=7707))
- **Real-time:** Highly experimental on large-scale assets; significant performance impact ([Godot VoronoiShatter](https://godotengine.org/asset-library/asset/3918))
- **Recommendation:** Pre-shatter meshes in editor/offline pipeline

**Complexity:** 6/10
- Algorithm itself is well-understood
- Robust Boolean operations on complex geometry are challenging
- Requires proper UV mapping and texture atlasing for visual quality

**Sources:**
- [Real-time fracturing in video games (Springer, 2022)](https://link.springer.com/article/10.1007/s11042-022-13049-x)
- [VoronoiShatter - Godot Asset Library](https://godotengine.org/asset-library/asset/3918)
- [Fracture Simulations in Game Dev](https://www.numberanalytics.com/blog/ultimate-guide-fracture-simulations-game-development)

---

### 1.2 Volumetric Approximate Convex Decomposition (VACD)

**Description:**
Advanced technique that decomposes meshes into approximate convex shapes, then applies fracture patterns dependent on impact location.

**Performance Characteristics:**
- Runs at >30 FPS including rigid body simulation, dust simulation, and rendering until original mesh splits into **20,000 separate pieces**
- Fracture time stays <50ms throughout simulation
- Handles unlimited number of pieces dynamically

**Advantages:**
- Convex shapes enable much faster collision detection than arbitrary concave fragments
- Impact-dependent fracture patterns provide realistic material response
- Scalable to large destruction scenarios

**Complexity:** 8/10
- Requires sophisticated convex decomposition algorithm (V-HACD or similar)
- Boolean operations for fracture pattern application
- Integration with physics engine for convex-convex collision optimization

**Sources:**
- [Real Time Dynamic Fracture with VACD (ACM TOG)](https://dl.acm.org/doi/10.1145/2461912.2461934)
- [VACD ResearchGate Paper](https://www.researchgate.net/publication/260398688_Real_Time_Dynamic_Fracture_with_Volumetric_Approximate_Convex_Decompositions)

---

### 1.3 Stress-Based Fracture Propagation

**Description:**
Physically-based approach using Finite Element Method (FEM) to compute stress tensors and propagate cracks from high-stress regions.

**Implementation Approach:**
- Tetrahedral mesh representation for FEM simulation
- Compute stress tensors for each tetrahedron
- Identify vertices exceeding tensile stress threshold
- Propagate cracks along stress direction
- Separate mesh at fracture boundaries

**Performance Characteristics:**
- Fully dynamic fracture produces more realistic, energetic results than quasi-static methods
- Material deforms pre-fracture, stores elastic energy, then releases as kinetic energy
- More computationally expensive than geometric methods (Voronoi)

**Advantages:**
- Physically accurate material behavior
- Natural crack propagation patterns
- Different materials can have different fracture thresholds
- Elastic energy conversion creates realistic post-fracture motion

**Complexity:** 9/10
- Requires FEM implementation or integration
- Stress tensor computation per-tetrahedron
- Crack topology management
- Mesh re-topologization after fracture

**Sources:**
- [Real-time deformation and fracture in a game environment (UC Berkeley)](http://graphics.berkeley.edu/papers/Parker-RTD-2009-08/)
- [Stress-based fracture at ACM SIGGRAPH/Eurographics](https://dl.acm.org/doi/10.1145/1599470.1599492)

---

### 1.4 Artist-Authored Pre-Fractured Assets

**Description:**
Artists manually create multiple versions of an asset in different destruction states. Swapped at runtime based on damage events.

**Implementation Approach:**
- Model intact asset
- Create damaged/destroyed variants with artist-controlled fracture patterns
- Set up swap conditions (health thresholds, hit locations, etc.)
- Use particle effects to mask transitions

**Performance Characteristics:**
- **Fastest runtime performance** (simple mesh swap)
- Fully predictable and controllable
- Known-in-advance results enable reliable optimization

**Advantages:**
- Complete artistic control over destruction aesthetics
- Guaranteed performance (no runtime computation)
- Deterministic (identical results every time)
- Easy to network (small state updates)

**Disadvantages:**
- High authoring cost (multiple asset versions)
- Limited variety (same destruction pattern repeats)
- Memory overhead (storing all variants)
- No dynamic interaction with environment

**Complexity:** 3/10
- Conceptually simple (asset swapping)
- Main challenge is authoring workflow efficiency
- Requires good artist tools

**Examples:**
- **Battlefield series:** Mesh swapping with particle effects to disguise transitions ([Number Analytics](https://www.numberanalytics.com/blog/ultimate-guide-fracture-simulations-game-development))
- **Red Faction: Guerrilla:** Combination of pre-fractured meshes + physics-based simulations ([ResearchGate](https://www.researchgate.net/publication/360585895_Real-time_fracturing_in_video_games))

---

### 1.5 Voxel-Based Destruction (Teardown Approach)

**Description:**
Represent world as voxel grid; destruction modifies voxel data directly. Rendering uses ray-marching through 3D textures.

**Implementation Approach:**
- Store world as 3D voxel textures (8-bit color palette per material)
- Render via ray-marching fragment shader (12 triangles per object, complexity in fragment shader)
- Use octree/mipmap acceleration for ray traversal
- Modify voxel data for destruction

**Performance Characteristics:**
- **Rendering:** Single draw call per object (12 triangles), complexity in fragment shader
- **Storage:** 1 byte per voxel (using 8-bit palette saves ~75% memory vs. full RGB)
- **Updates:** Fast scene updates for voxel modifications
- **Limitations:** Structural integrity for all voxels would make game unrunnable; requires radius-based async checking

**Advantages:**
- Extreme flexibility in destruction patterns
- No pre-computation required
- Unified representation (rendering + physics)
- Naturally supports arbitrary shapes

**Disadvantages:**
- Memory-intensive for large worlds
- Voxel resolution vs. visual quality trade-off
- Physics complexity (load-bearing calculations)

**Complexity:** 9/10
- Custom rendering pipeline (ray-marching shaders)
- Voxel DDA with octree acceleration
- Physics integration for voxel structures
- Structural integrity approximations

**2024 Update:**
Developer teased new voxel format with:
- Large object support
- Unique properties per voxel (no palettes)
- Very fast scene updates
- Higher memory per voxel, but fewer total voxels

**Sources:**
- [Teardown Breakdown Analysis](https://juandiegomontoya.github.io/teardown_breakdown.html)
- [Teardown Render Techniques](https://zacxalot.github.io/rendering/9-teardown/)
- [Teardown Developer Teased New Engine](https://80.lv/articles/teardown-creator-is-working-on-a-new-voxel-engine)

---

## 2. AAA Engine Implementations

### 2.1 Chaos Destruction (Unreal Engine 5)

**Overview:**
Unreal's built-in high-performance physics and destruction system, evolved from Fortnite and UEFN development.

**Key Features (2024-2025):**
- **Cache System:** Smooth replay of complex destruction at runtime with minimal performance impact
- **Nanite Integration:** Dramatically improved rendering performance for destruction debris
- **One-Way Interaction:** Simplified rigid body interaction for better performance and control
- **Trace & External Strain:** Lighter-weight alternatives to expensive field-based destruction

**Performance Benchmarks:**
- **Large-scale test:** 105 statues destruction maintains 30+ FPS (40-45 FPS average) with all optimizations ([UE Optimization Docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-optimization))
- **UE 5.4+ Improvements:** Significant performance gains from Fortnite/MetaHuman development

**Network Synchronization:**
- Chaos supports networked destruction with client prediction
- Cache system enables efficient replay/replication
- Server-authoritative with client-side prediction for responsiveness

**Complexity:** 7/10 (for integration)
- Engine handles heavy lifting
- Artist workflow complexity in Geometry Collections
- Optimization requires understanding of Cache System, Nanite, Root Mesh Proxies

**Sources:**
- [Chaos Destruction UE 5.7 Docs](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-in-unreal-engine)
- [Chaos Destruction Optimization](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-optimization)
- [Chaos Physics for Large-Scale Scenes (Unreal Fest 2024)](https://forums.unrealengine.com/t/talks-and-demos-using-chaos-physics-for-large-scale-and-high-fidelity-scenes-unreal-fest-2024/2025096)

---

### 2.2 The Finals - Server-Side Destruction (2024-2025)

**Overview:**
Embark Studios' revolutionary **server-side destruction** system. This is the new gold standard for multiplayer destruction.

**Technical Innovation:**
- **Server-side movement, physics, AND destruction** (holy grail achievement)
- Single source of truth for all players simultaneously
- Lower latency than traditional client-side physics with server reconciliation

**How It Works:**
- All environment physics runs on game servers (not client machines)
- Destruction events computed server-side
- Server streams synchronized destruction state to all clients
- Every player experiences same impact at same time

**Advantages:**
- Perfect synchronization across all players
- Eliminates client-side prediction divergence
- Anti-cheat friendly (physics cannot be manipulated client-side)
- No need for deterministic physics (server is authoritative)

**Performance:**
- Maintains real-time performance in fast-paced FPS environment
- Handles massive-scale environmental destruction (entire building collapses)
- Season 8 (2025): Introduced "Smooth Destruction" improvements

**Development Team:**
- Many developers from DICE's Battlefield team
- Leveraged Battlefield destruction experience to solve multiplayer sync challenges

**Complexity:** 10/10 (to implement from scratch)
- Requires massive server infrastructure
- Low-latency network protocol for physics streaming
- Server-side physics simulation at scale
- Client-side interpolation/extrapolation for responsiveness

**Implications for silmaril:**
- Server-authoritative architecture aligns with our goals
- Requires robust networking layer
- Consider hybrid: critical destruction server-side, cosmetic debris client-side

**Sources:**
- [The Finals Server-Side Destruction Analysis](https://www.digitaltrends.com/gaming/the-finals-preview-destruction-will-make-developers-panic/)
- [Embark Studios The Finals Tech](https://venturebeat.com/games/embark-studios-unveils-the-finals-team-based-shooter-game/)
- [The Finals Season 8 Smooth Destruction](https://biggo.com/news/202509120223_The_Finals_Season_8_Smooth_Destruction_Update)
- [Destruction White Whale Article](https://www.theringer.com/2024/02/02/video-games/destruction-video-games-battlefield-bad-company-red-faction-battlebit-teardown-the-finals)

---

### 2.3 Red Faction Series (GeoMod / GeoMod 2.0)

**Original Red Faction (2001) - GeoMod:**
- CSG-based real-time destruction
- Allowed real-time demolition via explosive weaponry
- **Multiplayer challenge (2001):** Synchronizing destruction was technically difficult, but resulted in unique unpredictable gameplay

**Red Faction: Guerrilla (2009) - GeoMod 2.0:**
- Physics-based destruction system
- Combination of pre-fractured meshes + physics simulation
- Multiplayer featured 8 game modes, 19+ weapons, 10 backpacks, 41 maps
- All multiplayer modes use GeoMod 2.0 with same weapons as singleplayer

**Network Evolution:**
- Original (2001): Bandwidth limitations made synchronization challenging
- Modern era: Better bandwidth makes physics-based multiplayer destruction more manageable

**Complexity:** 7/10 (for GeoMod 2.0 approach)
- Pre-fracture + physics hybrid reduces runtime complexity
- Still requires robust physics networking

**Sources:**
- [Red Faction Wiki - Multiplayer](https://redfaction.fandom.com/wiki/Multiplayer)
- [Red Faction Wikipedia](https://en.wikipedia.org/wiki/Red_Faction:_Guerrilla)
- [GDC: Multiplayer Level Design in Red Faction Guerrilla](https://gdcvault.com/play/1012881/Multiplayer-Level-Design-in-Red)

---

### 2.4 Havok Destruction vs PhysX Destruction

**PhysX Destruction:**
- Developed by NVIDIA (now deprecated in favor of Unreal's Chaos)
- Epic began replacing PhysX with Chaos around 2019 for Unreal Engine 5
- PhysX still available as open-source library for custom engines

**Havok Destruction:**
- Released 2008, officially discontinued as standalone product
- **2024 Update:** Havok posted new "Dynamic Destruction" video after years of silence
- Still powers custom-engine AAA games:
  - Starfield
  - Call of Duty
  - No Man's Sky
  - Spider-Man 2
  - Zelda: Tears of the Kingdom
  - Astro Bot
- Can now be integrated into Unreal Engine 5 projects

**Current State (2024-2026):**
- PhysX: Largely replaced by Chaos in UE; still used in legacy Unity projects
- Havok: Still actively developed and used in major AAA titles
- Chaos: Primary destruction system for Unreal Engine ecosystem

**Complexity:** N/A (middleware integration)
- Havok: Commercial licensing, integration complexity ~6/10
- PhysX: Open-source, integration complexity ~7/10
- Chaos: UE-integrated, complexity ~7/10 for advanced usage

**Sources:**
- [Havok vs PhysX Comparison 2024](https://www.saaskart.co/compare/physx-vs-havok)
- [Havok Dynamic Destruction 2024](https://www.neogaf.com/threads/havok-check-in-dynamic-destruction-with-havok-physics-example.1679823/)
- [Havok Wikipedia](https://en.wikipedia.org/wiki/Havok_(software))

---

## 3. Runtime vs Pre-Computed Approaches

### 3.1 Performance Comparison

| Approach | Frame Time Impact | Memory | Variety | Determinism | Network Bandwidth |
|----------|------------------|---------|---------|-------------|-------------------|
| **Pre-fractured Assets** | Minimal (<1ms swap) | High (all variants) | Low (repeating) | Perfect | Low (state IDs) |
| **Runtime Voronoi (pre-shattered)** | Low (<5ms trigger) | Medium | High | Good (seeded RNG) | Medium (seed + trigger) |
| **Runtime Voronoi (real-time)** | High (10-50ms) | Low | Highest | Poor (timing-dependent) | High (all fragments) |
| **Stress-Based FEM** | Very High (>50ms) | Medium | Highest | Poor | Very High |
| **VACD (pre-computed)** | Medium (1-5ms) | Medium-High | High | Good | Medium |
| **Voxel (Teardown)** | Medium (shader-bound) | Very High | Extreme | Perfect (voxel grid) | Very High |

### 3.2 Recommendations by Use Case

**Multiplayer FPS (Fast-paced):**
- **Recommended:** Pre-fractured + server-side triggering (The Finals approach)
- **Alternative:** Pre-shattered Voronoi with deterministic seeds
- **Avoid:** Real-time fracture computation (too slow for 60Hz server tick)

**Singleplayer Destruction Showcase:**
- **Recommended:** VACD or stress-based FEM for maximum realism
- **Alternative:** Real-time Voronoi for good balance
- **Best Example:** Teardown (voxel-based for ultimate flexibility)

**MMO / Large-Scale Multiplayer:**
- **Recommended:** Pre-fractured assets with LOD
- **Critical:** Aggressive debris cleanup and culling
- **Network:** State synchronization with importance-based prioritization

**Physics Puzzle Game:**
- **Recommended:** Pre-shattered Voronoi with deterministic physics
- **Alternative:** Artist-authored for guaranteed solvability
- **Network:** Lockstep deterministic simulation (inputs only)

### 3.3 Hybrid Approach (Recommended for silmaril)

**Combine strengths of multiple techniques:**

1. **Pre-compute fracture patterns offline** (Voronoi or VACD)
   - Store as asset variants or embedded fracture metadata
   - Artist can review and adjust if needed

2. **Trigger destruction at runtime** based on gameplay events
   - Server-authoritative trigger decision
   - Deterministic seed based on impact parameters

3. **Client-side cosmetic enhancement**
   - Server sends: object ID, fracture seed, impact point
   - Clients generate identical fragments (deterministic)
   - Clients add local-only cosmetic debris/particles

4. **Aggressive debris management**
   - Object pooling for fragment rigid bodies
   - LOD: distant debris uses simplified collision/rendering
   - Cleanup: despawn debris after timeout or when off-screen

**Complexity:** 7/10
**Performance:** Target <5ms for fracture trigger, <1ms for pre-fractured swap

---

## 4. Performance Optimization Techniques

### 4.1 Object Pooling for Debris

**Problem:**
- Constant `new`/`delete` (C++) or `SpawnActor`/`Destroy` (UE) causes:
  - Processing overhead (allocation/deallocation)
  - Frame rate spikes
  - Memory fragmentation
  - In extreme cases: millions of allocations per frame for 1-4 byte blocks

**Solution: Object Pooling**
- Pre-allocate fixed pool of debris objects at startup
- Reuse objects instead of allocating/freeing
- Return to pool when debris expires

**Benefits:**
- Eliminates allocation/deallocation overhead
- Stable memory footprint (no fragmentation)
- Faster access to pre-initialized objects
- Dramatically reduces GC pressure (in managed languages)

**Implementation Guidelines:**
- Pool size: 1000-5000 debris objects (tune based on max concurrent destruction)
- Reset object state on acquire from pool
- Use ring buffer or free-list for fast acquire/release
- Monitor pool exhaustion (expand dynamically if needed, or fail gracefully)

**When Pooling is Critical:**
- Shooters (bullet impacts, explosions)
- Hack-and-slash (repeated melee hits)
- Destruction-heavy games (building collapses)
- Particle-intensive effects

**Sources:**
- [Object Pooling in Games (DEV Community)](https://dev.to/patrocinioluisf/maximizing-memory-management-object-pooling-in-games-6bg)
- [Game Programming Patterns - Object Pool](https://gameprogrammingpatterns.com/object-pool.html)
- [Unreal Engine Object Pooling](https://outscal.com/blog/unreal-engine-object-pooling)

---

### 4.2 Level of Detail (LOD) for Destruction

**Rendering LOD:**
- Close range: Full-resolution fragments with PBR materials
- Mid range: Simplified geometry (reduced poly count)
- Far range: Billboards or instanced impostors
- Very far: Culled entirely (outside interest radius)

**Physics LOD:**
- Close range: Accurate convex collision meshes per fragment
- Mid range: Single compound collision shape for debris cluster
- Far range: Simplified bounding box or sphere
- Very far: Collision disabled (sleep/cull)

**Network LOD (Multiplayer):**
- Critical players (nearby, high priority): Full fragment synchronization
- Secondary players: Clustered/simplified debris representation
- Background players: Major destruction events only, no debris sync
- Out of interest: No synchronization (culled)

**Transition Strategies:**
- Smooth LOD transitions to avoid popping
- Hysteresis (different thresholds for upgrade vs. downgrade)
- Time-based: debris simplifies over time (fresh = detailed, old = simple)

**Sources:**
- [Destruction Debris LOD & Culling](https://www.linkedin.com/advice/0/what-best-ways-create-satisfying-destruction-physics-dz84f)
- [Chaos Destruction Optimization (UE)](https://dev.epicgames.com/documentation/en-us/unreal-engine/chaos-destruction-optimization)

---

### 4.3 Debris Cleanup Strategies

**Time-Based Cleanup:**
- Set maximum lifetime for debris (e.g., 10-30 seconds)
- Fade out visually before despawning (smooth transition)
- Prioritize cleanup of oldest debris first

**Performance-Based Cleanup:**
- Monitor frame rate
- If FPS drops below threshold, aggressively remove debris
- Remove smallest/furthest objects first
- Example: Dynamic Debris Removal mod (Steam Workshop) - removes debris only when FPS drops

**Spatial Cleanup:**
- Despawn debris outside player interest radius
- Remove debris in areas with no players nearby
- Keep debris near important locations (objectives, spawn points)

**Count-Based Cleanup:**
- Maximum concurrent debris limit (e.g., 500 fragments)
- When limit exceeded, remove oldest/least-important debris
- Use priority queue (importance = size × proximity × age)

**Network Optimization:**
- Server controls debris cleanup (authoritative)
- Clients can cleanup purely cosmetic local debris freely
- Sync important/persistent debris only

**Sources:**
- [Dynamic Debris Removal (Steam)](https://steamcommunity.com/sharedfiles/filedetails/?id=2677340659)
- [Unreal Multiplayer Destruction](https://forums.unrealengine.com/t/unreal-engine-4-and-multiplayer-destruction/119192)

---

### 4.4 Convex Decomposition for Collision Optimization

**Why Convex Matters:**
- Collision algorithms run **much faster** on convex shapes vs. arbitrary concave meshes
- GJK/EPA (common collision algorithms) optimized for convex-convex tests
- Broad-phase culling more effective with tight convex bounds

**Techniques:**
- **V-HACD (Voxel-based Hierarchical Approximate Convex Decomposition):**
  - Industry standard algorithm
  - Decomposes arbitrary mesh into approximate convex pieces
  - Tunable parameters: max convexity, max hulls, resolution

- **VACD (Volumetric Approximate Convex Decomposition):**
  - Volumetric approach (research technique)
  - Optimized for dynamic fracture scenarios

**Performance Impact:**
- Well-decomposed fragments: 5-10× faster collision detection
- Poor decomposition: Negligible improvement or worse (too many hulls)

**Best Practices:**
- Pre-compute convex decomposition offline (part of asset pipeline)
- Balance: fewer hulls (faster) vs. accuracy (more hulls)
- For destruction: accept some inaccuracy for speed (player won't notice)
- Store convex hulls with fractured assets

**Sources:**
- [VACD Research Paper](https://dl.acm.org/doi/10.1145/2461912.2461934)
- [Real-time fracturing research](https://link.springer.com/article/10.1007/s11042-022-13049-x)

---

## 5. Network Synchronization Approaches

### 5.1 Deterministic Physics (Lockstep)

**Concept:**
- All clients run identical local simulation
- Only exchange player inputs (not full state)
- Given same inputs, deterministic physics produces identical results

**Advantages:**
- **Extremely low bandwidth** (inputs only, ~100 bytes/sec per player)
- No state synchronization needed (all clients compute same state)
- Scales to massive entity counts without network cost

**Challenges:**
- **Floating-point determinism:** Requires identical CPU instruction sets, compiler settings
- **Physics engine determinism:** Most engines (PhysX, Havok) are NOT deterministic by default
- **Chain reactions:** Hard to synchronize and fix divergence
- **Debugging:** Difficult to reproduce issues (timing-sensitive)

**Making Physics Deterministic:**
- Fixed timestep (never variable dt)
- Deterministic math library (avoid platform-specific intrinsics)
- Deterministic RNG (seeded, reproducible)
- Same compiler, optimization flags, CPU features across all platforms
- Avoid floating-point non-associativity issues

**When to Use:**
- Turn-based strategy games
- RTS games (StarCraft model)
- Physics puzzle games (reproducibility critical)
- Games with limited player count (<16) and stable connections

**Examples:**
- **Rainbow Six Siege:** Uses seeded RNG for destruction; sends seed as input to network, all clients apply destruction locally using same seed

**Sources:**
- [Deterministic Simulation for Lockstep (DaydreamSoft)](https://www.daydreamsoft.com/blog/deterministic-simulation-for-lockstep-multiplayer-engines)
- [Floating Point Determinism (Gaffer On Games)](https://gafferongames.com/post/floating_point_determinism/)
- [Game Networking Demystified - Deterministic](https://ruoyusun.com/2019/03/29/game-networking-2.html)
- [Destruction in Video Games - Hypesio](https://hypesio.fr/en/destruction-in-video-games/)

---

### 5.2 State Synchronization (Server-Authoritative)

**Concept:**
- Server runs authoritative simulation
- Server sends full or delta state updates to clients
- Clients render/interpolate received state

**For Destruction:**
- Server triggers fracture events
- Server simulates debris physics
- Server sends fragment positions/velocities to clients
- Clients interpolate between updates

**Advantages:**
- No determinism requirements
- Server can use different/faster physics than clients
- Anti-cheat friendly (clients can't manipulate physics)
- Easier debugging (server is source of truth)

**Challenges:**
- **High bandwidth** for many fragments
- Requires efficient delta compression
- Interest management critical (don't sync all debris to all players)
- Latency compensation for responsiveness

**Optimization Techniques:**
- **Prioritization:** Send important debris first (close to player, large fragments)
- **Clustering:** Group nearby small debris into single network update
- **Culling:** Don't sync debris outside client's interest area
- **Delta compression:** Send velocity only, clients predict position
- **Sleeping physics:** Stop syncing debris that has stopped moving

**When to Use:**
- Fast-paced multiplayer (FPS, battle royale)
- Massive player counts (MMO)
- Cross-platform games (determinism hard to achieve)
- When anti-cheat is critical

**Examples:**
- **The Finals:** Server-side physics and destruction (state sync)
- **Battlefield:** Mesh swapping with state IDs (low bandwidth)

**Sources:**
- [Networked Physics Challenges Q&A](https://daily.dev/blog/networked-physics-challenges-qanda)
- [Choosing the Right Network Model](https://mas-bandwidth.com/choosing-the-right-network-model-for-your-multiplayer-game/)
- [The Finals Server-Side Destruction](https://www.digitaltrends.com/gaming/the-finals-preview-destruction-will-make-developers-panic/)

---

### 5.3 Hybrid Approach (Recommended for silmaril)

**Architecture:**

1. **Server-authoritative triggers:**
   - Server decides when/where destruction occurs
   - Server computes fracture seed based on impact parameters
   - Server sends: `{object_id, fracture_seed, impact_point, impact_force}`

2. **Client-side deterministic fracture:**
   - Clients use same fracture seed to generate identical fragments
   - Clients run local physics simulation for fragments
   - **Key insight:** Debris physics don't need to be perfectly synced (cosmetic variation acceptable)

3. **Periodic correction (optional):**
   - Server sends occasional fragment position corrections for "important" debris
   - Clients smoothly interpolate to corrected positions
   - Prevents major divergence while keeping bandwidth low

4. **Cleanup synchronization:**
   - Server authoritatively despawns debris after timeout
   - Sends despawn events to clients
   - Clients can despawn additional local-only cosmetic debris freely

**Advantages:**
- Low bandwidth (seed + periodic corrections)
- Visually consistent destruction (same fragments)
- Tolerates physics divergence (debris is cosmetic)
- Supports client-side prediction/enhancement

**Bandwidth Estimate:**
- Destruction trigger: ~32 bytes (object ID + seed + impact data)
- Per-fragment correction (if needed): ~12 bytes (entity ID + compressed pos)
- Total: 32 bytes/destruction + 12 bytes/fragment/correction (every 1-5 sec)

**Sources:**
- Synthesized from multiple approaches
- Inspired by Rainbow Six Siege's seeded RNG approach
- Informed by The Finals' server-authoritative architecture

---

## 6. Performance Targets

### 6.1 Runtime Performance

| Metric | Target | Critical Threshold | Notes |
|--------|--------|-------------------|-------|
| **Fracture Trigger Time** | <5ms | <16ms | Time to spawn debris on destruction event |
| **Fragments per Frame** | 50-200 | 500 | New fragments spawned in single frame |
| **Active Fragments (Physics)** | 500-2000 | 5000 | Concurrent fragments with active physics |
| **Active Fragments (Rendered)** | 2000-10000 | 20000 | Concurrent visible fragments |
| **Collision Accuracy** | 95%+ convex hull match | 80% | Fragment collision vs. visual mesh accuracy |
| **Debris Lifetime** | 10-30 sec | 60 sec | Time before automatic cleanup |
| **Frame Time Impact (avg)** | <2ms | <5ms | Physics + rendering overhead for all debris |
| **Frame Time Impact (spike)** | <10ms | <33ms | Worst-case spike during large destruction |

### 6.2 Memory Targets

| Resource | Target | Critical Threshold | Notes |
|----------|--------|-------------------|-------|
| **Fragment Pool Size** | 2000 objects | 5000 objects | Pre-allocated debris pool |
| **Memory per Fragment** | <1 KB | <2 KB | Average memory per debris object |
| **Total Debris Memory** | <50 MB | <200 MB | Maximum memory for all destruction systems |
| **Fractured Asset Overhead** | 2-5× base mesh | 10× base mesh | Pre-fractured variants vs. intact mesh |

### 6.3 Network Targets (Multiplayer)

| Metric | Target | Critical Threshold | Notes |
|--------|--------|-------------------|-------|
| **Destruction Event Size** | 32-64 bytes | 256 bytes | Network message for single destruction trigger |
| **Fragments Synced per Event** | 0-10 | 50 | Important fragments requiring full sync |
| **Bandwidth per Player (destruction)** | <5 KB/sec | <20 KB/sec | Bandwidth dedicated to destruction sync |
| **Latency to Visual Feedback** | <100ms | <300ms | Time from server trigger to client visual response |
| **Debris Sync Rate** | 1-5 Hz | 10 Hz | Update rate for fragment positions (if synced) |

### 6.4 Benchmarking Methodology

**Setup:**
1. Create test scene with destructible objects of varying complexity
2. Configure destruction density (fragments per object: low=10-20, medium=50-100, high=200-500)
3. Instrument with profiling markers (Phase 0 profiling infrastructure)

**Test Scenarios:**

**A. Single Destruction Event:**
- Trigger destruction of one object
- Measure: fracture time, fragment spawn time, peak frame time
- Repeat 100× and compute percentiles (p50, p95, p99)

**B. Sustained Destruction:**
- Trigger destruction events every 0.5 seconds for 30 seconds
- Measure: average frame time, fragment count over time, memory usage
- Track cleanup behavior

**C. Massive Destruction (Stress Test):**
- Trigger simultaneous destruction of 10-50 objects
- Measure: worst-case frame time spike, memory peak, recovery time
- Ensure game remains playable (>20 FPS)

**D. Network Stress Test (Multiplayer):**
- Simulate 32 clients
- Trigger destruction events at 10 Hz
- Measure: bandwidth per client, server CPU usage, client latency to visual response

**Metrics to Track:**
- Frame time (CPU, GPU separately)
- Physics step time
- Active fragment count (physics, rendering)
- Memory usage (total, per-fragment)
- Network bandwidth (up/down)
- Collision accuracy (% correct vs. visual mesh)

**Acceptance Criteria:**
- All "Target" metrics met in normal scenarios
- All "Critical Threshold" metrics met in stress scenarios
- No crashes or memory leaks over 1-hour sustained test

---

## 7. Testing Requirements

### 7.1 Unit Tests

**Fracture Algorithm Tests:**
- Test Voronoi cell generation with various seed counts (5, 10, 50, 100)
- Test convex hull generation from fragments
- Test edge cases: degenerate geometry, very small meshes, very large meshes
- Test determinism: same seed produces identical fragments across runs

**Object Pool Tests:**
- Test acquire/release from pool
- Test pool exhaustion (behavior when empty)
- Test pool expansion (if dynamic)
- Test reset on acquire (fragments have clean state)

**Memory Management Tests:**
- Test for memory leaks (allocate/deallocate 10,000 fragments)
- Test fragmentation (repeated destruction over time)
- Test pool memory footprint (matches expectations)

---

### 7.2 Integration Tests

**Physics Integration:**
- Test fragment collision detection (against each other, against world)
- Test fragment sleeping (debris stops moving after settling)
- Test convex decomposition integration (correct collision shapes)

**Rendering Integration:**
- Test fragment LOD transitions (no popping or missing meshes)
- Test fragment culling (off-screen fragments not rendered)
- Test material/texture preservation (fragments match original appearance)

**Network Integration (Multiplayer):**
- Test destruction event serialization/deserialization
- Test deterministic fragment generation from seed (client matches server)
- Test fragment synchronization (positions within tolerance)
- Test cleanup synchronization (clients remove debris when server despawns)

---

### 7.3 Stress Tests

**Fragment Count Stress:**
- Spawn 5,000 fragments simultaneously
- Ensure frame rate stays above critical threshold (>20 FPS)
- Ensure memory stays within budget (<200 MB)

**Sustained Destruction Stress:**
- Run destruction events at 10 Hz for 10 minutes
- Monitor for memory leaks (memory should stabilize)
- Monitor for performance degradation (frame time should stabilize)

**Network Stress (Multiplayer):**
- 64 simultaneous clients
- 50 destruction events per second across all clients
- Ensure server stays below 16ms tick time
- Ensure client bandwidth stays below critical threshold (<20 KB/sec per client)

---

### 7.4 Visual Quality Tests

**Artistic Review:**
- Artist review of fracture patterns (do they look believable?)
- Test with various materials (wood, concrete, metal, glass)
- Test with various mesh types (organic, hard-surface, low-poly, high-poly)

**Collision Accuracy Test:**
- Visual comparison: fragment collision mesh vs. render mesh
- Test penetration: fragments should not visibly intersect world geometry
- Test stability: debris piles should settle without jitter

**LOD Quality Test:**
- Verify LOD transitions are smooth (no popping)
- Verify distant debris still looks reasonable
- Verify culling works correctly (no missing debris in view)

---

## 8. Implementation Recommendations for silmaril

### 8.1 Proposed Architecture

**Phase 1: Pre-Fractured System (MVP)**
- Artist-authored or offline Voronoi pre-fracturing
- Runtime trigger system (swap intact mesh with fractured variants)
- Basic object pooling for debris
- Server-authoritative triggers
- Simple cleanup (time-based)

**Complexity:** 5/10
**Timeline:** 2-3 weeks
**Performance:** Excellent (pre-computed)

---

**Phase 2: Runtime Voronoi with Deterministic Seeds**
- Offline pre-computation of Voronoi seeds
- Runtime fragment generation from seeds (deterministic)
- Convex decomposition for collision optimization
- Network sync: server sends seed, clients generate fragments
- Advanced cleanup (performance-based + time-based)

**Complexity:** 7/10
**Timeline:** 4-6 weeks (depends on Voronoi library choice)
**Performance:** Good (5-10ms fracture trigger)

---

**Phase 3: Advanced Features (Post-MVP)**
- Stress-based fracture (optional, for hero destruction moments)
- Dynamic LOD for debris
- Clustering for network optimization
- Chaos Destruction integration (if using Unreal) or Havok (for custom engine)

**Complexity:** 9/10
**Timeline:** 8-12 weeks
**Performance:** Target-dependent (tune per use case)

---

### 8.2 Technology Recommendations

**Voronoi Library:**
- **voro++** (C++): Mature, well-tested, BSD license
- **Bullet Physics Voronoi** (built-in): Simple integration if using Bullet
- **Custom implementation:** Only if you need extreme control (not recommended)

**Physics Engine:**
- **Rapier** (Rust-native): Already chosen for silmaril, supports convex shapes well
- **Ensure deterministic mode** (if using lockstep networking): Fixed timestep, reproducible RNG

**Convex Decomposition:**
- **V-HACD** (open-source): Industry standard, good quality/performance balance
- Integrate into offline asset pipeline (not runtime)

**Networking:**
- Server-authoritative triggers
- Hybrid deterministic fragments (seeded) with optional corrections
- Use existing silmaril networking crate (TCP for triggers, UDP for corrections if needed)

**Memory Management:**
- Use silmaril's **arena** or **pool** allocators for debris (already implemented)
- Pre-allocate pool at startup: 2,000 fragments × 1 KB = ~2 MB
- Monitor pool usage, expand if needed (or fail gracefully)

---

### 8.3 Integration with Existing Systems

**ECS Integration:**
```rust
// Example component structure

#[derive(Component)]
pub struct Destructible {
    pub fracture_pattern: FracturePattern,  // Pre-computed or runtime config
    pub health: f32,
    pub fracture_threshold: f32,
}

#[derive(Component)]
pub struct DebrisFragment {
    pub parent_entity: Entity,  // Original destructible object
    pub fragment_index: u32,
    pub lifetime_remaining: f32,
    pub importance: f32,  // For prioritization (size, proximity)
}

pub enum FracturePattern {
    PreFractured { asset_id: AssetId },  // Swap to pre-fractured mesh
    Voronoi { seed: u64, cell_count: u32 },  // Runtime generation
    Custom { fragments: Vec<FragmentData> },  // Artist-authored
}
```

**Physics Integration (Rapier):**
```rust
// Fragment spawning system
pub fn spawn_debris_fragments(
    fracture_events: EventReader<FractureEvent>,
    pool: Res<DebrisPool>,
    physics: ResMut<RapierContext>,
    query: Query<(&Transform, &Destructible)>,
) {
    for event in fracture_events.iter() {
        let (transform, destructible) = query.get(event.entity).unwrap();

        // Generate or retrieve fragments
        let fragments = match &destructible.fracture_pattern {
            FracturePattern::PreFractured { asset_id } => load_fragments(asset_id),
            FracturePattern::Voronoi { seed, cell_count } => {
                generate_voronoi_fragments(*seed, *cell_count, transform)
            },
            FracturePattern::Custom { fragments } => fragments.clone(),
        };

        // Spawn fragments from pool
        for fragment_data in fragments {
            let debris = pool.acquire();
            spawn_debris_rigid_body(&mut physics, debris, fragment_data);
        }

        // Despawn original entity
        commands.entity(event.entity).despawn();
    }
}
```

**Networking Integration:**
```rust
// Server sends fracture event
#[derive(Serialize, Deserialize)]
pub struct FractureEventNet {
    pub entity_id: u64,
    pub fracture_seed: u64,  // Deterministic generation
    pub impact_point: Vec3,
    pub impact_force: Vec3,
}

// Client receives and replicates fracture
pub fn handle_fracture_event_client(
    events: EventReader<FractureEventNet>,
    mut commands: Commands,
    pool: Res<DebrisPool>,
) {
    for event in events.iter() {
        // Generate identical fragments using same seed
        let fragments = generate_voronoi_fragments(
            event.fracture_seed,
            DEFAULT_CELL_COUNT,
            event.impact_point,
        );

        // Spawn local debris (cosmetic, not authoritative)
        spawn_local_debris(&mut commands, &pool, fragments);
    }
}
```

**Profiling Integration (Phase 0):**
```rust
use engine_profiling::{profile_scope, ProfileCategory};

#[profile(category = "Physics")]
pub fn fracture_system(/* ... */) {
    profile_scope!("destruction_fracture");

    for event in fracture_events.iter() {
        {
            profile_scope!("voronoi_generation");
            let fragments = generate_voronoi_fragments(/* ... */);
        }

        {
            profile_scope!("debris_spawning");
            spawn_debris_fragments(/* ... */);
        }
    }
}
```

---

### 8.4 Asset Pipeline

**Offline Pre-Fracturing:**
1. Artist creates intact mesh in Blender/Maya
2. Export to engine format (GLTF or custom)
3. Run offline fracture tool (Houdini, custom script, or Blender addon)
4. Generate fractured variants with Voronoi/artist-authored patterns
5. Compute convex decomposition for collision (V-HACD)
6. Package as engine asset with metadata:
   ```yaml
   asset_id: "building_wall_01"
   intact_mesh: "wall_intact.mesh"
   fractured_variants:
     - pattern: "voronoi_light"
       seed: 12345
       fragments: 20
       mesh: "wall_fractured_light.mesh"
       collision: "wall_fractured_light_collision.mesh"
     - pattern: "voronoi_heavy"
       seed: 67890
       fragments: 100
       mesh: "wall_fractured_heavy.mesh"
       collision: "wall_fractured_heavy_collision.mesh"
   ```

**Runtime Workflow:**
1. Game triggers destruction event (explosion, projectile impact)
2. Server selects fracture pattern based on damage type/intensity
3. Server sends event to clients (entity ID + pattern ID or seed)
4. Clients swap mesh or generate fragments deterministically
5. Physics simulation runs (server-authoritative or client-local)
6. Cleanup system despawns debris after timeout

---

### 8.5 Performance Validation Checklist

Before merging destruction system:

- [ ] **Fracture trigger time** <5ms (p95) for 50 fragments
- [ ] **Fracture trigger time** <16ms (p99) for 200 fragments
- [ ] **Sustained destruction** (10 events/sec for 1 min) maintains >60 FPS
- [ ] **Memory usage** stable over 10-minute test (no leaks)
- [ ] **Fragment pool** does not exhaust under stress test
- [ ] **Collision accuracy** >90% visual match for convex hulls
- [ ] **Network bandwidth** <10 KB/sec per client (32 clients, 5 events/sec)
- [ ] **Server tick time** <16ms with 64 clients and sustained destruction
- [ ] **Cleanup system** maintains target fragment count (removes oldest/least important)
- [ ] **Profiling markers** present and integrated with Phase 0 infrastructure
- [ ] **All tests pass** (unit, integration, stress)
- [ ] **Cross-platform** tested (Windows, Linux, macOS)

---

## 9. Conclusion

### 9.1 Key Takeaways

1. **Server-side destruction** (The Finals approach) is the new gold standard for multiplayer synchronization
2. **Pre-fractured assets** remain most reliable for performance, but limit variety
3. **Hybrid approach** (pre-computed patterns + deterministic seeds) offers best balance for silmaril
4. **Object pooling** is mandatory for debris management (avoid fragmentation)
5. **Convex decomposition** is critical for collision performance
6. **LOD + cleanup** are essential for sustained destruction without performance collapse
7. **Deterministic physics** enables low-bandwidth networking but requires significant engineering effort
8. **Chaos Destruction** (UE5) is production-ready and highly optimized for those using Unreal

---

### 9.2 Recommended Implementation Path for silmaril

**Immediate (Phase 1 - 2-3 weeks):**
- Pre-fractured asset system with runtime swap
- Basic object pooling (2000 fragments)
- Server-authoritative triggers
- Time-based cleanup

**Near-term (Phase 2 - 4-6 weeks):**
- Runtime Voronoi with deterministic seeds
- Convex decomposition integration (V-HACD)
- Network sync (seed-based)
- Performance-based cleanup + LOD

**Future (Phase 3 - Post-MVP):**
- Stress-based fracture for hero moments
- Advanced network optimization (clustering, prioritization)
- Potential Havok Destruction integration (if AAA performance needed)

**Total Estimated Effort:** 8-12 weeks for full-featured destruction system

---

### 9.3 Sources Summary

This research synthesized information from:
- **Academic Research:** ACM, Springer, UC Berkeley, SIGGRAPH/Eurographics
- **AAA Game Engines:** Unreal Engine 5 (Chaos), Havok, PhysX
- **Indie Showcases:** Teardown, The Finals, Red Faction, Battlefield
- **Technical Articles:** Game Developer, Gaffer On Games, Hypesio, 80 Level
- **Open Source:** Godot VoronoiShatter, Bullet Physics, V-HACD

Complete source list with hyperlinks provided throughout document.

---

**Document Version:** 1.0
**Last Updated:** 2026-02-02
**Maintained By:** silmaril AI Research Agent

