# Phase 2.5: State Synchronization

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** Critical (core multiplayer functionality)

---

## 🎯 **Objective**

Implement state synchronization system that keeps clients in sync with server authority. Includes full snapshots, delta updates, automatic reconciliation, and adaptive sync strategy.

**Features:**
- Full state snapshots (initial + periodic)
- Delta updates (incremental changes)
- Automatic delta/snapshot selection (efficiency-based)
- Client state reconciliation
- Bandwidth optimization

---

## 📋 **Detailed Tasks**

### **1. Server State Manager** (Day 1-2)

**File:** `engine/networking/src/sync/server.rs`

```rust
use std::collections::HashMap;

/// Server-side state synchronization manager
pub struct ServerStateSynchronizer {
    /// Current world state
    world: World,

    /// Last full snapshot tick
    last_snapshot_tick: u64,

    /// Snapshot history (for delta calculation)
    snapshot_history: Vec<(u64, WorldState)>,

    /// Client states (last ack'd tick per client)
    client_states: HashMap<u64, ClientSyncState>,

    /// Configuration
    config: SyncConfig,
}

#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Send full snapshot every N ticks
    pub snapshot_interval: u64,

    /// Keep N snapshots in history for delta calculation
    pub snapshot_history_size: usize,

    /// Delta size threshold (% of full snapshot)
    /// If delta > threshold * full, send full instead
    pub delta_threshold: f32,

    /// Send rate (updates per second)
    pub update_rate: u32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            snapshot_interval: 60,  // Every 1 second at 60 TPS
            snapshot_history_size: 10,
            delta_threshold: 0.8,   // Send full if delta > 80% of full
            update_rate: 20,        // 20 updates/second
        }
    }
}

#[derive(Debug, Clone)]
struct ClientSyncState {
    client_id: u64,
    last_ack_tick: u64,
    last_sent_tick: u64,
}

impl ServerStateSynchronizer {
    pub fn new(world: World, config: SyncConfig) -> Self {
        Self {
            world,
            last_snapshot_tick: 0,
            snapshot_history: Vec::new(),
            client_states: HashMap::new(),
            config,
        }
    }

    /// Update world and generate sync messages
    pub fn tick(&mut self, tick: u64) -> Vec<(u64, Vec<u8>)> {
        // Save snapshot to history
        let current_state = WorldState::snapshot(&self.world);
        self.snapshot_history.push((tick, current_state.clone()));

        // Trim history
        if self.snapshot_history.len() > self.config.snapshot_history_size {
            self.snapshot_history.remove(0);
        }

        // Generate updates for each client
        let mut messages = Vec::new();

        for (client_id, client_state) in &mut self.client_states {
            let message = self.generate_update_for_client(*client_id, tick, client_state);
            messages.push((*client_id, message));
        }

        messages
    }

    /// Generate update for specific client
    fn generate_update_for_client(
        &self,
        client_id: u64,
        tick: u64,
        client_state: &mut ClientSyncState,
    ) -> Vec<u8> {
        // Check if should send full snapshot
        let should_send_full = tick - self.last_snapshot_tick >= self.config.snapshot_interval;

        if should_send_full {
            // Send full snapshot
            tracing::debug!("Sending full snapshot to client {} at tick {}", client_id, tick);
            let message = WorldSnapshotBuilder::build(tick, &self.world);
            client_state.last_sent_tick = tick;
            message
        } else {
            // Try to send delta
            if let Some(base_state) = self.find_base_state_for_client(client_state.last_ack_tick) {
                let current_state = WorldState::snapshot(&self.world);
                let delta = WorldStateDelta::compute(base_state, &current_state);

                // Encode delta
                let delta_message = WorldDeltaBuilder::build(
                    client_state.last_ack_tick,
                    tick,
                    &delta,
                );

                // Encode full snapshot for comparison
                let full_message = WorldSnapshotBuilder::build(tick, &self.world);

                // Check if delta is efficient
                if delta_message.len() as f32 / full_message.len() as f32
                    < self.config.delta_threshold
                {
                    tracing::debug!(
                        "Sending delta to client {} ({}% of full)",
                        client_id,
                        (delta_message.len() as f32 / full_message.len() as f32 * 100.0) as u32
                    );
                    client_state.last_sent_tick = tick;
                    delta_message
                } else {
                    tracing::debug!(
                        "Delta too large for client {}, sending full snapshot",
                        client_id
                    );
                    client_state.last_sent_tick = tick;
                    full_message
                }
            } else {
                // No base state found, send full
                tracing::debug!(
                    "No base state for client {}, sending full snapshot",
                    client_id
                );
                let message = WorldSnapshotBuilder::build(tick, &self.world);
                client_state.last_sent_tick = tick;
                message
            }
        }
    }

    /// Find base state for delta calculation
    fn find_base_state_for_client(&self, tick: u64) -> Option<&WorldState> {
        self.snapshot_history
            .iter()
            .find(|(t, _)| *t == tick)
            .map(|(_, state)| state)
    }

    /// Register new client
    pub fn add_client(&mut self, client_id: u64) {
        self.client_states.insert(
            client_id,
            ClientSyncState {
                client_id,
                last_ack_tick: 0,
                last_sent_tick: 0,
            },
        );
        tracing::info!("Client {} added to state synchronizer", client_id);
    }

    /// Remove client
    pub fn remove_client(&mut self, client_id: u64) {
        self.client_states.remove(&client_id);
        tracing::info!("Client {} removed from state synchronizer", client_id);
    }

    /// Client acknowledged tick
    pub fn acknowledge_tick(&mut self, client_id: u64, tick: u64) {
        if let Some(state) = self.client_states.get_mut(&client_id) {
            state.last_ack_tick = tick;
        }
    }
}
```

