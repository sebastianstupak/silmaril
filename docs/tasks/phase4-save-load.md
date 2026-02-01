# Phase 4.6: Save/Load System

**Status:** ⚪ Not Started
**Estimated Time:** 2-3 days
**Priority:** High (game state persistence)

---

## 🎯 **Objective**

Implement save/load system for game state persistence. Support saving WorldState to disk, loading saved states, save game management with multiple slots, and versioning for backward compatibility.

**Must support:**
- WorldState serialization to disk
- Load saved states with validation
- Multiple save slots
- Save game metadata (timestamp, playtime, etc.)
- Version migration for backward compatibility
- Compression for smaller save files

---

## 📋 **Detailed Tasks**

### **1. Save Data Structures** (Day 1)

**File:** `engine/save_system/src/save_data.rs`

```rust
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Save file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    pub save_name: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub playtime_seconds: u64,
    pub screenshot_path: Option<PathBuf>,
    pub custom_data: serde_json::Value,
}

impl SaveMetadata {
    pub fn new(save_name: String, version: String) -> Self {
        let now = Utc::now();
        Self {
            save_name,
            version,
            created_at: now,
            modified_at: now,
            playtime_seconds: 0,
            screenshot_path: None,
            custom_data: serde_json::Value::Null,
        }
    }

    pub fn update_modified(&mut self) {
        self.modified_at = Utc::now();
    }
}

/// Save file container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveFile {
    pub metadata: SaveMetadata,
    pub world_state: WorldStateSnapshot,
}

/// World state snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateSnapshot {
    pub entities: Vec<EntitySnapshot>,
    pub resources: serde_json::Value,
}

/// Entity snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: u64,
    pub components: serde_json::Value,
}

impl SaveFile {
    /// Create new save file
    pub fn new(save_name: String, version: String, world_state: WorldStateSnapshot) -> Self {
        Self {
            metadata: SaveMetadata::new(save_name, version),
            world_state,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self, compress: bool) -> Result<Vec<u8>, SaveError> {
        let json = serde_json::to_vec(self).map_err(|e| SaveError::SerializationFailed {
            details: e.to_string(),
        })?;

        if compress {
            Self::compress(&json)
        } else {
            Ok(json)
        }
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8], compressed: bool) -> Result<Self, SaveError> {
        let json = if compressed {
            Self::decompress(bytes)?
        } else {
            bytes.to_vec()
        };

        serde_json::from_slice(&json).map_err(|e| SaveError::DeserializationFailed {
            details: e.to_string(),
        })
    }

    /// Compress data using zstd
    fn compress(data: &[u8]) -> Result<Vec<u8>, SaveError> {
        zstd::encode_all(data, 3).map_err(|e| SaveError::CompressionFailed {
            details: e.to_string(),
        })
    }

    /// Decompress data using zstd
    fn decompress(data: &[u8]) -> Result<Vec<u8>, SaveError> {
        zstd::decode_all(data).map_err(|e| SaveError::DecompressionFailed {
            details: e.to_string(),
        })
    }
}
```

---

### **2. Save Manager** (Day 1-2)

**File:** `engine/save_system/src/manager.rs`

