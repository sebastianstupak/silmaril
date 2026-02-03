# Phase 5.4: MOBA Example

**Status:** ⚪ Not Started
**Estimated Time:** 5-6 days
**Priority:** Medium (demonstrates competitive multiplayer)

---

## 🎯 **Objective**

Create a MOBA (Multiplayer Online Battle Arena) style game example that showcases team-based competitive gameplay, lanes, minions, towers, and real-time combat. This demonstrates how to build competitive multiplayer games with complex game mechanics.

**Game Concept:**
- **Genre:** Multiplayer Online Battle Arena (MOBA)
- **Teams:** 2 teams (Red vs Blue)
- **Players:** 5v5 or 3v3
- **Map:** 3 lanes with jungle areas
- **Objectives:** Destroy enemy base/nexus
- **Features:** Champions, minions, towers, items, abilities
- **Match Duration:** 20-40 minutes

---

## 📋 **Detailed Tasks**

### **1. Project Setup** (Day 1)

**Directory Structure:**
```
examples/moba/
├── Cargo.toml
├── server/
│   ├── src/
│   │   ├── main.rs
│   │   ├── game/
│   │   │   ├── mod.rs
│   │   │   ├── map.rs
│   │   │   ├── minions.rs
│   │   │   ├── towers.rs
│   │   │   └── fog_of_war.rs
│   │   └── matchmaking.rs
│   └── Cargo.toml
├── client/
│   ├── src/
│   │   ├── main.rs
│   │   ├── ui/
│   │   │   ├── mod.rs
│   │   │   ├── minimap.rs
│   │   │   ├── abilities.rs
│   │   │   └── scoreboard.rs
│   │   └── camera.rs
│   └── Cargo.toml
└── shared/
    ├── src/
    │   ├── lib.rs
    │   ├── champions.rs
    │   ├── abilities.rs
    │   ├── items.rs
    │   └── combat.rs
    └── Cargo.toml
```

---

### **2. Shared Components** (Day 1)

**File:** `examples/moba/shared/src/champions.rs`