---

### **2. Client State Manager** (Day 2-3)

**File:** `engine/networking/src/sync/client.rs`

```rust
/// Client-side state synchronization
pub struct ClientStateSynchronizer {
    /// Local world state
    world: World,

    /// Server tick (latest received)
    server_tick: u64,

    /// Pending world states (for interpolation)
    pending_states: Vec<(u64, WorldState)>,

    /// Configuration
    config: ClientSyncConfig,
}

#[derive(Debug, Clone)]
pub struct ClientSyncConfig {
    /// Interpolation delay (ticks)
    pub interpolation_delay: u64,

    /// Max pending states
    pub max_pending_states: usize,
}

impl Default for ClientSyncConfig {
    fn default() -> Self {
        Self {
            interpolation_delay: 2, // 2 ticks delay for smooth interpolation
            max_pending_states: 10,
        }
    }
}

impl ClientStateSynchronizer {
    pub fn new(config: ClientSyncConfig) -> Self {
        Self {
            world: World::new(),
            server_tick: 0,
            pending_states: Vec::new(),
            config,
        }
    }

    /// Receive server update
    pub fn receive_update(&mut self, data: &[u8]) -> Result<(), NetworkError> {
        let packet = Protocol::decode_server_packet(data)?;

        match packet.message_type() {
            ServerMessage::WorldSnapshot => {
                let snapshot = packet.message_as_world_snapshot().unwrap();
                self.apply_snapshot(snapshot)?;
            }
            ServerMessage::WorldDelta => {
                let delta = packet.message_as_world_delta().unwrap();
                self.apply_delta(delta)?;
            }
            _ => {
                tracing::warn!("Unexpected server message type");
            }
        }

        Ok(())
    }

    /// Apply full snapshot
    fn apply_snapshot(&mut self, snapshot: WorldSnapshot) -> Result<(), NetworkError> {
        let tick = snapshot.tick();

        // Convert FlatBuffers snapshot to WorldState
        let world_state = self.convert_snapshot(snapshot)?;

        // Add to pending states
        self.pending_states.push((tick, world_state));

        // Trim pending states
        if self.pending_states.len() > self.config.max_pending_states {
            self.pending_states.remove(0);
        }

        self.server_tick = tick;

        tracing::debug!("Received snapshot at tick {}", tick);

        Ok(())
    }

    /// Apply delta update
    fn apply_delta(&mut self, delta: WorldDelta) -> Result<(), NetworkError> {
        let base_tick = delta.base_tick();
        let target_tick = delta.target_tick();

        // Find base state
        let base_state = self
            .pending_states
            .iter()
            .find(|(t, _)| *t == base_tick)
            .map(|(_, s)| s.clone())
            .ok_or_else(|| NetworkError::DeltaBaseMissing {
                tick: base_tick,
            })?;

        // Convert FlatBuffers delta to WorldStateDelta
        let world_delta = self.convert_delta(delta)?;

        // Apply delta to base state
        let mut new_state = base_state;
        world_delta.apply(&mut new_state);

        // Add to pending states
        self.pending_states.push((target_tick, new_state));

        // Trim pending states
        if self.pending_states.len() > self.config.max_pending_states {
            self.pending_states.remove(0);
        }

        self.server_tick = target_tick;

        tracing::debug!("Received delta: {} -> {}", base_tick, target_tick);

        Ok(())
    }

    /// Interpolate state for rendering
    pub fn interpolate(&mut self, render_tick: u64) {
        // Find two states to interpolate between
        let target_tick = render_tick.saturating_sub(self.config.interpolation_delay);

        if self.pending_states.len() < 2 {
            return; // Not enough states for interpolation
        }

        // Find states before and after target tick
        let (state_before, state_after) = self.find_interpolation_states(target_tick);

        if let (Some((tick_before, state_before)), Some((tick_after, state_after))) =
            (state_before, state_after)
        {
            // Calculate interpolation factor
            let t = if tick_after > tick_before {
                (target_tick - tick_before) as f32 / (tick_after - tick_before) as f32
            } else {
                0.0
            };

            // Interpolate and apply to world
            let interpolated = self.interpolate_states(state_before, state_after, t);
            interpolated.restore(&mut self.world);

            tracing::trace!("Interpolated at t={:.2} between ticks {} and {}", t, tick_before, tick_after);
        } else {
            // Use most recent state
            if let Some((_, state)) = self.pending_states.last() {
                state.restore(&mut self.world);
            }
        }
    }

    /// Find states for interpolation
    fn find_interpolation_states(
        &self,
        target_tick: u64,
    ) -> (Option<(u64, &WorldState)>, Option<(u64, &WorldState)>) {
        let mut before = None;
        let mut after = None;

        for (tick, state) in &self.pending_states {
            if *tick <= target_tick {
                before = Some((*tick, state));
            } else if after.is_none() {
                after = Some((*tick, state));
                break;
            }
        }

        (before, after)
    }

    /// Interpolate between two states
    fn interpolate_states(&self, state_a: &WorldState, state_b: &WorldState, t: f32) -> WorldState {
        // For now, simple component-wise interpolation
        // In practice, would interpolate Transform positions, etc.

        // Clone state_a as base
        let mut interpolated = state_a.clone();

        // Interpolate components for matching entities
        for (entity, components_b) in &state_b.components {
            if let Some(components_a) = state_a.components.get(entity) {
                // Interpolate each component
                let interpolated_components: Vec<_> = components_a
                    .iter()
                    .zip(components_b.iter())
                    .map(|(comp_a, comp_b)| self.interpolate_component(comp_a, comp_b, t))
                    .collect();

                interpolated.components.insert(*entity, interpolated_components);
            }
        }

        interpolated
    }

    /// Interpolate single component
    fn interpolate_component(&self, comp_a: &ComponentData, comp_b: &ComponentData, t: f32) -> ComponentData {
        match (comp_a, comp_b) {
            (ComponentData::Transform(a), ComponentData::Transform(b)) => {
                ComponentData::Transform(Transform {
                    position: a.position.lerp(b.position, t),
                    rotation: a.rotation.slerp(b.rotation, t),
                    scale: a.scale.lerp(b.scale, t),
                })
            }
            // Other components - just use state_b
            _ => comp_b.clone(),
        }
    }

    // Conversion helpers...
    fn convert_snapshot(&self, snapshot: WorldSnapshot) -> Result<WorldState, NetworkError> {
        // Convert FlatBuffers to WorldState
        todo!()
    }

    fn convert_delta(&self, delta: WorldDelta) -> Result<WorldStateDelta, NetworkError> {
        // Convert FlatBuffers to WorldStateDelta
        todo!()
    }

    pub fn world(&self) -> &World {
        &self.world
    }
}
```