```rust
use std::path::{Path, PathBuf};
use std::fs;

/// Save slot
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SaveSlot(pub u32);

impl SaveSlot {
    pub fn auto_save() -> Self {
        Self(0)
    }

    pub fn quick_save() -> Self {
        Self(1)
    }

    pub fn manual(slot: u32) -> Self {
        Self(slot + 2) // Reserve 0 and 1 for auto/quick
    }
}

/// Save manager
pub struct SaveManager {
    save_directory: PathBuf,
    current_version: String,
    compress_saves: bool,
}

impl SaveManager {
    /// Create new save manager
    pub fn new(save_directory: PathBuf, version: String) -> Self {
        Self {
            save_directory,
            current_version: version,
            compress_saves: true,
        }
    }

    /// Initialize save directory
    pub fn initialize(&self) -> Result<(), SaveError> {
        if !self.save_directory.exists() {
            fs::create_dir_all(&self.save_directory).map_err(|e| SaveError::IoError {
                details: format!("Failed to create save directory: {}", e),
            })?;
        }

        tracing::info!("Save directory initialized: {:?}", self.save_directory);
        Ok(())
    }

    /// Get save file path
    fn get_save_path(&self, slot: SaveSlot) -> PathBuf {
        self.save_directory.join(format!("save_{}.dat", slot.0))
    }

    /// Get metadata path
    fn get_metadata_path(&self, slot: SaveSlot) -> PathBuf {
        self.save_directory.join(format!("save_{}.meta", slot.0))
    }

    /// Save game
    pub fn save_game(
        &mut self,
        slot: SaveSlot,
        save_name: String,
        world_state: WorldStateSnapshot,
    ) -> Result<(), SaveError> {
        tracing::info!("Saving game to slot {:?}: {}", slot, save_name);

        let mut save_file = SaveFile::new(save_name, self.current_version.clone(), world_state);

        // Update timestamp
        save_file.metadata.update_modified();

        // Serialize and save
        let bytes = save_file.to_bytes(self.compress_saves)?;

        let save_path = self.get_save_path(slot);
        fs::write(&save_path, bytes).map_err(|e| SaveError::IoError {
            details: format!("Failed to write save file: {}", e),
        })?;

        // Save metadata separately for quick access
        let metadata_bytes = serde_json::to_vec_pretty(&save_file.metadata).map_err(|e| {
            SaveError::SerializationFailed {
                details: e.to_string(),
            }
        })?;

        let metadata_path = self.get_metadata_path(slot);
        fs::write(&metadata_path, metadata_bytes).map_err(|e| SaveError::IoError {
            details: format!("Failed to write metadata: {}", e),
        })?;

        tracing::info!("Game saved successfully to slot {:?}", slot);
        Ok(())
    }

    /// Load game
    pub fn load_game(&self, slot: SaveSlot) -> Result<SaveFile, SaveError> {
        tracing::info!("Loading game from slot {:?}", slot);

        let save_path = self.get_save_path(slot);

        if !save_path.exists() {
            return Err(SaveError::SaveNotFound { slot });
        }

        // Load and deserialize
        let bytes = fs::read(&save_path).map_err(|e| SaveError::IoError {
            details: format!("Failed to read save file: {}", e),
        })?;

        let save_file = SaveFile::from_bytes(&bytes, self.compress_saves)?;

        // Validate version
        if save_file.metadata.version != self.current_version {
            tracing::warn!(
                "Save file version mismatch: {} != {}",
                save_file.metadata.version,
                self.current_version
            );
            // TODO: Attempt migration
        }

        tracing::info!("Game loaded successfully from slot {:?}", slot);
        Ok(save_file)
    }

    /// Delete save
    pub fn delete_save(&self, slot: SaveSlot) -> Result<(), SaveError> {
        tracing::info!("Deleting save in slot {:?}", slot);

        let save_path = self.get_save_path(slot);
        let metadata_path = self.get_metadata_path(slot);

        if save_path.exists() {
            fs::remove_file(&save_path).map_err(|e| SaveError::IoError {
                details: format!("Failed to delete save file: {}", e),
            })?;
        }

        if metadata_path.exists() {
            fs::remove_file(&metadata_path).map_err(|e| SaveError::IoError {
                details: format!("Failed to delete metadata: {}", e),
            })?;
        }

        tracing::info!("Save deleted from slot {:?}", slot);
        Ok(())
    }

    /// List all saves
    pub fn list_saves(&self) -> Result<Vec<(SaveSlot, SaveMetadata)>, SaveError> {
        let mut saves = Vec::new();

        for entry in fs::read_dir(&self.save_directory).map_err(|e| SaveError::IoError {
            details: format!("Failed to read save directory: {}", e),
        })? {
            let entry = entry.map_err(|e| SaveError::IoError {
                details: format!("Failed to read directory entry: {}", e),
            })?;

            let path = entry.path();

            // Check for metadata files
            if let Some(ext) = path.extension() {
                if ext == "meta" {
                    // Parse slot number from filename
                    if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Some(slot_str) = file_name.strip_prefix("save_") {
                            if let Ok(slot_num) = slot_str.parse::<u32>() {
                                let slot = SaveSlot(slot_num);

                                // Load metadata
                                let metadata_bytes = fs::read(&path).map_err(|e| SaveError::IoError {
                                    details: format!("Failed to read metadata: {}", e),
                                })?;

                                let metadata: SaveMetadata = serde_json::from_slice(&metadata_bytes)
                                    .map_err(|e| SaveError::DeserializationFailed {
                                        details: e.to_string(),
                                    })?;

                                saves.push((slot, metadata));
                            }
                        }
                    }
                }
            }
        }

        // Sort by modified date (newest first)
        saves.sort_by(|a, b| b.1.modified_at.cmp(&a.1.modified_at));

        Ok(saves)
    }

    /// Check if save exists
    pub fn save_exists(&self, slot: SaveSlot) -> bool {
        self.get_save_path(slot).exists()
    }
}
```

