# Phase 5.3: Turn-Based Strategy Example

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** Medium (demonstrates tactical gameplay)

---

## 🎯 **Objective**

Create a turn-based strategy game example that showcases tactical gameplay, grid-based movement, unit abilities, and AI opponents. This demonstrates how the engine handles discrete turn-based mechanics and strategic gameplay.

**Game Concept:**
- **Genre:** Turn-based tactical strategy
- **Grid:** Hexagonal or square tile grid
- **Units:** Different unit types with unique abilities
- **Combat:** Action points, range, line of sight
- **AI:** Minimax or behavior tree AI opponents
- **Win Condition:** Eliminate all enemy units or capture objectives

---

## 📋 **Detailed Tasks**

### **1. Project Setup** (Day 1 Morning)

**File:** `examples/turnbased/Cargo.toml`

```toml
[package]
name = "turnbased-strategy"
version = "0.1.0"
edition = "2021"

[dependencies]
agent-game-engine-core = { path = "../../engine/core" }
agent-game-engine-macros = { path = "../../engine/macros" }
agent-game-engine-platform = { path = "../../engine/platform" }
agent-game-engine-rendering = { path = "../../engine/rendering" }
glam = "0.24"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
rand = "0.8"
pathfinding = "4.0"

[dev-dependencies]
criterion = "0.5"
```

**Directory Structure:**
```
examples/turnbased/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── components.rs
│   ├── grid.rs
│   ├── units.rs
│   ├── abilities.rs
│   ├── ai/
│   │   ├── mod.rs
│   │   ├── minimax.rs
│   │   └── behavior.rs
│   ├── systems/
│   │   ├── mod.rs
│   │   ├── turn.rs
│   │   ├── combat.rs
│   │   └── pathfinding.rs
│   └── ui/
│       ├── mod.rs
│       ├── grid_renderer.rs
│       └── hud.rs
└── README.md
```

---

### **2. Grid System** (Day 1)

**File:** `examples/turnbased/src/grid.rs`