---

### **3. Bandwidth Optimization** (Day 3-4)

**File:** `engine/networking/src/sync/bandwidth.rs`

```rust
/// Bandwidth statistics tracker
pub struct BandwidthTracker {
    /// Bytes sent (windowed)
    bytes_sent: Vec<(Instant, usize)>,

    /// Window duration
    window_duration: Duration,
}

impl BandwidthTracker {
    pub fn new(window_duration: Duration) -> Self {
        Self {
            bytes_sent: Vec::new(),
            window_duration,
        }
    }

    /// Record bytes sent
    pub fn record_sent(&mut self, bytes: usize) {
        self.bytes_sent.push((Instant::now(), bytes));
        self.trim_old_entries();
    }

    /// Get current bandwidth usage (bytes/second)
    pub fn bytes_per_second(&mut self) -> f64 {
        self.trim_old_entries();

        let total_bytes: usize = self.bytes_sent.iter().map(|(_, bytes)| bytes).sum();
        let elapsed = self.window_duration.as_secs_f64();

        total_bytes as f64 / elapsed
    }

    /// Trim entries outside window
    fn trim_old_entries(&mut self) {
        let cutoff = Instant::now() - self.window_duration;
        self.bytes_sent.retain(|(time, _)| *time >= cutoff);
    }
}

/// Adaptive sync rate controller
pub struct AdaptiveSyncRate {
    current_rate: u32,
    target_bandwidth: usize, // bytes/second
    bandwidth_tracker: BandwidthTracker,
}

impl AdaptiveSyncRate {
    pub fn new(initial_rate: u32, target_bandwidth: usize) -> Self {
        Self {
            current_rate: initial_rate,
            target_bandwidth,
            bandwidth_tracker: BandwidthTracker::new(Duration::from_secs(1)),
        }
    }

    /// Adjust rate based on current bandwidth usage
    pub fn adjust_rate(&mut self) -> u32 {
        let current_bandwidth = self.bandwidth_tracker.bytes_per_second();

        if current_bandwidth > self.target_bandwidth as f64 * 1.1 {
            // Reduce rate if over budget
            self.current_rate = (self.current_rate as f32 * 0.9) as u32;
            self.current_rate = self.current_rate.max(1);
        } else if current_bandwidth < self.target_bandwidth as f64 * 0.8 {
            // Increase rate if under budget
            self.current_rate = (self.current_rate as f32 * 1.1) as u32;
            self.current_rate = self.current_rate.min(60);
        }

        self.current_rate
    }

    pub fn record_sent(&mut self, bytes: usize) {
        self.bandwidth_tracker.record_sent(bytes);
    }
}
```