---

### **3. WorldState Serialization** (Day 2)

**File:** `engine/ecs/src/world_serialization.rs`

```rust
use crate::{World, Entity, Component};
use serde_json::Value;
use std::collections::HashMap;

/// Serialize world state
pub fn serialize_world(world: &World) -> Result<WorldStateSnapshot, SaveError> {
    let mut entities = Vec::new();

    // Serialize each entity
    for entity in world.entities() {
        let entity_snapshot = serialize_entity(world, entity)?;
        entities.push(entity_snapshot);
    }

    // Serialize resources
    let resources = serialize_resources(world)?;

    Ok(WorldStateSnapshot { entities, resources })
}

/// Serialize entity
fn serialize_entity(world: &World, entity: Entity) -> Result<EntitySnapshot, SaveError> {
    let mut components = HashMap::new();

    // Get all components for this entity
    // This requires reflection or manual component registration
    // For now, assume we have a component registry

    // Example: Transform component
    if let Some(transform) = world.get_component::<Transform>(entity) {
        let json = serde_json::to_value(transform).map_err(|e| SaveError::SerializationFailed {
            details: e.to_string(),
        })?;
        components.insert("Transform".to_string(), json);
    }

    // Example: Other components...

    let components_value = serde_json::to_value(&components).map_err(|e| {
        SaveError::SerializationFailed {
            details: e.to_string(),
        }
    })?;

    Ok(EntitySnapshot {
        id: entity.id(),
        components: components_value,
    })
}

/// Serialize resources
fn serialize_resources(world: &World) -> Result<Value, SaveError> {
    let mut resources = HashMap::new();

    // Serialize global resources
    // TODO: Implement resource serialization

    serde_json::to_value(&resources).map_err(|e| SaveError::SerializationFailed {
        details: e.to_string(),
    })
}

/// Deserialize world state
pub fn deserialize_world(
    world: &mut World,
    snapshot: WorldStateSnapshot,
) -> Result<(), SaveError> {
    // Clear existing entities (optional)
    // world.clear();

    // Deserialize entities
    for entity_snapshot in snapshot.entities {
        deserialize_entity(world, entity_snapshot)?;
    }

    // Deserialize resources
    deserialize_resources(world, snapshot.resources)?;

    Ok(())
}

/// Deserialize entity
fn deserialize_entity(world: &mut World, snapshot: EntitySnapshot) -> Result<Entity, SaveError> {
    let entity = world.spawn_entity();

    // Deserialize components
    if let Value::Object(components) = snapshot.components {
        for (component_name, component_value) in components {
            // Use component registry to deserialize
            match component_name.as_str() {
                "Transform" => {
                    let transform: Transform = serde_json::from_value(component_value)
                        .map_err(|e| SaveError::DeserializationFailed {
                            details: e.to_string(),
                        })?;
                    world.add_component(entity, transform);
                }
                // Other components...
                _ => {
                    tracing::warn!("Unknown component type: {}", component_name);
                }
            }
        }
    }

    Ok(entity)
}

/// Deserialize resources
fn deserialize_resources(world: &mut World, resources: Value) -> Result<(), SaveError> {
    // TODO: Implement resource deserialization
    Ok(())
}
```

---

### **4. Version Migration** (Day 3)

**File:** `engine/save_system/src/migration.rs`

