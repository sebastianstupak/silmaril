# Phase 5.2: MMORPG Example

**Status:** ⚪ Not Started
**Estimated Time:** 5-6 days
**Priority:** High (demonstrates multiplayer capabilities)

---

## 🎯 **Objective**

Create a multiplayer online RPG example that showcases the engine's networking, state synchronization, and server architecture. This demonstrates how to build massively multiplayer games with the Silmaril.

**Game Concept:**
- **Genre:** Online multiplayer RPG
- **Players:** Multiple concurrent players
- **Features:** Chat, parties, quests, trading, PvE combat
- **Server:** Authoritative game server (60 TPS)
- **Client:** Predictive movement with server reconciliation
- **Persistence:** Save player progress to database

---

## 📋 **Detailed Tasks**

### **1. Project Setup** (Day 1 Morning)

**Directory Structure:**
```
examples/mmorpg/
├── Cargo.toml
├── server/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── server.rs
│   │   ├── systems/
│   │   │   ├── mod.rs
│   │   │   ├── combat.rs
│   │   │   ├── quests.rs
│   │   │   └── trading.rs
│   │   ├── database/
│   │   │   ├── mod.rs
│   │   │   └── models.rs
│   │   └── config.toml
│   └── README.md
├── client/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── client.rs
│   │   ├── ui/
│   │   │   ├── mod.rs
│   │   │   ├── chat.rs
│   │   │   ├── inventory.rs
│   │   │   └── quest_log.rs
│   │   └── prediction.rs
│   └── README.md
└── shared/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── components.rs
        ├── protocol.fbs
        └── messages.rs
```

**File:** `examples/mmorpg/Cargo.toml`

```toml
[workspace]
members = ["server", "client", "shared"]

[workspace.dependencies]
silmaril-core = { path = "../../engine/core" }
silmaril-networking = { path = "../../engine/networking" }
silmaril-macros = { path = "../../engine/macros" }
glam = "0.24"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
anyhow = "1.0"
```

---

### **2. Shared Components & Protocol** (Day 1)

**File:** `examples/mmorpg/shared/src/components.rs`

```rust
use silmaril_core::prelude::*;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Player character
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub account_id: u64,
    pub character_name: String,
    pub level: u32,
    pub experience: u64,
    pub gold: u32,
}

/// Character stats
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Stats {
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub strength: u32,
    pub intelligence: u32,
    pub agility: u32,
}

impl Stats {
    pub fn new(level: u32) -> Self {
        let base_health = 100.0 + (level as f32 * 10.0);
        let base_mana = 50.0 + (level as f32 * 5.0);

        Self {
            health: base_health,
            max_health: base_health,
            mana: base_mana,
            max_mana: base_mana,
            strength: 10 + level,
            intelligence: 10 + level,
            agility: 10 + level,
        }
    }
}

/// Network player connection
#[derive(Component, Debug, Clone, Copy)]
pub struct NetworkPlayer {
    pub connection_id: u64,
    pub last_input_tick: u64,
}

/// Transform with interpolation support
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: f32,
    pub velocity: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: 0.0,
            velocity: Vec3::ZERO,
        }
    }
}

/// Character class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CharacterClass {
    Warrior,
    Mage,
    Rogue,
    Healer,
}

#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Class {
    pub class_type: CharacterClass,
}

/// Inventory system
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<Item>>,
    pub capacity: usize,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
            capacity,
        }
    }

    pub fn add_item(&mut self, item: Item) -> bool {
        for slot in &mut self.slots {
            if slot.is_none() {
                *slot = Some(item);
                return true;
            }
        }
        false // Inventory full
    }

    pub fn remove_item(&mut self, slot: usize) -> Option<Item> {
        self.slots.get_mut(slot)?.take()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub item_id: u32,
    pub name: String,
    pub item_type: ItemType,
    pub rarity: ItemRarity,
    pub stats: ItemStats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemType {
    Weapon,
    Armor,
    Consumable,
    QuestItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ItemStats {
    pub damage: f32,
    pub armor: f32,
    pub health_bonus: f32,
    pub mana_bonus: f32,
}

/// Quest system
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct QuestLog {
    pub active_quests: Vec<Quest>,
    pub completed_quests: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub quest_id: u32,
    pub title: String,
    pub description: String,
    pub objectives: Vec<QuestObjective>,
    pub rewards: QuestRewards,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    pub description: String,
    pub current: u32,
    pub target: u32,
    pub completed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QuestRewards {
    pub experience: u64,
    pub gold: u32,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub channel: ChatChannel,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatChannel {
    Global,
    Party,
    Whisper,
    System,
}

/// Party system
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub party_id: u64,
    pub leader: u64,
    pub members: Vec<u64>,
    pub max_members: usize,
}

impl Party {
    pub fn new(leader: u64) -> Self {
        Self {
            party_id: rand::random(),
            leader,
            members: vec![leader],
            max_members: 5,
        }
    }

    pub fn add_member(&mut self, player_id: u64) -> bool {
        if self.members.len() < self.max_members {
            self.members.push(player_id);
            true
        } else {
            false
        }
    }

    pub fn remove_member(&mut self, player_id: u64) {
        self.members.retain(|&id| id != player_id);
    }
}

/// NPC
#[derive(Component, Debug, Clone)]
pub struct Npc {
    pub npc_id: u32,
    pub name: String,
    pub npc_type: NpcType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcType {
    Merchant,
    QuestGiver,
    Enemy,
    Friendly,
}
```