```rust
use glam::{IVec2, Vec2, Vec3};
use std::collections::{HashMap, HashSet};

/// Grid type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridType {
    Square,
    Hexagonal,
}

/// Tile grid for tactical gameplay
#[derive(Debug, Clone)]
pub struct Grid {
    grid_type: GridType,
    width: i32,
    height: i32,
    tiles: HashMap<IVec2, Tile>,
}

#[derive(Debug, Clone)]
pub struct Tile {
    pub position: IVec2,
    pub tile_type: TileType,
    pub height: i32,
    pub occupant: Option<Entity>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    Ground,
    Water,
    Mountain,
    Forest,
    Wall,
}

impl Grid {
    pub fn new(grid_type: GridType, width: i32, height: i32) -> Self {
        let mut tiles = HashMap::new();

        for y in 0..height {
            for x in 0..width {
                let pos = IVec2::new(x, y);
                tiles.insert(pos, Tile {
                    position: pos,
                    tile_type: TileType::Ground,
                    height: 0,
                    occupant: None,
                });
            }
        }

        Self {
            grid_type,
            width,
            height,
            tiles,
        }
    }

    /// Get tile at position
    pub fn get_tile(&self, pos: IVec2) -> Option<&Tile> {
        self.tiles.get(&pos)
    }

    /// Get mutable tile
    pub fn get_tile_mut(&mut self, pos: IVec2) -> Option<&mut Tile> {
        self.tiles.get_mut(&pos)
    }

    /// Check if position is valid
    pub fn is_valid(&self, pos: IVec2) -> bool {
        pos.x >= 0 && pos.x < self.width && pos.y >= 0 && pos.y < self.height
    }

    /// Check if tile is walkable
    pub fn is_walkable(&self, pos: IVec2) -> bool {
        if let Some(tile) = self.get_tile(pos) {
            match tile.tile_type {
                TileType::Ground | TileType::Forest => tile.occupant.is_none(),
                _ => false,
            }
        } else {
            false
        }
    }

    /// Get neighbors (based on grid type)
    pub fn get_neighbors(&self, pos: IVec2) -> Vec<IVec2> {
        match self.grid_type {
            GridType::Square => self.get_square_neighbors(pos),
            GridType::Hexagonal => self.get_hex_neighbors(pos),
        }
    }

    fn get_square_neighbors(&self, pos: IVec2) -> Vec<IVec2> {
        let directions = [
            IVec2::new(-1, 0),  // Left
            IVec2::new(1, 0),   // Right
            IVec2::new(0, -1),  // Up
            IVec2::new(0, 1),   // Down
            // Diagonals (optional)
            IVec2::new(-1, -1),
            IVec2::new(1, -1),
            IVec2::new(-1, 1),
            IVec2::new(1, 1),
        ];

        directions
            .iter()
            .map(|&dir| pos + dir)
            .filter(|&p| self.is_valid(p))
            .collect()
    }

    fn get_hex_neighbors(&self, pos: IVec2) -> Vec<IVec2> {
        // Axial coordinates for hexagonal grid
        let directions = if pos.y % 2 == 0 {
            // Even row
            vec![
                IVec2::new(-1, 0),
                IVec2::new(1, 0),
                IVec2::new(-1, -1),
                IVec2::new(0, -1),
                IVec2::new(-1, 1),
                IVec2::new(0, 1),
            ]
        } else {
            // Odd row
            vec![
                IVec2::new(-1, 0),
                IVec2::new(1, 0),
                IVec2::new(0, -1),
                IVec2::new(1, -1),
                IVec2::new(0, 1),
                IVec2::new(1, 1),
            ]
        };

        directions
            .iter()
            .map(|&dir| pos + dir)
            .filter(|&p| self.is_valid(p))
            .collect()
    }

    /// Convert grid position to world position
    pub fn grid_to_world(&self, pos: IVec2) -> Vec3 {
        match self.grid_type {
            GridType::Square => {
                Vec3::new(pos.x as f32, 0.0, pos.y as f32)
            }
            GridType::Hexagonal => {
                // Hexagonal grid layout
                let x = pos.x as f32 * 0.75;
                let z = pos.y as f32 + (pos.x % 2) as f32 * 0.5;
                Vec3::new(x, 0.0, z)
            }
        }
    }

    /// Convert world position to grid position
    pub fn world_to_grid(&self, world_pos: Vec3) -> IVec2 {
        match self.grid_type {
            GridType::Square => {
                IVec2::new(world_pos.x.round() as i32, world_pos.z.round() as i32)
            }
            GridType::Hexagonal => {
                // Reverse hexagonal conversion
                let x = (world_pos.x / 0.75).round() as i32;
                let z = (world_pos.z - (x % 2) as f32 * 0.5).round() as i32;
                IVec2::new(x, z)
            }
        }
    }

    /// Find path between two positions (A* pathfinding)
    pub fn find_path(&self, start: IVec2, goal: IVec2) -> Option<Vec<IVec2>> {
        use pathfinding::prelude::astar;

        let result = astar(
            &start,
            |&pos| {
                self.get_neighbors(pos)
                    .into_iter()
                    .filter(|&n| self.is_walkable(n))
                    .map(|n| (n, 1)) // Cost = 1 per tile
                    .collect::<Vec<_>>()
            },
            |&pos| {
                // Heuristic: Manhattan distance
                ((pos.x - goal.x).abs() + (pos.y - goal.y).abs()) as u32
            },
            |&pos| pos == goal,
        );

        result.map(|(path, _cost)| path)
    }

    /// Get tiles in range
    pub fn get_tiles_in_range(&self, center: IVec2, range: i32) -> Vec<IVec2> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![(center, 0)];

        while let Some((pos, dist)) = queue.pop() {
            if visited.contains(&pos) || dist > range {
                continue;
            }

            visited.insert(pos);
            result.push(pos);

            for neighbor in self.get_neighbors(pos) {
                if !visited.contains(&neighbor) {
                    queue.push((neighbor, dist + 1));
                }
            }
        }

        result
    }

    /// Check line of sight
    pub fn has_line_of_sight(&self, from: IVec2, to: IVec2) -> bool {
        // Bresenham's line algorithm
        let dx = (to.x - from.x).abs();
        let dy = (to.y - from.y).abs();
        let sx = if from.x < to.x { 1 } else { -1 };
        let sy = if from.y < to.y { 1 } else { -1 };
        let mut err = dx - dy;

        let mut current = from;

        loop {
            // Check if tile blocks vision
            if let Some(tile) = self.get_tile(current) {
                match tile.tile_type {
                    TileType::Wall | TileType::Mountain => return false,
                    _ => {}
                }
            }

            if current == to {
                return true;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                current.x += sx;
            }
            if e2 < dx {
                err += dx;
                current.y += sy;
            }
        }
    }
}

impl Tile {
    /// Get movement cost for this tile
    pub fn movement_cost(&self) -> i32 {
        match self.tile_type {
            TileType::Ground => 1,
            TileType::Forest => 2,
            TileType::Water => 3,
            TileType::Mountain | TileType::Wall => i32::MAX, // Impassable
        }
    }

    /// Get defense bonus
    pub fn defense_bonus(&self) -> f32 {
        match self.tile_type {
            TileType::Forest => 0.2,
            TileType::Mountain => 0.5,
            _ => 0.0,
        }
    }
}
```