```rust
use semver::Version;

/// Version migrator
pub struct VersionMigrator {
    migrations: Vec<Migration>,
}

impl VersionMigrator {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Register migration
    pub fn register_migration(&mut self, migration: Migration) {
        self.migrations.push(migration);
        self.migrations
            .sort_by(|a, b| a.from_version.cmp(&b.from_version));
    }

    /// Migrate save file from one version to another
    pub fn migrate(
        &self,
        save_file: &mut SaveFile,
        target_version: &str,
    ) -> Result<(), SaveError> {
        let current_version = Version::parse(&save_file.metadata.version).map_err(|e| {
            SaveError::MigrationFailed {
                details: format!("Invalid version: {}", e),
            }
        })?;

        let target = Version::parse(target_version).map_err(|e| SaveError::MigrationFailed {
            details: format!("Invalid target version: {}", e),
        })?;

        if current_version >= target {
            return Ok(()); // No migration needed
        }

        tracing::info!(
            "Migrating save from {} to {}",
            current_version,
            target
        );

        // Find applicable migrations
        let mut applicable_migrations: Vec<&Migration> = self
            .migrations
            .iter()
            .filter(|m| {
                let from = Version::parse(&m.from_version).unwrap();
                let to = Version::parse(&m.to_version).unwrap();
                from >= current_version && to <= target
            })
            .collect();

        // Apply migrations in order
        for migration in applicable_migrations {
            tracing::info!(
                "Applying migration: {} -> {}",
                migration.from_version,
                migration.to_version
            );
            (migration.migrate_fn)(save_file)?;
            save_file.metadata.version = migration.to_version.clone();
        }

        Ok(())
    }
}

/// Migration definition
pub struct Migration {
    pub from_version: String,
    pub to_version: String,
    pub migrate_fn: Box<dyn Fn(&mut SaveFile) -> Result<(), SaveError>>,
}

impl Migration {
    pub fn new(
        from_version: impl Into<String>,
        to_version: impl Into<String>,
        migrate_fn: impl Fn(&mut SaveFile) -> Result<(), SaveError> + 'static,
    ) -> Self {
        Self {
            from_version: from_version.into(),
            to_version: to_version.into(),
            migrate_fn: Box::new(migrate_fn),
        }
    }
}

// Example migration
pub fn create_example_migration() -> Migration {
    Migration::new("1.0.0", "1.1.0", |save_file| {
        // Modify save file structure
        // Example: Add new field to metadata
        save_file.metadata.custom_data = serde_json::json!({
            "migrated": true,
        });

        Ok(())
    })
}
```

---

### **5. Save/Load UI** (Day 3)

**File:** `engine/save_system/src/ui.rs`

```rust
use egui::{Context, Window, ScrollArea, Button};

/// Save/Load UI
pub struct SaveLoadUI {
    show_window: bool,
    mode: SaveLoadMode,
    saves: Vec<(SaveSlot, SaveMetadata)>,
    selected_slot: Option<SaveSlot>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SaveLoadMode {
    Save,
    Load,
}

impl SaveLoadUI {
    pub fn new() -> Self {
        Self {
            show_window: false,
            mode: SaveLoadMode::Load,
            saves: Vec::new(),
            selected_slot: None,
        }
    }

    /// Show save dialog
    pub fn show_save_dialog(&mut self, saves: Vec<(SaveSlot, SaveMetadata)>) {
        self.show_window = true;
        self.mode = SaveLoadMode::Save;
        self.saves = saves;
    }

    /// Show load dialog
    pub fn show_load_dialog(&mut self, saves: Vec<(SaveSlot, SaveMetadata)>) {
        self.show_window = true;
        self.mode = SaveLoadMode::Load;
        self.saves = saves;
    }

    /// Render UI
    pub fn render(&mut self, ctx: &Context) -> SaveLoadAction {
        let mut action = SaveLoadAction::None;

        if !self.show_window {
            return action;
        }

        let title = match self.mode {
            SaveLoadMode::Save => "Save Game",
            SaveLoadMode::Load => "Load Game",
        };

        Window::new(title)
            .default_width(500.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                ui.heading("Save Slots");
                ui.separator();

                ScrollArea::vertical().show(ui, |ui| {
                    for (slot, metadata) in &self.saves {
                        let is_selected = self.selected_slot == Some(*slot);

                        let response = ui.selectable_label(is_selected, format!(
                            "{} - {} ({})",
                            slot.0,
                            metadata.save_name,
                            metadata.modified_at.format("%Y-%m-%d %H:%M:%S")
                        ));

                        if response.clicked() {
                            self.selected_slot = Some(*slot);
                        }
                    }

                    // Empty slots for saving
                    if self.mode == SaveLoadMode::Save {
                        for i in self.saves.len()..10 {
                            let slot = SaveSlot::manual(i as u32);
                            let is_selected = self.selected_slot == Some(slot);

                            let response = ui.selectable_label(is_selected, format!(
                                "{} - <Empty>",
                                slot.0
                            ));

                            if response.clicked() {
                                self.selected_slot = Some(slot);
                            }
                        }
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    let button_text = match self.mode {
                        SaveLoadMode::Save => "Save",
                        SaveLoadMode::Load => "Load",
                    };

                    if ui.button(button_text).clicked() {
                        if let Some(slot) = self.selected_slot {
                            action = match self.mode {
                                SaveLoadMode::Save => SaveLoadAction::Save(slot),
                                SaveLoadMode::Load => SaveLoadAction::Load(slot),
                            };
                            self.show_window = false;
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.show_window = false;
                    }

                    if self.selected_slot.is_some() && ui.button("Delete").clicked() {
                        if let Some(slot) = self.selected_slot {
                            action = SaveLoadAction::Delete(slot);
                            self.selected_slot = None;
                        }
                    }
                });
            });

        action
    }
}

/// Save/Load action
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SaveLoadAction {
    None,
    Save(SaveSlot),
    Load(SaveSlot),
    Delete(SaveSlot),
}
```