```rust
use silmaril_core::prelude::*;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Champion (player-controlled hero)
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Champion {
    pub champion_id: u32,
    pub champion_name: String,
    pub champion_type: ChampionType,
    pub player_id: u64,
    pub team: Team,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChampionType {
    Tank,
    Fighter,
    Assassin,
    Mage,
    Marksman,
    Support,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Team {
    Red,
    Blue,
}

/// Champion stats
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChampionStats {
    // Core stats
    pub level: u32,
    pub experience: u32,

    // Combat stats
    pub health: f32,
    pub max_health: f32,
    pub mana: f32,
    pub max_mana: f32,
    pub attack_damage: f32,
    pub ability_power: f32,
    pub armor: f32,
    pub magic_resist: f32,
    pub attack_speed: f32,
    pub movement_speed: f32,

    // Regen
    pub health_regen: f32,
    pub mana_regen: f32,

    // Combat
    pub attack_range: f32,
    pub crit_chance: f32,
    pub crit_damage: f32,
}

impl ChampionStats {
    pub fn new(champion_type: ChampionType, level: u32) -> Self {
        let base = match champion_type {
            ChampionType::Tank => (800.0, 300.0, 65.0, 15.0, 60.0, 30.0, 125.0),
            ChampionType::Fighter => (650.0, 250.0, 75.0, 10.0, 50.0, 25.0, 150.0),
            ChampionType::Assassin => (550.0, 200.0, 85.0, 0.0, 30.0, 30.0, 175.0),
            ChampionType::Mage => (500.0, 400.0, 55.0, 80.0, 25.0, 30.0, 135.0),
            ChampionType::Marksman => (550.0, 250.0, 70.0, 0.0, 25.0, 25.0, 140.0),
            ChampionType::Support => (600.0, 350.0, 50.0, 50.0, 40.0, 30.0, 130.0),
        };

        let (hp, mana, ad, ap, armor, mr, ms) = base;

        // Scale with level
        let level_mult = 1.0 + (level as f32 - 1.0) * 0.1;

        Self {
            level,
            experience: 0,
            health: hp * level_mult,
            max_health: hp * level_mult,
            mana: mana,
            max_mana: mana,
            attack_damage: ad * level_mult,
            ability_power: ap * level_mult,
            armor: armor + (level as f32 * 3.0),
            magic_resist: mr + (level as f32 * 2.0),
            attack_speed: 1.0 + (level as f32 * 0.02),
            movement_speed: ms,
            health_regen: 5.0 + (level as f32 * 0.5),
            mana_regen: 3.0 + (level as f32 * 0.3),
            attack_range: match champion_type {
                ChampionType::Marksman | ChampionType::Mage => 550.0,
                _ => 175.0,
            },
            crit_chance: 0.0,
            crit_damage: 2.0,
        }
    }

    pub fn take_damage(&mut self, damage: f32, damage_type: DamageType) {
        let mitigation = match damage_type {
            DamageType::Physical => self.armor / (self.armor + 100.0),
            DamageType::Magical => self.magic_resist / (self.magic_resist + 100.0),
            DamageType::True => 0.0,
        };

        let actual_damage = damage * (1.0 - mitigation);
        self.health = (self.health - actual_damage).max(0.0);
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn can_level_up(&self) -> bool {
        self.level < 18 && self.experience >= self.xp_for_next_level()
    }

    pub fn xp_for_next_level(&self) -> u32 {
        100 + (self.level * 50)
    }

    pub fn level_up(&mut self) {
        if self.can_level_up() {
            self.level += 1;
            *self = Self::new(
                ChampionType::Fighter, // Would need to store type
                self.level
            );
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Physical,
    Magical,
    True,
}

/// Abilities
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct AbilitySet {
    pub q: Ability,
    pub w: Ability,
    pub e: Ability,
    pub r: Ability, // Ultimate
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub ability_id: u32,
    pub name: String,
    pub description: String,
    pub ability_type: AbilityType,
    pub cooldown: f32,
    pub current_cooldown: f32,
    pub mana_cost: f32,
    pub damage: f32,
    pub damage_type: DamageType,
    pub range: f32,
    pub area_of_effect: f32,
    pub cast_time: f32,
    pub level: u32,
    pub max_level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityType {
    Skillshot,      // Needs aiming
    Targeted,       // Locks on target
    AreaOfEffect,   // Ground-targeted
    SelfBuff,       // Self-cast
    Passive,        // Always active
}

impl Ability {
    pub fn can_cast(&self, stats: &ChampionStats) -> bool {
        self.current_cooldown <= 0.0 && stats.mana >= self.mana_cost
    }

    pub fn cast(&mut self, stats: &mut ChampionStats) {
        stats.mana -= self.mana_cost;
        self.current_cooldown = self.cooldown;
    }

    pub fn tick(&mut self, dt: f32) {
        if self.current_cooldown > 0.0 {
            self.current_cooldown = (self.current_cooldown - dt).max(0.0);
        }
    }
}

/// Items
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<Option<Item>>,
    pub gold: u32,
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            items: vec![None; 6], // 6 item slots
            gold: 500, // Starting gold
        }
    }

    pub fn can_buy(&self, item: &Item) -> bool {
        self.gold >= item.cost && self.items.iter().any(|slot| slot.is_none())
    }

    pub fn buy_item(&mut self, item: Item) -> bool {
        if !self.can_buy(&item) {
            return false;
        }

        for slot in &mut self.items {
            if slot.is_none() {
                *slot = Some(item.clone());
                self.gold -= item.cost;
                return true;
            }
        }

        false
    }

    pub fn total_stats(&self) -> ItemStats {
        let mut total = ItemStats::default();

        for slot in &self.items {
            if let Some(item) = slot {
                total = total + item.stats;
            }
        }

        total
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub item_id: u32,
    pub name: String,
    pub description: String,
    pub cost: u32,
    pub stats: ItemStats,
    pub passive: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ItemStats {
    pub health: f32,
    pub mana: f32,
    pub attack_damage: f32,
    pub ability_power: f32,
    pub armor: f32,
    pub magic_resist: f32,
    pub attack_speed: f32,
    pub movement_speed: f32,
    pub crit_chance: f32,
}

impl std::ops::Add for ItemStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            health: self.health + other.health,
            mana: self.mana + other.mana,
            attack_damage: self.attack_damage + other.attack_damage,
            ability_power: self.ability_power + other.ability_power,
            armor: self.armor + other.armor,
            magic_resist: self.magic_resist + other.magic_resist,
            attack_speed: self.attack_speed + other.attack_speed,
            movement_speed: self.movement_speed + other.movement_speed,
            crit_chance: self.crit_chance + other.crit_chance,
        }
    }
}

/// Minion (AI-controlled lane units)
#[derive(Component, Debug, Clone)]
pub struct Minion {
    pub team: Team,
    pub minion_type: MinionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinionType {
    Melee,
    Caster,
    Siege,
    Super,
}

/// Tower (defensive structure)
#[derive(Component, Debug, Clone)]
pub struct Tower {
    pub team: Team,
    pub lane: Lane,
    pub tier: u32, // 1, 2, 3, Inhibitor, Nexus
    pub health: f32,
    pub max_health: f32,
    pub attack_damage: f32,
    pub attack_range: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    Top,
    Middle,
    Bottom,
}

/// Combat stats
#[derive(Component, Debug, Clone, Copy)]
pub struct CombatState {
    pub is_attacking: bool,
    pub attack_cooldown: f32,
    pub target: Option<Entity>,
    pub last_damage_time: f32,
}
```