---

### **3. Unit Components** (Day 1-2)

**File:** `examples/turnbased/src/components.rs`

```rust
use agent_game_engine_core::prelude::*;
use glam::IVec2;

/// Tactical unit
#[derive(Component, Debug, Clone)]
pub struct Unit {
    pub unit_name: String,
    pub unit_type: UnitType,
    pub team: Team,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitType {
    Infantry,
    Archer,
    Cavalry,
    Mage,
    Tank,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
}

/// Unit stats
#[derive(Component, Debug, Clone, Copy)]
pub struct UnitStats {
    pub max_health: i32,
    pub current_health: i32,
    pub attack: i32,
    pub defense: i32,
    pub movement: i32,
    pub attack_range: i32,
}

impl UnitStats {
    pub fn new(unit_type: UnitType) -> Self {
        match unit_type {
            UnitType::Infantry => Self {
                max_health: 100,
                current_health: 100,
                attack: 20,
                defense: 15,
                movement: 3,
                attack_range: 1,
            },
            UnitType::Archer => Self {
                max_health: 60,
                current_health: 60,
                attack: 25,
                defense: 5,
                movement: 3,
                attack_range: 4,
            },
            UnitType::Cavalry => Self {
                max_health: 80,
                current_health: 80,
                attack: 30,
                defense: 10,
                movement: 5,
                attack_range: 1,
            },
            UnitType::Mage => Self {
                max_health: 50,
                current_health: 50,
                attack: 40,
                defense: 3,
                movement: 2,
                attack_range: 3,
            },
            UnitType::Tank => Self {
                max_health: 150,
                current_health: 150,
                attack: 15,
                defense: 30,
                movement: 2,
                attack_range: 1,
            },
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current_health > 0
    }

    pub fn take_damage(&mut self, damage: i32) {
        self.current_health = (self.current_health - damage).max(0);
    }

    pub fn heal(&mut self, amount: i32) {
        self.current_health = (self.current_health + amount).min(self.max_health);
    }
}

/// Grid position
#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
    pub position: IVec2,
}

/// Action points for turn-based gameplay
#[derive(Component, Debug, Clone, Copy)]
pub struct ActionPoints {
    pub current: i32,
    pub max: i32,
}

impl ActionPoints {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }

    pub fn reset(&mut self) {
        self.current = self.max;
    }

    pub fn spend(&mut self, amount: i32) -> bool {
        if self.current >= amount {
            self.current -= amount;
            true
        } else {
            false
        }
    }

    pub fn has_points(&self) -> bool {
        self.current > 0
    }
}

/// Unit abilities
#[derive(Component, Debug, Clone)]
pub struct Abilities {
    pub abilities: Vec<Ability>,
}

#[derive(Debug, Clone)]
pub struct Ability {
    pub ability_id: u32,
    pub name: String,
    pub ability_type: AbilityType,
    pub cooldown: i32,
    pub current_cooldown: i32,
    pub action_cost: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbilityType {
    Attack,
    Heal,
    Buff,
    Debuff,
    Move,
}

impl Ability {
    pub fn is_ready(&self) -> bool {
        self.current_cooldown == 0
    }

    pub fn use_ability(&mut self) {
        self.current_cooldown = self.cooldown;
    }

    pub fn tick_cooldown(&mut self) {
        if self.current_cooldown > 0 {
            self.current_cooldown -= 1;
        }
    }
}

/// AI controller
#[derive(Component, Debug, Clone)]
pub struct AiController {
    pub ai_type: AiType,
    pub difficulty: AiDifficulty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiType {
    Aggressive,
    Defensive,
    Balanced,
    Support,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiDifficulty {
    Easy,
    Medium,
    Hard,
}

/// Visual representation
#[derive(Component, Debug, Clone)]
pub struct UnitSprite {
    pub texture: String,
    pub color: [f32; 4],
}
```