---

### **3. Server Implementation** (Day 2-3)

**File:** `examples/mmorpg/server/src/server.rs`

```rust
use silmaril_core::prelude::*;
use silmaril_networking::server::*;
use mmorpg_shared::components::*;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use anyhow::Result;

pub struct MmorpgServer {
    world: World,
    network: NetworkServer,
    current_tick: u64,
    tick_rate: u32,
    tick_duration: Duration,

    // Player management
    players: HashMap<u64, PlayerSession>,
    parties: HashMap<u64, Party>,

    // Game state
    spawn_points: Vec<Vec3>,
    npcs: Vec<Entity>,
}

#[derive(Debug)]
struct PlayerSession {
    connection_id: u64,
    account_id: u64,
    entity: Option<Entity>,
    character_name: String,
    last_heartbeat: Instant,
}

impl MmorpgServer {
    pub async fn new(config: ServerConfig) -> Result<Self> {
        tracing::info!("Starting MMORPG server...");

        let mut world = World::new();
        Self::register_components(&mut world);

        let network = NetworkServer::new(&config).await?;

        let mut server = Self {
            world,
            network,
            current_tick: 0,
            tick_rate: config.tick_rate,
            tick_duration: Duration::from_secs_f64(1.0 / config.tick_rate as f64),
            players: HashMap::new(),
            parties: HashMap::new(),
            spawn_points: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(-10.0, 0.0, -10.0),
            ],
            npcs: Vec::new(),
        };

        // Spawn NPCs
        server.spawn_npcs();

        Ok(server)
    }

    fn register_components(world: &mut World) {
        world.register::<Player>();
        world.register::<Stats>();
        world.register::<NetworkPlayer>();
        world.register::<Transform>();
        world.register::<Class>();
        world.register::<Inventory>();
        world.register::<QuestLog>();
        world.register::<Party>();
        world.register::<Npc>();
    }

    fn spawn_npcs(&mut self) {
        // Spawn merchants
        for i in 0..3 {
            let npc = self.world.spawn();
            self.world.add(npc, Npc {
                npc_id: i,
                name: format!("Merchant {}", i),
                npc_type: NpcType::Merchant,
            });
            self.world.add(npc, Transform {
                position: Vec3::new(i as f32 * 5.0, 0.0, 0.0),
                ..Default::default()
            });
            self.npcs.push(npc);
        }

        // Spawn quest givers
        for i in 0..5 {
            let npc = self.world.spawn();
            self.world.add(npc, Npc {
                npc_id: 100 + i,
                name: format!("Quest Giver {}", i),
                npc_type: NpcType::QuestGiver,
            });
            self.world.add(npc, Transform {
                position: Vec3::new(0.0, 0.0, i as f32 * 8.0),
                ..Default::default()
            });
            self.npcs.push(npc);
        }

        tracing::info!("Spawned {} NPCs", self.npcs.len());
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Server running at {} TPS", self.tick_rate);

        loop {
            let tick_start = Instant::now();

            // Process network events
            self.process_network_events().await?;

            // Run game tick
            self.tick();

            // Send state updates
            self.send_state_updates().await?;

            // Sleep until next tick
            let elapsed = tick_start.elapsed();
            if elapsed < self.tick_duration {
                tokio::time::sleep(self.tick_duration - elapsed).await;
            } else {
                tracing::warn!(
                    "Tick overran: {:.2}ms (budget: {:.2}ms)",
                    elapsed.as_secs_f64() * 1000.0,
                    self.tick_duration.as_secs_f64() * 1000.0
                );
            }

            self.current_tick += 1;

            // Log stats every 60 ticks (1 second)
            if self.current_tick % 60 == 0 {
                self.log_stats();
            }
        }
    }

    async fn process_network_events(&mut self) -> Result<()> {
        while let Some(event) = self.network.poll_event() {
            match event {
                NetworkEvent::ClientConnected { connection_id, addr } => {
                    self.handle_client_connected(connection_id, addr);
                }
                NetworkEvent::ClientDisconnected { connection_id } => {
                    self.handle_client_disconnected(connection_id);
                }
                NetworkEvent::MessageReceived { connection_id, data } => {
                    self.handle_client_message(connection_id, data)?;
                }
            }
        }
        Ok(())
    }

    fn handle_client_connected(&mut self, connection_id: u64, addr: String) {
        tracing::info!("Client {} connected from {}", connection_id, addr);

        // Client will send LoginRequest next
    }

    fn handle_client_disconnected(&mut self, connection_id: u64) {
        tracing::info!("Client {} disconnected", connection_id);

        if let Some(session) = self.players.remove(&connection_id) {
            // Remove player entity
            if let Some(entity) = session.entity {
                self.world.despawn(entity);
            }

            // Remove from party
            for party in self.parties.values_mut() {
                party.remove_member(session.account_id);
            }

            // Broadcast disconnect
            self.broadcast_chat(ChatMessage {
                sender: "Server".to_string(),
                channel: ChatChannel::System,
                message: format!("{} has left the game", session.character_name),
                timestamp: self.current_tick,
            });
        }
    }

    fn handle_client_message(&mut self, connection_id: u64, data: Vec<u8>) -> Result<()> {
        // Deserialize message
        let message: ClientMessage = bincode::deserialize(&data)?;

        match message {
            ClientMessage::Login { account_id, character_name } => {
                self.handle_login(connection_id, account_id, character_name)?;
            }
            ClientMessage::PlayerInput { input } => {
                self.handle_player_input(connection_id, input)?;
            }
            ClientMessage::Chat { message } => {
                self.handle_chat(connection_id, message)?;
            }
            ClientMessage::PartyInvite { target_player } => {
                self.handle_party_invite(connection_id, target_player)?;
            }
            ClientMessage::QuestAccept { quest_id } => {
                self.handle_quest_accept(connection_id, quest_id)?;
            }
            ClientMessage::Trade { target_player } => {
                self.handle_trade_request(connection_id, target_player)?;
            }
        }

        Ok(())
    }

    fn handle_login(
        &mut self,
        connection_id: u64,
        account_id: u64,
        character_name: String,
    ) -> Result<()> {
        tracing::info!("Player '{}' logging in (account {})", character_name, account_id);

        // Load player data from database (TODO)
        // For now, create new character

        let spawn_point = self.spawn_points[account_id as usize % self.spawn_points.len()];

        // Spawn player entity
        let entity = self.world.spawn();

        self.world.add(entity, Player {
            account_id,
            character_name: character_name.clone(),
            level: 1,
            experience: 0,
            gold: 100,
        });

        self.world.add(entity, Stats::new(1));
        self.world.add(entity, NetworkPlayer {
            connection_id,
            last_input_tick: 0,
        });
        self.world.add(entity, Transform {
            position: spawn_point,
            ..Default::default()
        });
        self.world.add(entity, Class {
            class_type: CharacterClass::Warrior,
        });
        self.world.add(entity, Inventory::new(20));
        self.world.add(entity, QuestLog {
            active_quests: Vec::new(),
            completed_quests: Vec::new(),
        });

        // Create session
        let session = PlayerSession {
            connection_id,
            account_id,
            entity: Some(entity),
            character_name: character_name.clone(),
            last_heartbeat: Instant::now(),
        };

        self.players.insert(connection_id, session);

        // Send login success
        self.network.send_to_client(
            connection_id,
            ServerMessage::LoginSuccess {
                player_entity: entity,
                world_state: self.serialize_world_state(),
            },
        ).await?;

        // Broadcast join
        self.broadcast_chat(ChatMessage {
            sender: "Server".to_string(),
            channel: ChatChannel::System,
            message: format!("{} has joined the game", character_name),
            timestamp: self.current_tick,
        });

        Ok(())
    }

    fn tick(&mut self) {
        // Update player stats (regen)
        self.stats_regen_system();

        // Update quests
        self.quest_update_system();

        // Apply movement
        self.movement_system();

        // Combat (if any)
        self.combat_system();

        // Cleanup
        self.cleanup_system();
    }

    fn stats_regen_system(&mut self) {
        for (_, stats) in self.world.query::<&mut Stats>() {
            // Regen health
            if stats.health < stats.max_health {
                stats.health = (stats.health + 1.0).min(stats.max_health);
            }

            // Regen mana
            if stats.mana < stats.max_mana {
                stats.mana = (stats.mana + 2.0).min(stats.max_mana);
            }
        }
    }

    fn movement_system(&mut self) {
        let dt = self.tick_duration.as_secs_f32();

        for (_, transform) in self.world.query::<&mut Transform>() {
            transform.position += transform.velocity * dt;

            // Simple bounds checking
            transform.position.x = transform.position.x.clamp(-100.0, 100.0);
            transform.position.z = transform.position.z.clamp(-100.0, 100.0);
        }
    }

    fn broadcast_chat(&mut self, message: ChatMessage) {
        let msg = ServerMessage::Chat { message };

        for (connection_id, _) in &self.players {
            let _ = self.network.send_to_client(*connection_id, msg.clone());
        }
    }

    fn log_stats(&self) {
        let entity_count = self.world.entity_count();
        let player_count = self.players.len();

        tracing::info!(
            "Tick {}: {} players, {} entities",
            self.current_tick,
            player_count,
            entity_count
        );
    }

    async fn send_state_updates(&mut self) -> Result<()> {
        // Send state to each player
        for (connection_id, session) in &self.players {
            if let Some(entity) = session.entity {
                let state = self.get_player_view(entity);

                self.network.send_to_client(
                    *connection_id,
                    ServerMessage::StateUpdate { state },
                ).await?;
            }
        }

        Ok(())
    }

    fn get_player_view(&self, player_entity: Entity) -> WorldState {
        // Get player position
        let player_pos = self.world.get::<Transform>(player_entity)
            .map(|t| t.position)
            .unwrap_or(Vec3::ZERO);

        // Find nearby entities (100 unit radius)
        let nearby_entities = self.world
            .query::<(Entity, &Transform)>()
            .filter(|(_, transform)| {
                transform.position.distance(player_pos) < 100.0
            })
            .map(|(entity, _)| entity)
            .collect();

        WorldState {
            tick: self.current_tick,
            entities: nearby_entities,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ClientMessage {
    Login { account_id: u64, character_name: String },
    PlayerInput { input: PlayerInput },
    Chat { message: String },
    PartyInvite { target_player: u64 },
    QuestAccept { quest_id: u32 },
    Trade { target_player: u64 },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ServerMessage {
    LoginSuccess { player_entity: Entity, world_state: WorldState },
    StateUpdate { state: WorldState },
    Chat { message: ChatMessage },
    PartyUpdate { party: Party },
    QuestUpdate { quest: Quest },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlayerInput {
    pub sequence: u32,
    pub movement: Vec3,
    pub actions: Vec<PlayerAction>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlayerAction {
    Attack,
    UseAbility { ability_id: u32 },
    Interact { target: Entity },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldState {
    pub tick: u64,
    pub entities: Vec<Entity>,
}
```

