//! Delta compression for efficient state synchronization

use super::{ComponentData, WorldState};
use crate::ecs::Entity;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Delta between two world states
///
/// Computes the minimal set of changes needed to transform one world state
/// into another. This is used for efficient network synchronization and
/// save file compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateDelta {
    /// Base state version
    pub base_version: u32,
    /// Target state version
    pub target_version: u32,
    /// Added entities
    pub added_entities: Vec<super::EntityMetadata>,
    /// Removed entities
    pub removed_entities: Vec<Entity>,
    /// Modified components (entity, component data)
    pub modified_components: HashMap<Entity, Vec<ComponentData>>,
    /// Removed components (entity, type name)
    pub removed_components: HashMap<Entity, Vec<String>>,
}

impl WorldStateDelta {
    /// Compute delta from old to new state
    ///
    /// Analyzes both states and produces a minimal delta containing only
    /// the changes needed to transform old into new.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::serialization::{WorldState, WorldStateDelta};
    /// # let old_state = WorldState::new();
    /// # let new_state = WorldState::new();
    /// let delta = WorldStateDelta::compute(&old_state, &new_state);
    /// ```
    pub fn compute(old: &WorldState, new: &WorldState) -> Self {
        let old_entities: HashSet<_> = old.entities.iter().map(|e| e.entity).collect();
        let new_entities: HashSet<_> = new.entities.iter().map(|e| e.entity).collect();

        // Find added entities
        let added_entities: Vec<_> = new
            .entities
            .iter()
            .filter(|e| !old_entities.contains(&e.entity))
            .cloned()
            .collect();

        // Find removed entities
        let removed_entities: Vec<_> = old_entities.difference(&new_entities).copied().collect();

        // Find modified and removed components
        let mut modified_components = HashMap::new();
        let mut removed_components = HashMap::new();

        for entity in new_entities.iter() {
            let old_comps = old.components.get(entity);
            let new_comps = new.components.get(entity);

            match (old_comps, new_comps) {
                (Some(old), Some(new)) => {
                    // Find modified components
                    let modified: Vec<_> = new
                        .iter()
                        .filter(|nc| {
                            !old.iter().any(|oc| {
                                oc.type_id() == nc.type_id()
                                    && bincode::serialize(oc).unwrap()
                                        == bincode::serialize(nc).unwrap()
                            })
                        })
                        .cloned()
                        .collect();

                    if !modified.is_empty() {
                        modified_components.insert(*entity, modified);
                    }

                    // Find removed components
                    let removed: Vec<_> = old
                        .iter()
                        .filter(|oc| !new.iter().any(|nc| nc.type_id() == oc.type_id()))
                        .map(|c| c.type_name().to_string())
                        .collect();

                    if !removed.is_empty() {
                        removed_components.insert(*entity, removed);
                    }
                }
                (None, Some(new)) => {
                    // All components are new for this entity
                    modified_components.insert(*entity, new.clone());
                }
                _ => {}
            }
        }

        Self {
            base_version: old.metadata.version,
            target_version: new.metadata.version,
            added_entities,
            removed_entities,
            modified_components,
            removed_components,
        }
    }

    /// Apply delta to a base state
    ///
    /// Modifies the base state in-place by applying all changes from the delta.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use engine_core::serialization::{WorldState, WorldStateDelta};
    /// # let mut base = WorldState::new();
    /// # let delta = WorldStateDelta {
    /// #     base_version: 1,
    /// #     target_version: 2,
    /// #     added_entities: Vec::new(),
    /// #     removed_entities: Vec::new(),
    /// #     modified_components: Default::default(),
    /// #     removed_components: Default::default(),
    /// # };
    /// delta.apply(&mut base);
    /// ```
    pub fn apply(&self, base: &mut WorldState) {
        // Remove entities
        for entity in &self.removed_entities {
            base.entities.retain(|e| e.entity != *entity);
            base.components.remove(entity);
        }

        // Add entities
        base.entities.extend(self.added_entities.clone());

        // Modify components
        for (entity, components) in &self.modified_components {
            let entry = base.components.entry(*entity).or_default();

            for new_comp in components {
                // Remove old version of this component type
                entry.retain(|c| c.type_id() != new_comp.type_id());
                // Add new version
                entry.push(new_comp.clone());
            }
        }

        // Remove components
        for (entity, type_names) in &self.removed_components {
            if let Some(components) = base.components.get_mut(entity) {
                components.retain(|c| !type_names.contains(&c.type_name().to_string()));
            }
        }

        // Update metadata
        base.metadata.version = self.target_version;
        base.metadata.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        base.metadata.entity_count = base.entities.len();
        base.metadata.component_count = base.components.values().map(|v| v.len()).sum();
    }

    /// Check if delta is smaller than full state
    ///
    /// Returns true if transmitting the delta would use less bandwidth
    /// than transmitting the full state.
    pub fn is_smaller_than(&self, full_state: &WorldState) -> bool {
        let delta_size = bincode::serialize(self).unwrap().len();
        let full_size = bincode::serialize(full_state).unwrap().len();
        delta_size < full_size
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_empty_delta() {
        let state1 = WorldState::new();
        let state2 = WorldState::new();

        let delta = WorldStateDelta::compute(&state1, &state2);

        assert_eq!(delta.added_entities.len(), 0);
        assert_eq!(delta.removed_entities.len(), 0);
        assert_eq!(delta.modified_components.len(), 0);
    }

    #[test]
    fn test_delta_apply() {
        let mut state1 = WorldState::new();
        let state2 = WorldState::new();

        let delta = WorldStateDelta::compute(&state1, &state2);
        delta.apply(&mut state1);

        assert_eq!(state1.metadata.version, state2.metadata.version);
        assert_eq!(state1.entities.len(), state2.entities.len());
    }

    #[test]
    fn test_delta_serialization() {
        let state1 = WorldState::new();
        let state2 = WorldState::new();

        let delta = WorldStateDelta::compute(&state1, &state2);

        // Test bincode serialization
        let bytes = bincode::serialize(&delta).unwrap();
        let deserialized: WorldStateDelta = bincode::deserialize(&bytes).unwrap();

        assert_eq!(delta.base_version, deserialized.base_version);
        assert_eq!(delta.target_version, deserialized.target_version);
    }
}