---

### **4. Turn System** (Day 2)

**File:** `examples/turnbased/src/systems/turn.rs`

```rust
use crate::components::*;
use agent_game_engine_core::prelude::*;

/// Game phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GamePhase {
    PlayerTurn,
    EnemyTurn,
    Victory,
    Defeat,
}

/// Turn manager
pub struct TurnManager {
    current_phase: GamePhase,
    turn_number: u32,
    active_unit: Option<Entity>,
}

impl TurnManager {
    pub fn new() -> Self {
        Self {
            current_phase: GamePhase::PlayerTurn,
            turn_number: 1,
            active_unit: None,
        }
    }

    pub fn current_phase(&self) -> GamePhase {
        self.current_phase
    }

    pub fn turn_number(&self) -> u32 {
        self.turn_number
    }

    pub fn active_unit(&self) -> Option<Entity> {
        self.active_unit
    }

    pub fn start_turn(&mut self, world: &mut World) {
        tracing::info!("Turn {} - {:?}", self.turn_number, self.current_phase);

        // Reset action points for all units of current team
        let team = match self.current_phase {
            GamePhase::PlayerTurn => Team::Player,
            GamePhase::EnemyTurn => Team::Enemy,
            _ => return,
        };

        for (entity, (unit, mut action_points, abilities)) in
            world.query::<(&Unit, &mut ActionPoints, &mut Abilities)>()
        {
            if unit.team == team {
                action_points.reset();

                // Tick cooldowns
                for ability in &mut abilities.abilities {
                    ability.tick_cooldown();
                }
            }
        }

        // Find first unit with actions
        self.active_unit = self.find_next_unit(world);
    }

    pub fn end_turn(&mut self, world: &mut World) {
        // Switch phase
        self.current_phase = match self.current_phase {
            GamePhase::PlayerTurn => {
                GamePhase::EnemyTurn
            }
            GamePhase::EnemyTurn => {
                self.turn_number += 1;
                GamePhase::PlayerTurn
            }
            phase => phase,
        };

        self.start_turn(world);

        // Check win/lose conditions
        self.check_victory_conditions(world);
    }

    pub fn next_unit(&mut self, world: &mut World) {
        self.active_unit = self.find_next_unit(world);

        if self.active_unit.is_none() {
            // No more units with actions, end turn
            self.end_turn(world);
        }
    }

    fn find_next_unit(&self, world: &World) -> Option<Entity> {
        let team = match self.current_phase {
            GamePhase::PlayerTurn => Team::Player,
            GamePhase::EnemyTurn => Team::Enemy,
            _ => return None,
        };

        // Find first unit with action points
        for (entity, (unit, action_points, stats)) in
            world.query::<(&Unit, &ActionPoints, &UnitStats)>()
        {
            if unit.team == team && action_points.has_points() && stats.is_alive() {
                return Some(entity);
            }
        }

        None
    }

    fn check_victory_conditions(&mut self, world: &World) {
        let mut player_alive = false;
        let mut enemy_alive = false;

        for (_, (unit, stats)) in world.query::<(&Unit, &UnitStats)>() {
            if !stats.is_alive() {
                continue;
            }

            match unit.team {
                Team::Player => player_alive = true,
                Team::Enemy => enemy_alive = true,
                _ => {}
            }
        }

        if !player_alive {
            self.current_phase = GamePhase::Defeat;
            tracing::info!("Defeat! All player units eliminated");
        } else if !enemy_alive {
            self.current_phase = GamePhase::Victory;
            tracing::info!("Victory! All enemy units eliminated");
        }
    }
}

/// Execute unit action
pub fn execute_action(
    world: &mut World,
    unit_entity: Entity,
    action: UnitAction,
) -> ActionResult {
    let mut action_points = world.get_mut::<ActionPoints>(unit_entity)
        .expect("Unit has no action points");

    // Check if unit has enough action points
    let cost = action.cost();
    if !action_points.spend(cost) {
        return ActionResult::InsufficientActionPoints;
    }

    match action {
        UnitAction::Move { path } => {
            execute_move(world, unit_entity, path)
        }
        UnitAction::Attack { target } => {
            execute_attack(world, unit_entity, target)
        }
        UnitAction::UseAbility { ability_id, target } => {
            execute_ability(world, unit_entity, ability_id, target)
        }
        UnitAction::Wait => {
            ActionResult::Success
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnitAction {
    Move { path: Vec<IVec2> },
    Attack { target: Entity },
    UseAbility { ability_id: u32, target: Entity },
    Wait,
}

impl UnitAction {
    fn cost(&self) -> i32 {
        match self {
            UnitAction::Move { path } => path.len() as i32,
            UnitAction::Attack { .. } => 1,
            UnitAction::UseAbility { .. } => 2,
            UnitAction::Wait => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionResult {
    Success,
    Failure,
    InsufficientActionPoints,
    InvalidTarget,
    OutOfRange,
}

fn execute_move(world: &mut World, unit: Entity, path: Vec<IVec2>) -> ActionResult {
    if path.is_empty() {
        return ActionResult::Failure;
    }

    let final_pos = path[path.len() - 1];

    if let Some(mut grid_pos) = world.get_mut::<GridPosition>(unit) {
        grid_pos.position = final_pos;
        tracing::debug!("Unit {:?} moved to {:?}", unit, final_pos);
        ActionResult::Success
    } else {
        ActionResult::Failure
    }
}

fn execute_attack(world: &mut World, attacker: Entity, defender: Entity) -> ActionResult {
    // Get attacker stats
    let attacker_stats = world.get::<UnitStats>(attacker)
        .expect("Attacker has no stats");
    let attack_value = attacker_stats.attack;

    // Get defender stats
    let mut defender_stats = world.get_mut::<UnitStats>(defender)
        .expect("Defender has no stats");

    // Calculate damage
    let damage = (attack_value - defender_stats.defense).max(1);
    defender_stats.take_damage(damage);

    tracing::info!(
        "Unit {:?} attacked {:?} for {} damage (health: {}/{})",
        attacker,
        defender,
        damage,
        defender_stats.current_health,
        defender_stats.max_health
    );

    ActionResult::Success
}

fn execute_ability(
    world: &mut World,
    caster: Entity,
    ability_id: u32,
    target: Entity,
) -> ActionResult {
    // Find and use ability
    if let Some(mut abilities) = world.get_mut::<Abilities>(caster) {
        if let Some(ability) = abilities.abilities.iter_mut()
            .find(|a| a.ability_id == ability_id)
        {
            if !ability.is_ready() {
                return ActionResult::Failure;
            }

            ability.use_ability();

            // Apply ability effect (simplified)
            match ability.ability_type {
                AbilityType::Heal => {
                    if let Some(mut stats) = world.get_mut::<UnitStats>(target) {
                        stats.heal(30);
                        tracing::info!("Unit {:?} healed {:?}", caster, target);
                    }
                }
                AbilityType::Attack => {
                    // Special attack
                    execute_attack(world, caster, target)
                }
                _ => {}
            }

            return ActionResult::Success;
        }
    }

    ActionResult::Failure
}
```