---

### **4. Client Implementation** (Day 4-5)

**File:** `examples/mmorpg/client/src/client.rs`

```rust
use silmaril_core::prelude::*;
use silmaril_networking::client::*;
use silmaril_platform::{Platform, Input};
use silmaril_rendering::Renderer;
use mmorpg_shared::components::*;
use std::collections::VecDeque;
use anyhow::Result;

pub struct MmorpgClient {
    world: World,
    network: NetworkClient,

    // Player state
    player_entity: Option<Entity>,
    input_sequence: u32,

    // Client-side prediction
    pending_inputs: VecDeque<PlayerInput>,
    last_server_tick: u64,

    // UI state
    chat_messages: Vec<ChatMessage>,
    ui_state: UiState,
}

#[derive(Debug, Default)]
struct UiState {
    show_inventory: bool,
    show_quest_log: bool,
    show_chat: bool,
    chat_input: String,
}

impl MmorpgClient {
    pub async fn new(server_addr: &str) -> Result<Self> {
        tracing::info!("Connecting to server: {}", server_addr);

        let network = NetworkClient::connect(server_addr).await?;
        let mut world = World::new();
        Self::register_components(&mut world);

        Ok(Self {
            world,
            network,
            player_entity: None,
            input_sequence: 0,
            pending_inputs: VecDeque::new(),
            last_server_tick: 0,
            chat_messages: Vec::new(),
            ui_state: UiState::default(),
        })
    }

    fn register_components(world: &mut World) {
        world.register::<Player>();
        world.register::<Stats>();
        world.register::<Transform>();
        world.register::<Class>();
        world.register::<Inventory>();
        world.register::<QuestLog>();
    }

    pub async fn login(&mut self, account_id: u64, character_name: String) -> Result<()> {
        self.network.send(ClientMessage::Login {
            account_id,
            character_name,
        }).await?;

        // Wait for login response
        while let Some(message) = self.network.receive().await {
            if let ServerMessage::LoginSuccess { player_entity, world_state } = message {
                self.player_entity = Some(player_entity);
                self.apply_world_state(world_state);
                tracing::info!("Login successful, player entity: {:?}", player_entity);
                return Ok(());
            }
        }

        Err(anyhow::anyhow!("Login failed"))
    }

    pub fn update(&mut self, input: &Input, dt: f32) -> Result<()> {
        // Process network messages
        while let Ok(message) = self.network.try_receive() {
            self.handle_server_message(message)?;
        }

        // Generate player input
        let player_input = self.generate_input(input);

        // Apply input locally (client-side prediction)
        self.apply_input_locally(&player_input, dt);

        // Send input to server
        self.network.send_async(ClientMessage::PlayerInput {
            input: player_input.clone(),
        });

        // Store for reconciliation
        self.pending_inputs.push_back(player_input);

        // Keep only last 60 inputs (1 second at 60 FPS)
        while self.pending_inputs.len() > 60 {
            self.pending_inputs.pop_front();
        }

        Ok(())
    }

    fn generate_input(&mut self, input: &Input) -> PlayerInput {
        let mut movement = Vec3::ZERO;

        if input.is_key_pressed("W") {
            movement.z += 1.0;
        }
        if input.is_key_pressed("S") {
            movement.z -= 1.0;
        }
        if input.is_key_pressed("A") {
            movement.x -= 1.0;
        }
        if input.is_key_pressed("D") {
            movement.x += 1.0;
        }

        if movement.length_squared() > 0.0 {
            movement = movement.normalize();
        }

        let mut actions = Vec::new();

        if input.is_key_just_pressed("Space") {
            actions.push(PlayerAction::Attack);
        }

        if input.is_key_just_pressed("1") {
            actions.push(PlayerAction::UseAbility { ability_id: 1 });
        }

        self.input_sequence += 1;

        PlayerInput {
            sequence: self.input_sequence,
            movement,
            actions,
        }
    }

    fn apply_input_locally(&mut self, input: &PlayerInput, dt: f32) {
        if let Some(entity) = self.player_entity {
            if let Some(transform) = self.world.get_mut::<Transform>(entity) {
                const MOVE_SPEED: f32 = 5.0;
                transform.velocity = input.movement * MOVE_SPEED;
                transform.position += transform.velocity * dt;
            }
        }
    }

    fn handle_server_message(&mut self, message: ServerMessage) -> Result<()> {
        match message {
            ServerMessage::StateUpdate { state } => {
                self.apply_world_state(state);
                self.reconcile_prediction();
            }
            ServerMessage::Chat { message } => {
                self.chat_messages.push(message);
            }
            ServerMessage::PartyUpdate { party } => {
                // Update party UI
                tracing::debug!("Party updated: {:?}", party);
            }
            ServerMessage::QuestUpdate { quest } => {
                // Update quest log
                tracing::debug!("Quest updated: {:?}", quest);
            }
            _ => {}
        }

        Ok(())
    }

    fn apply_world_state(&mut self, state: WorldState) {
        self.last_server_tick = state.tick;

        // Update entities from server
        for entity_data in state.entities {
            // Deserialize and update
            // (Implementation depends on serialization format)
        }
    }

    fn reconcile_prediction(&mut self) {
        // Server reconciliation: replay inputs since last server state
        // This corrects any prediction errors

        if let Some(entity) = self.player_entity {
            // Find server-confirmed input
            let confirmed_sequence = self.last_server_tick as u32;

            // Remove confirmed inputs
            while let Some(input) = self.pending_inputs.front() {
                if input.sequence <= confirmed_sequence {
                    self.pending_inputs.pop_front();
                } else {
                    break;
                }
            }

            // Replay remaining inputs
            // (This corrects client position if prediction was wrong)
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer) -> Result<()> {
        // Render world
        for (_, (transform, player)) in self.world.query::<(&Transform, &Player)>() {
            renderer.draw_character(
                &player.character_name,
                transform.position,
                transform.rotation,
            )?;
        }

        // Render UI
        self.render_ui(renderer)?;

        Ok(())
    }

    fn render_ui(&self, renderer: &mut Renderer) -> Result<()> {
        // Chat window
        if self.ui_state.show_chat {
            for (i, msg) in self.chat_messages.iter().rev().take(10).enumerate() {
                renderer.draw_text(
                    &format!("[{}] {}: {}",
                        match msg.channel {
                            ChatChannel::Global => "Global",
                            ChatChannel::Party => "Party",
                            ChatChannel::Whisper => "Whisper",
                            ChatChannel::System => "System",
                        },
                        msg.sender,
                        msg.message
                    ),
                    10.0,
                    550.0 - (i as f32 * 20.0),
                )?;
            }
        }

        // Other UI elements...

        Ok(())
    }
}
```