---

## ✅ **Acceptance Criteria**

- [ ] WorldState serialization to JSON
- [ ] Save to disk with compression (zstd)
- [ ] Load from disk with decompression
- [ ] Multiple save slots (auto, quick, manual)
- [ ] Save metadata (name, timestamp, playtime)
- [ ] List all saves with metadata
- [ ] Delete saves
- [ ] Version migration system
- [ ] Save/Load UI with egui
- [ ] Error handling (corrupted saves, missing files)

---

## 🧪 **Tests**

```rust
#[test]
fn test_save_file_serialization() {
    let world_state = WorldStateSnapshot {
        entities: Vec::new(),
        resources: serde_json::Value::Null,
    };

    let save_file = SaveFile::new("Test Save".to_string(), "1.0.0".to_string(), world_state);

    let bytes = save_file.to_bytes(true).unwrap();
    let loaded = SaveFile::from_bytes(&bytes, true).unwrap();

    assert_eq!(loaded.metadata.save_name, "Test Save");
    assert_eq!(loaded.metadata.version, "1.0.0");
}

#[test]
fn test_save_manager() {
    let temp_dir = std::env::temp_dir().join("save_test");
    let mut manager = SaveManager::new(temp_dir.clone(), "1.0.0".to_string());
    manager.initialize().unwrap();

    let world_state = WorldStateSnapshot {
        entities: Vec::new(),
        resources: serde_json::Value::Null,
    };

    let slot = SaveSlot::manual(0);
    manager
        .save_game(slot, "Test".to_string(), world_state.clone())
        .unwrap();

    assert!(manager.save_exists(slot));

    let loaded = manager.load_game(slot).unwrap();
    assert_eq!(loaded.metadata.save_name, "Test");

    manager.delete_save(slot).unwrap();
    assert!(!manager.save_exists(slot));

    std::fs::remove_dir_all(temp_dir).unwrap();
}

#[test]
fn test_version_migration() {
    let mut migrator = VersionMigrator::new();

    migrator.register_migration(Migration::new("1.0.0", "1.1.0", |save| {
        save.metadata.custom_data = serde_json::json!({ "migrated": true });
        Ok(())
    }));

    let mut save_file = SaveFile::new(
        "Test".to_string(),
        "1.0.0".to_string(),
        WorldStateSnapshot {
            entities: Vec::new(),
            resources: serde_json::Value::Null,
        },
    );

    migrator.migrate(&mut save_file, "1.1.0").unwrap();

    assert_eq!(save_file.metadata.version, "1.1.0");
    assert_eq!(
        save_file.metadata.custom_data,
        serde_json::json!({ "migrated": true })
    );
}
```

---

## ⚡ **Performance Targets**

- **Save Time:** <500ms for typical game state
- **Load Time:** <1 second for typical save file
- **Compression Ratio:** 60-80% size reduction
- **Memory Usage:** <50 MB during save/load
- **List Saves:** <100ms

---

## 📚 **Dependencies**

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
zstd = "0.13"
chrono = { version = "0.4", features = ["serde"] }
semver = "1.0"
egui = "0.28"
thiserror = "1.0"
```

---

**Dependencies:** [phase4-hot-reload.md](phase4-hot-reload.md)
**Next:** Phase 5 (Advanced Features)