---

### **5. AI System** (Day 3-4)

**File:** `examples/turnbased/src/ai/minimax.rs`

```rust
use crate::components::*;
use crate::grid::Grid;
use crate::systems::turn::*;
use agent_game_engine_core::prelude::*;
use glam::IVec2;

/// Minimax AI for tactical decisions
pub struct MinimaxAi {
    max_depth: i32,
}

impl MinimaxAi {
    pub fn new(difficulty: AiDifficulty) -> Self {
        let max_depth = match difficulty {
            AiDifficulty::Easy => 1,
            AiDifficulty::Medium => 2,
            AiDifficulty::Hard => 3,
        };

        Self { max_depth }
    }

    /// Choose best action for unit
    pub fn choose_action(
        &self,
        world: &World,
        grid: &Grid,
        unit: Entity,
    ) -> Option<UnitAction> {
        let possible_actions = self.generate_actions(world, grid, unit);

        if possible_actions.is_empty() {
            return Some(UnitAction::Wait);
        }

        // Evaluate each action
        let mut best_action = None;
        let mut best_score = f32::MIN;

        for action in possible_actions {
            let score = self.evaluate_action(world, grid, unit, &action);

            if score > best_score {
                best_score = score;
                best_action = Some(action);
            }
        }

        best_action
    }

    fn generate_actions(
        &self,
        world: &World,
        grid: &Grid,
        unit: Entity,
    ) -> Vec<UnitAction> {
        let mut actions = Vec::new();

        let grid_pos = world.get::<GridPosition>(unit)
            .expect("Unit has no position");
        let stats = world.get::<UnitStats>(unit)
            .expect("Unit has no stats");

        // Generate movement actions
        let reachable = grid.get_tiles_in_range(grid_pos.position, stats.movement);
        for pos in reachable {
            if grid.is_walkable(pos) {
                if let Some(path) = grid.find_path(grid_pos.position, pos) {
                    actions.push(UnitAction::Move { path });
                }
            }
        }

        // Generate attack actions
        let attack_range = grid.get_tiles_in_range(grid_pos.position, stats.attack_range);
        for pos in attack_range {
            if let Some(tile) = grid.get_tile(pos) {
                if let Some(target) = tile.occupant {
                    // Check if target is enemy
                    if let Some(target_unit) = world.get::<Unit>(target) {
                        if let Some(unit_comp) = world.get::<Unit>(unit) {
                            if target_unit.team != unit_comp.team {
                                actions.push(UnitAction::Attack { target });
                            }
                        }
                    }
                }
            }
        }

        actions
    }

    fn evaluate_action(
        &self,
        world: &World,
        grid: &Grid,
        unit: Entity,
        action: &UnitAction,
    ) -> f32 {
        match action {
            UnitAction::Attack { target } => {
                // Prioritize attacking weak enemies
                if let Some(target_stats) = world.get::<UnitStats>(*target) {
                    let damage_potential = 100.0 - target_stats.current_health as f32;
                    return damage_potential * 2.0;
                }
                50.0
            }
            UnitAction::Move { path } => {
                if path.is_empty() {
                    return 0.0;
                }

                let final_pos = path[path.len() - 1];

                // Find nearest enemy
                let mut min_distance = f32::MAX;

                for (_, (enemy_unit, enemy_pos, enemy_stats)) in
                    world.query::<(&Unit, &GridPosition, &UnitStats)>()
                {
                    if let Some(unit_comp) = world.get::<Unit>(unit) {
                        if enemy_unit.team != unit_comp.team && enemy_stats.is_alive() {
                            let distance = ((final_pos.x - enemy_pos.position.x).abs()
                                + (final_pos.y - enemy_pos.position.y).abs()) as f32;
                            min_distance = min_distance.min(distance);
                        }
                    }
                }

                // Prefer moving closer to enemies
                100.0 / (min_distance + 1.0)
            }
            UnitAction::Wait => 1.0,
            _ => 10.0,
        }
    }
}
```