---

### **3. Map System** (Day 2)

**File:** `examples/moba/server/src/game/map.rs`

```rust
use glam::Vec3;
use moba_shared::champions::*;

/// MOBA map
pub struct MobaMap {
    width: f32,
    height: f32,
    lanes: Vec<LaneInfo>,
    spawn_points: SpawnPoints,
    jungle_camps: Vec<JungleCamp>,
}

#[derive(Debug, Clone)]
pub struct LaneInfo {
    pub lane: Lane,
    pub waypoints: Vec<Vec3>,
    pub towers_red: Vec<Vec3>,
    pub towers_blue: Vec<Vec3>,
}

#[derive(Debug, Clone)]
pub struct SpawnPoints {
    pub red_fountain: Vec3,
    pub blue_fountain: Vec3,
}

#[derive(Debug, Clone)]
pub struct JungleCamp {
    pub position: Vec3,
    pub camp_type: JungleCampType,
    pub spawn_time: f32,
    pub current_timer: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JungleCampType {
    BlueBuff,
    RedBuff,
    Dragon,
    Baron,
    Gromp,
    Wolves,
    Raptors,
    Krugs,
}

impl MobaMap {
    pub fn new() -> Self {
        let width = 15000.0;
        let height = 15000.0;

        let lanes = vec![
            Self::create_lane(Lane::Top),
            Self::create_lane(Lane::Middle),
            Self::create_lane(Lane::Bottom),
        ];

        let spawn_points = SpawnPoints {
            red_fountain: Vec3::new(14500.0, 0.0, 14500.0),
            blue_fountain: Vec3::new(500.0, 0.0, 500.0),
        };

        let jungle_camps = vec![
            JungleCamp {
                position: Vec3::new(3800.0, 0.0, 8000.0),
                camp_type: JungleCampType::BlueBuff,
                spawn_time: 90.0,
                current_timer: 0.0,
            },
            JungleCamp {
                position: Vec3::new(11200.0, 0.0, 7000.0),
                camp_type: JungleCampType::RedBuff,
                spawn_time: 90.0,
                current_timer: 0.0,
            },
            JungleCamp {
                position: Vec3::new(9800.0, 0.0, 4500.0),
                camp_type: JungleCampType::Dragon,
                spawn_time: 300.0,
                current_timer: 0.0,
            },
            JungleCamp {
                position: Vec3::new(5200.0, 0.0, 10500.0),
                camp_type: JungleCampType::Baron,
                spawn_time: 1200.0, // 20 minutes
                current_timer: 0.0,
            },
        ];

        Self {
            width,
            height,
            lanes,
            spawn_points,
            jungle_camps,
        }
    }

    fn create_lane(lane: Lane) -> LaneInfo {
        let waypoints = match lane {
            Lane::Top => vec![
                Vec3::new(1000.0, 0.0, 1000.0),
                Vec3::new(1000.0, 0.0, 7500.0),
                Vec3::new(7500.0, 0.0, 14000.0),
                Vec3::new(14000.0, 0.0, 14000.0),
            ],
            Lane::Middle => vec![
                Vec3::new(1000.0, 0.0, 1000.0),
                Vec3::new(7500.0, 0.0, 7500.0),
                Vec3::new(14000.0, 0.0, 14000.0),
            ],
            Lane::Bottom => vec![
                Vec3::new(1000.0, 0.0, 1000.0),
                Vec3::new(7500.0, 0.0, 1000.0),
                Vec3::new(14000.0, 0.0, 7500.0),
                Vec3::new(14000.0, 0.0, 14000.0),
            ],
        };

        let towers_blue = match lane {
            Lane::Top => vec![
                Vec3::new(1500.0, 0.0, 6000.0),
                Vec3::new(1500.0, 0.0, 10000.0),
                Vec3::new(1500.0, 0.0, 13000.0),
            ],
            Lane::Middle => vec![
                Vec3::new(3000.0, 0.0, 3000.0),
                Vec3::new(6000.0, 0.0, 6000.0),
                Vec3::new(9000.0, 0.0, 9000.0),
            ],
            Lane::Bottom => vec![
                Vec3::new(6000.0, 0.0, 1500.0),
                Vec3::new(10000.0, 0.0, 1500.0),
                Vec3::new(13000.0, 0.0, 1500.0),
            ],
        };

        let towers_red = towers_blue.iter()
            .map(|&pos| Vec3::new(15000.0 - pos.x, pos.y, 15000.0 - pos.z))
            .collect();

        LaneInfo {
            lane,
            waypoints,
            towers_red,
            towers_blue,
        }
    }

    pub fn get_spawn_point(&self, team: Team) -> Vec3 {
        match team {
            Team::Red => self.spawn_points.red_fountain,
            Team::Blue => self.spawn_points.blue_fountain,
        }
    }

    pub fn update_jungle(&mut self, dt: f32) {
        for camp in &mut self.jungle_camps {
            if camp.current_timer > 0.0 {
                camp.current_timer -= dt;
            }
        }
    }
}
```