---

### **5. Main Binaries** (Day 5-6)

**File:** `examples/mmorpg/server/src/main.rs`

```rust
mod server;

use server::*;
use anyhow::Result;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = ServerConfig {
        bind_addr: "0.0.0.0:7777".to_string(),
        tick_rate: 60,
        max_players: 1000,
    };

    let mut server = MmorpgServer::new(config).await?;

    // Spawn server task
    let server_task = tokio::spawn(async move {
        server.run().await
    });

    // Wait for shutdown
    signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    server_task.abort();

    Ok(())
}
```

**File:** `examples/mmorpg/client/src/main.rs`

```rust
mod client;

use client::*;
use silmaril_platform::{Platform, WindowConfig};
use silmaril_rendering::Renderer;
use anyhow::Result;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Connect to server
    let mut client = MmorpgClient::new("127.0.0.1:7777").await?;

    // Login
    let account_id = rand::random();
    let character_name = format!("Player_{}", account_id % 1000);
    client.login(account_id, character_name).await?;

    // Create window
    let mut platform = Platform::new()?;
    let window = platform.create_window(WindowConfig {
        title: "MMORPG Example".to_string(),
        width: 1280,
        height: 720,
        ..Default::default()
    })?;

    let mut renderer = Renderer::new(&window)?;

    // Game loop
    let mut last_frame = Instant::now();

    while !client.should_quit() {
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;

        let input = platform.poll_events();

        client.update(&input, dt)?;

        renderer.begin_frame()?;
        client.render(&mut renderer)?;
        renderer.end_frame()?;
    }

    Ok(())
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Server runs stable at 60 TPS
- [ ] Multiple clients can connect simultaneously
- [ ] Player movement synchronized across clients
- [ ] Chat system works (global, party, whisper)
- [ ] Party system allows grouping
- [ ] Quest system tracks objectives
- [ ] Inventory system stores items
- [ ] Client-side prediction reduces perceived latency
- [ ] Server reconciliation corrects prediction errors
- [ ] NPCs spawn and persist
- [ ] 100+ concurrent players supported
- [ ] No memory leaks during extended sessions
- [ ] Graceful disconnect handling

---

## 🎯 **Performance Targets**

| Metric | Target | Critical |
|--------|--------|----------|
| Server tick rate | 60 TPS | 50 TPS |
| Concurrent players | 100+ | 50+ |
| Network latency | < 50ms | < 100ms |
| Client FPS | 60 FPS | 30 FPS |
| Memory (server) | < 1 GB | < 2 GB |
| Memory (client) | < 500 MB | < 1 GB |
| State sync size | < 10 KB/player | < 50 KB |

---

**Dependencies:** Phase 2 (Networking), Phase 3 (Rendering)
**Next:** [phase5-turnbased-example.md](phase5-turnbased-example.md)