---

### **4. Reconciliation** (Day 4-5)

**File:** `engine/networking/src/sync/reconciliation.rs`

```rust
/// State reconciliation for mispredicted clients
pub struct StateReconciliation {
    /// Predicted states (local simulation)
    predicted_states: Vec<(u64, WorldState)>,

    /// Max predicted states to keep
    max_predicted_states: usize,
}

impl StateReconciliation {
    pub fn new(max_predicted_states: usize) -> Self {
        Self {
            predicted_states: Vec::new(),
            max_predicted_states,
        }
    }

    /// Record predicted state
    pub fn record_prediction(&mut self, tick: u64, state: WorldState) {
        self.predicted_states.push((tick, state));

        if self.predicted_states.len() > self.max_predicted_states {
            self.predicted_states.remove(0);
        }
    }

    /// Reconcile with authoritative server state
    pub fn reconcile(
        &mut self,
        server_tick: u64,
        server_state: &WorldState,
        current_tick: u64,
    ) -> Option<WorldState> {
        // Find predicted state at server tick
        let predicted = self
            .predicted_states
            .iter()
            .find(|(t, _)| *t == server_tick)
            .map(|(_, s)| s);

        if let Some(predicted_state) = predicted {
            // Check if prediction matches server
            if Self::states_match(predicted_state, server_state) {
                tracing::trace!("Prediction matched server at tick {}", server_tick);
                None // No correction needed
            } else {
                tracing::debug!("Prediction mismatch at tick {}, reconciling", server_tick);

                // Rewind and replay
                let mut corrected_state = server_state.clone();

                // Replay all inputs from server_tick to current_tick
                for tick in server_tick + 1..=current_tick {
                    // Apply input for this tick
                    // (Would need to store input history)
                    // corrected_state = apply_input(corrected_state, tick);
                }

                Some(corrected_state)
            }
        } else {
            // No prediction found, use server state
            Some(server_state.clone())
        }
    }

    /// Check if two states match (within tolerance)
    fn states_match(a: &WorldState, b: &WorldState) -> bool {
        // Simple check: same entity count
        if a.entities.len() != b.entities.len() {
            return false;
        }

        // Check each entity's Transform (main component for reconciliation)
        for entity_a in &a.entities {
            if let Some(components_a) = a.components.get(&entity_a.entity) {
                if let Some(components_b) = b.components.get(&entity_a.entity) {
                    // Find Transform components
                    let transform_a = components_a.iter().find_map(|c| match c {
                        ComponentData::Transform(t) => Some(t),
                        _ => None,
                    });

                    let transform_b = components_b.iter().find_map(|c| match c {
                        ComponentData::Transform(t) => Some(t),
                        _ => None,
                    });

                    if let (Some(ta), Some(tb)) = (transform_a, transform_b) {
                        // Check if positions are close (within 0.01 units)
                        if ta.position.distance(tb.position) > 0.01 {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Server sends full snapshots periodically
- [ ] Server sends delta updates between snapshots
- [ ] Delta/snapshot selection automatic (efficiency-based)
- [ ] Client receives and applies snapshots
- [ ] Client receives and applies deltas
- [ ] Client interpolates for smooth rendering
- [ ] State reconciliation works
- [ ] Bandwidth tracking accurate
- [ ] Adaptive rate adjustment works
- [ ] No desyncs after 1000+ ticks

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Snapshot encode (1000 entities) | < 5ms | < 10ms |
| Delta encode (100 changes) | < 1ms | < 3ms |
| Client apply snapshot | < 3ms | < 8ms |
| Client apply delta | < 0.5ms | < 2ms |
| Interpolation (per frame) | < 1ms | < 3ms |
| Bandwidth (per client) | < 20 KB/s | < 50 KB/s |

---

**Dependencies:** [phase2-network-protocol.md](phase2-network-protocol.md), [phase2-tcp-connection.md](phase2-tcp-connection.md)
**Next:** [phase2-server-tick.md](phase2-server-tick.md)