---

### **4. Minion Wave System** (Day 2-3)

**File:** `examples/moba/server/src/game/minions.rs`

```rust
use silmaril_core::prelude::*;
use moba_shared::champions::*;
use glam::Vec3;

pub struct MinionSpawner {
    spawn_interval: f32,
    spawn_timer: f32,
    wave_number: u32,
}

impl MinionSpawner {
    pub fn new() -> Self {
        Self {
            spawn_interval: 30.0, // Spawn every 30 seconds
            spawn_timer: 0.0,
            wave_number: 0,
        }
    }

    pub fn update(&mut self, world: &mut World, map: &MobaMap, dt: f32) {
        self.spawn_timer += dt;

        if self.spawn_timer >= self.spawn_interval {
            self.spawn_timer = 0.0;
            self.wave_number += 1;
            self.spawn_wave(world, map);
        }
    }

    fn spawn_wave(&self, world: &mut World, map: &MobaMap) {
        for lane_info in &map.lanes {
            // Spawn for both teams
            self.spawn_lane_wave(world, lane_info, Team::Red);
            self.spawn_lane_wave(world, lane_info, Team::Blue);
        }

        tracing::info!("Spawned minion wave {}", self.wave_number);
    }

    fn spawn_lane_wave(&self, world: &mut World, lane: &LaneInfo, team: Team) {
        let spawn_pos = match team {
            Team::Blue => lane.waypoints[0],
            Team::Red => lane.waypoints[lane.waypoints.len() - 1],
        };

        // Spawn 3 melee minions
        for i in 0..3 {
            let offset = Vec3::new(i as f32 * 50.0, 0.0, 0.0);
            self.spawn_minion(world, spawn_pos + offset, MinionType::Melee, team, lane.lane);
        }

        // Spawn 3 caster minions
        for i in 0..3 {
            let offset = Vec3::new(i as f32 * 50.0, 0.0, 100.0);
            self.spawn_minion(world, spawn_pos + offset, MinionType::Caster, team, lane.lane);
        }

        // Every 3rd wave, spawn siege minion
        if self.wave_number % 3 == 0 {
            let offset = Vec3::new(0.0, 0.0, 200.0);
            self.spawn_minion(world, spawn_pos + offset, MinionType::Siege, team, lane.lane);
        }
    }

    fn spawn_minion(
        &self,
        world: &mut World,
        position: Vec3,
        minion_type: MinionType,
        team: Team,
        lane: Lane,
    ) -> Entity {
        let entity = world.spawn();

        world.add(entity, Minion { team, minion_type });

        let stats = match minion_type {
            MinionType::Melee => ChampionStats {
                health: 400.0,
                max_health: 400.0,
                attack_damage: 20.0,
                armor: 10.0,
                attack_range: 100.0,
                movement_speed: 325.0,
                ..Default::default()
            },
            MinionType::Caster => ChampionStats {
                health: 250.0,
                max_health: 250.0,
                attack_damage: 25.0,
                armor: 0.0,
                attack_range: 550.0,
                movement_speed: 325.0,
                ..Default::default()
            },
            MinionType::Siege => ChampionStats {
                health: 800.0,
                max_health: 800.0,
                attack_damage: 40.0,
                armor: 15.0,
                attack_range: 300.0,
                movement_speed: 300.0,
                ..Default::default()
            },
            MinionType::Super => ChampionStats {
                health: 1500.0,
                max_health: 1500.0,
                attack_damage: 100.0,
                armor: 30.0,
                attack_range: 200.0,
                movement_speed: 350.0,
                ..Default::default()
            },
        };

        world.add(entity, stats);
        world.add(entity, Transform { position, ..Default::default() });
        world.add(entity, CombatState::default());

        entity
    }
}

/// Minion AI - follow lane and attack enemies
pub fn minion_ai_system(world: &mut World, map: &MobaMap, dt: f32) {
    for (entity, (minion, transform, stats, mut combat)) in
        world.query::<(&Minion, &mut Transform, &ChampionStats, &mut CombatState)>()
    {
        if !stats.is_alive() {
            continue;
        }

        // Find targets in range
        let target = find_nearest_enemy(world, transform.position, minion.team, 1000.0);

        if let Some(target_entity) = target {
            combat.target = Some(target_entity);

            // Move toward target if out of attack range
            if let Some(target_transform) = world.get::<Transform>(target_entity) {
                let distance = transform.position.distance(target_transform.position);

                if distance > stats.attack_range {
                    // Move toward target
                    let direction = (target_transform.position - transform.position).normalize();
                    transform.position += direction * stats.movement_speed * dt;
                } else {
                    // Attack
                    if combat.attack_cooldown <= 0.0 {
                        attack(world, entity, target_entity);
                        combat.attack_cooldown = 1.0 / stats.attack_speed;
                    }
                }
            }
        } else {
            // No target, follow lane
            // (Simplified - would follow waypoints)
            combat.target = None;
        }

        // Update cooldowns
        if combat.attack_cooldown > 0.0 {
            combat.attack_cooldown -= dt;
        }
    }
}

fn find_nearest_enemy(
    world: &World,
    position: Vec3,
    team: Team,
    max_range: f32,
) -> Option<Entity> {
    let mut nearest: Option<(Entity, f32)> = None;

    // Check champions
    for (entity, (champion, transform, stats)) in
        world.query::<(&Champion, &Transform, &ChampionStats)>()
    {
        if champion.team != team && stats.is_alive() {
            let distance = position.distance(transform.position);
            if distance < max_range {
                if nearest.is_none() || distance < nearest.unwrap().1 {
                    nearest = Some((entity, distance));
                }
            }
        }
    }

    // Check minions
    for (entity, (minion, transform, stats)) in
        world.query::<(&Minion, &Transform, &ChampionStats)>()
    {
        if minion.team != team && stats.is_alive() {
            let distance = position.distance(transform.position);
            if distance < max_range {
                if nearest.is_none() || distance < nearest.unwrap().1 {
                    nearest = Some((entity, distance));
                }
            }
        }
    }

    nearest.map(|(entity, _)| entity)
}

fn attack(world: &mut World, attacker: Entity, target: Entity) {
    if let Some(attacker_stats) = world.get::<ChampionStats>(attacker) {
        let damage = attacker_stats.attack_damage;

        if let Some(mut target_stats) = world.get_mut::<ChampionStats>(target) {
            target_stats.take_damage(damage, DamageType::Physical);

            tracing::debug!(
                "Entity {:?} attacked {:?} for {} damage",
                attacker,
                target,
                damage
            );
        }
    }
}
```