---

### **6. Main Game Loop** (Day 4-5)

**File:** `examples/turnbased/src/main.rs`

```rust
mod components;
mod grid;
mod units;
mod abilities;
mod ai;
mod systems;
mod ui;

use agent_game_engine_core::prelude::*;
use agent_game_engine_platform::{Platform, WindowConfig, Input};
use agent_game_engine_rendering::Renderer;
use components::*;
use grid::*;
use systems::turn::*;
use ai::minimax::MinimaxAi;
use anyhow::Result;
use std::time::Instant;
use glam::IVec2;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Turn-Based Strategy Example");

    // Create platform
    let mut platform = Platform::new()?;
    let window = platform.create_window(WindowConfig {
        title: "Turn-Based Strategy".to_string(),
        width: 1280,
        height: 720,
        ..Default::default()
    })?;

    let mut renderer = Renderer::new(&window)?;

    // Create world
    let mut world = World::new();
    register_components(&mut world);

    // Create grid
    let mut grid = Grid::new(GridType::Square, 20, 15);

    // Add terrain
    setup_terrain(&mut grid);

    // Spawn units
    spawn_units(&mut world, &mut grid);

    // Create turn manager
    let mut turn_manager = TurnManager::new();
    turn_manager.start_turn(&mut world);

    // Create AI
    let ai = MinimaxAi::new(AiDifficulty::Medium);

    // UI state
    let mut selected_unit: Option<Entity> = None;
    let mut highlighted_tiles: Vec<IVec2> = Vec::new();

    // Game loop
    let mut last_frame = Instant::now();
    let mut running = true;

    while running {
        let now = Instant::now();
        let dt = (now - last_frame).as_secs_f32();
        last_frame = now;

        // Input
        let input = platform.poll_events();
        if input.should_quit() {
            running = false;
        }

        // Handle current phase
        match turn_manager.current_phase() {
            GamePhase::PlayerTurn => {
                handle_player_turn(
                    &mut world,
                    &mut grid,
                    &mut turn_manager,
                    &input,
                    &mut selected_unit,
                    &mut highlighted_tiles,
                );
            }
            GamePhase::EnemyTurn => {
                handle_enemy_turn(
                    &mut world,
                    &mut grid,
                    &mut turn_manager,
                    &ai,
                );
            }
            GamePhase::Victory | GamePhase::Defeat => {
                // Show end screen
                if input.is_key_just_pressed("Enter") {
                    running = false;
                }
            }
        }

        // Render
        renderer.begin_frame()?;
        render_grid(&grid, &mut renderer)?;
        render_units(&world, &grid, &mut renderer)?;
        render_ui(&world, &turn_manager, &mut renderer, &highlighted_tiles)?;
        renderer.end_frame()?;
    }

    Ok(())
}

fn register_components(world: &mut World) {
    world.register::<Unit>();
    world.register::<UnitStats>();
    world.register::<GridPosition>();
    world.register::<ActionPoints>();
    world.register::<Abilities>();
    world.register::<AiController>();
    world.register::<UnitSprite>();
}

fn setup_terrain(grid: &mut Grid) {
    // Add some variety to terrain
    for y in 0..15 {
        for x in 0..20 {
            let pos = IVec2::new(x, y);
            if let Some(tile) = grid.get_tile_mut(pos) {
                // Add forests
                if (x + y) % 5 == 0 {
                    tile.tile_type = TileType::Forest;
                }
                // Add mountains
                if x == 10 && y >= 5 && y <= 10 {
                    tile.tile_type = TileType::Mountain;
                }
            }
        }
    }
}

fn spawn_units(world: &mut World, grid: &mut Grid) {
    // Spawn player units
    let player_positions = vec![
        IVec2::new(2, 7),
        IVec2::new(3, 6),
        IVec2::new(3, 8),
        IVec2::new(4, 7),
    ];

    for (i, pos) in player_positions.iter().enumerate() {
        let unit_type = match i {
            0 => UnitType::Infantry,
            1 => UnitType::Archer,
            2 => UnitType::Mage,
            _ => UnitType::Cavalry,
        };

        spawn_unit(world, grid, *pos, unit_type, Team::Player);
    }

    // Spawn enemy units
    let enemy_positions = vec![
        IVec2::new(17, 7),
        IVec2::new(16, 6),
        IVec2::new(16, 8),
        IVec2::new(15, 7),
    ];

    for (i, pos) in enemy_positions.iter().enumerate() {
        let unit_type = match i {
            0 => UnitType::Tank,
            1 => UnitType::Archer,
            2 => UnitType::Mage,
            _ => UnitType::Infantry,
        };

        spawn_unit(world, grid, *pos, unit_type, Team::Enemy);
    }
}

fn spawn_unit(
    world: &mut World,
    grid: &mut Grid,
    pos: IVec2,
    unit_type: UnitType,
    team: Team,
) -> Entity {
    let entity = world.spawn();

    world.add(entity, Unit {
        unit_name: format!("{:?}", unit_type),
        unit_type,
        team,
    });

    world.add(entity, UnitStats::new(unit_type));
    world.add(entity, GridPosition { position: pos });
    world.add(entity, ActionPoints::new(2));
    world.add(entity, Abilities {
        abilities: Vec::new(),
    });

    if team == Team::Enemy {
        world.add(entity, AiController {
            ai_type: AiType::Balanced,
            difficulty: AiDifficulty::Medium,
        });
    }

    let color = match team {
        Team::Player => [0.2, 0.6, 1.0, 1.0],
        Team::Enemy => [1.0, 0.2, 0.2, 1.0],
        Team::Neutral => [0.5, 0.5, 0.5, 1.0],
    };

    world.add(entity, UnitSprite {
        texture: format!("{:?}.png", unit_type),
        color,
    });

    // Mark tile as occupied
    if let Some(tile) = grid.get_tile_mut(pos) {
        tile.occupant = Some(entity);
    }

    entity
}

fn handle_player_turn(
    world: &mut World,
    grid: &mut Grid,
    turn_manager: &mut TurnManager,
    input: &Input,
    selected_unit: &mut Option<Entity>,
    highlighted_tiles: &mut Vec<IVec2>,
) {
    // Handle unit selection and actions
    // (Simplified - full implementation would handle mouse input)

    if input.is_key_just_pressed("Space") {
        turn_manager.end_turn(world);
    }
}

fn handle_enemy_turn(
    world: &mut World,
    grid: &mut Grid,
    turn_manager: &mut TurnManager,
    ai: &MinimaxAi,
) {
    if let Some(active_unit) = turn_manager.active_unit() {
        // AI chooses action
        if let Some(action) = ai.choose_action(world, grid, active_unit) {
            execute_action(world, active_unit, action);
        }

        turn_manager.next_unit(world);
    } else {
        turn_manager.end_turn(world);
    }
}

fn render_grid(grid: &Grid, renderer: &mut Renderer) -> Result<()> {
    // Render grid tiles
    Ok(())
}

fn render_units(world: &World, grid: &Grid, renderer: &mut Renderer) -> Result<()> {
    for (_, (grid_pos, sprite)) in world.query::<(&GridPosition, &UnitSprite)>() {
        let world_pos = grid.grid_to_world(grid_pos.position);
        renderer.draw_sprite(&sprite.texture, world_pos, 1.0, sprite.color)?;
    }
    Ok(())
}

fn render_ui(
    world: &World,
    turn_manager: &TurnManager,
    renderer: &mut Renderer,
    highlighted_tiles: &[IVec2],
) -> Result<()> {
    // Render HUD, turn info, etc.
    Ok(())
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Grid system supports square and hexagonal layouts
- [ ] Units have stats, abilities, action points
- [ ] Turn-based gameplay works correctly
- [ ] Pathfinding finds optimal routes
- [ ] Combat resolves damage accurately
- [ ] AI makes reasonable tactical decisions
- [ ] Line of sight calculated correctly
- [ ] Movement restricted by action points
- [ ] Victory/defeat conditions trigger
- [ ] UI displays game state clearly
- [ ] Performance: 60 FPS with 50+ units
- [ ] Clean code with documentation

---

## 🎯 **Performance Targets**

| Metric | Target | Critical |
|--------|--------|----------|
| Frame rate | 60 FPS | 30 FPS |
| Pathfinding | < 10ms | < 50ms |
| AI decision | < 100ms | < 500ms |
| Grid size | 50x50 | 20x20 |
| Unit count | 100+ | 50+ |

---

**Dependencies:** Phase 1-3 (ECS, Rendering, Platform)
**Next:** [phase5-moba-example.md](phase5-moba-example.md)