---

### **5. Client UI** (Day 4-5)

**File:** `examples/moba/client/src/ui/abilities.rs`

```rust
use silmaril_rendering::ui::*;
use moba_shared::champions::*;

pub struct AbilityBar {
    q_button: Button,
    w_button: Button,
    e_button: Button,
    r_button: Button,
}

impl AbilityBar {
    pub fn new() -> Self {
        Self {
            q_button: Button::new(10.0, 650.0, 64.0, 64.0, "Q"),
            w_button: Button::new(84.0, 650.0, 64.0, 64.0, "W"),
            e_button: Button::new(158.0, 650.0, 64.0, 64.0, "E"),
            r_button: Button::new(232.0, 650.0, 64.0, 64.0, "R"),
        }
    }

    pub fn render(&self, renderer: &mut Renderer, abilities: &AbilitySet, stats: &ChampionStats) {
        self.render_ability(renderer, &abilities.q, &self.q_button, stats);
        self.render_ability(renderer, &abilities.w, &self.w_button, stats);
        self.render_ability(renderer, &abilities.e, &self.e_button, stats);
        self.render_ability(renderer, &abilities.r, &self.r_button, stats);
    }

    fn render_ability(
        &self,
        renderer: &mut Renderer,
        ability: &Ability,
        button: &Button,
        stats: &ChampionStats,
    ) {
        // Draw button background
        let color = if ability.can_cast(stats) {
            [0.2, 0.6, 1.0, 1.0]
        } else {
            [0.3, 0.3, 0.3, 1.0]
        };

        renderer.draw_rect(button.x, button.y, button.width, button.height, color);

        // Draw cooldown overlay
        if ability.current_cooldown > 0.0 {
            let cooldown_percent = ability.current_cooldown / ability.cooldown;
            let overlay_height = button.height * cooldown_percent;

            renderer.draw_rect(
                button.x,
                button.y,
                button.width,
                overlay_height,
                [0.0, 0.0, 0.0, 0.7],
            );

            // Draw cooldown number
            renderer.draw_text(
                &format!("{:.1}", ability.current_cooldown),
                button.x + button.width / 2.0,
                button.y + button.height / 2.0,
            );
        }

        // Draw hotkey
        renderer.draw_text(
            &button.label,
            button.x + button.width / 2.0,
            button.y + button.height + 10.0,
        );
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Server supports 10+ concurrent players
- [ ] Champions have unique stats and abilities
- [ ] Minions spawn in waves and push lanes
- [ ] Towers defend lanes and deal damage
- [ ] Combat system with damage types
- [ ] Items provide stat bonuses
- [ ] Experience and leveling system
- [ ] Fog of war implemented
- [ ] Minimap shows game state
- [ ] Ability UI displays cooldowns
- [ ] Match ends when nexus destroyed
- [ ] Performance: 60 FPS with 100+ entities
- [ ] Network latency < 100ms

---

## 🎯 **Performance Targets**

| Metric | Target | Critical |
|--------|--------|----------|
| Server tick rate | 60 TPS | 30 TPS |
| Concurrent players | 10+ | 6+ |
| Entity count | 200+ | 100+ |
| Network latency | < 50ms | < 100ms |
| Client FPS | 60 FPS | 30 FPS |
| Match duration | 20-40 min | 10-60 min |

---

**Dependencies:** Phase 2 (Networking), Phase 3 (Rendering)
**Next:** [phase5-mdbook.md](phase5-mdbook.md)
