//! Undo/redo system — command pattern with snapshot checkpoints.
//!
//! Provides a reversible action stack for editor operations. Each [`EditorAction`]
//! captures enough state to both apply and reverse the operation.

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Opaque entity identifier used across the undo system.
pub type EntityId = u64;

/// Snapshot of an entity's complete state for restoration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub name: Option<String>,
    pub components: Vec<(String, Value)>,
}

/// A reversible editor action.
///
/// Each variant stores enough data to both apply (redo) and reverse (undo)
/// the operation without consulting external state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditorAction {
    /// Overwrite a component value.
    SetComponent {
        entity: EntityId,
        type_name: String,
        old: Value,
        new: Value,
    },
    /// Attach a new component to an entity.
    AddComponent {
        entity: EntityId,
        type_name: String,
        data: Value,
    },
    /// Detach a component, keeping a snapshot for restoration.
    RemoveComponent {
        entity: EntityId,
        type_name: String,
        snapshot: Value,
    },
    /// Spawn a new entity.
    CreateEntity {
        id: EntityId,
        name: Option<String>,
    },
    /// Destroy an entity, keeping a full snapshot for restoration.
    DeleteEntity {
        id: EntityId,
        snapshot: EntitySnapshot,
    },
    /// Rename an entity.
    RenameEntity {
        id: EntityId,
        old_name: Option<String>,
        new_name: Option<String>,
    },
    /// A group of actions applied atomically.
    Batch {
        label: String,
        actions: Vec<EditorAction>,
    },
}

impl EditorAction {
    /// Human-readable description of this action.
    pub fn description(&self) -> String {
        match self {
            EditorAction::SetComponent { entity, type_name, .. } => {
                format!("Set {type_name} on Entity {entity}")
            }
            EditorAction::AddComponent { entity, type_name, .. } => {
                format!("Add {type_name} to Entity {entity}")
            }
            EditorAction::RemoveComponent { entity, type_name, .. } => {
                format!("Remove {type_name} from Entity {entity}")
            }
            EditorAction::CreateEntity { id, name } => {
                let label = name.as_deref().unwrap_or("unnamed");
                format!("Create Entity {id} ({label})")
            }
            EditorAction::DeleteEntity { id, .. } => format!("Delete Entity {id}"),
            EditorAction::RenameEntity { id, new_name, .. } => {
                let label = new_name.as_deref().unwrap_or("unnamed");
                format!("Rename Entity {id} to {label}")
            }
            EditorAction::Batch { label, .. } => label.clone(),
        }
    }
}

/// Undo/redo stack with configurable depth.
///
/// Actions are pushed onto the *done* stack. Undoing moves an action to the
/// *undone* stack; redoing moves it back. Any new push clears the redo stack.
#[derive(Debug, Serialize, Deserialize)]
pub struct UndoStack {
    done: VecDeque<EditorAction>,
    undone: VecDeque<EditorAction>,
    max_depth: usize,
}

impl UndoStack {
    /// Create a new stack that retains at most `max_depth` actions.
    pub fn new(max_depth: usize) -> Self {
        Self {
            done: VecDeque::new(),
            undone: VecDeque::new(),
            max_depth,
        }
    }

    /// Record a new action. Clears the redo stack and enforces max depth.
    pub fn push(&mut self, action: EditorAction) {
        self.undone.clear();
        self.done.push_back(action);
        while self.done.len() > self.max_depth {
            self.done.pop_front();
        }
    }

    /// Pop the most recent action from done, move it to undone, and return it
    /// so the caller can reverse it. Returns Err(()) when stack is empty.
    pub fn undo(&mut self) -> Result<EditorAction, ()> {
        match self.done.pop_back() {
            Some(action) => {
                self.undone.push_back(action.clone());
                Ok(action)
            }
            None => Err(()),
        }
    }

    /// Pop from undone, push to done, and return the action for re-execution.
    /// Returns Err(()) when stack is empty.
    pub fn redo(&mut self) -> Result<EditorAction, ()> {
        match self.undone.pop_back() {
            Some(action) => {
                self.done.push_back(action.clone());
                Ok(action)
            }
            None => Err(()),
        }
    }

    /// Whether there is an action to undo.
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    /// Whether there is an action to redo.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Discard all history.
    pub fn clear(&mut self) {
        self.done.clear();
        self.undone.clear();
    }

    /// Human-readable label of the action that would be undone next.
    pub fn undo_description(&self) -> Option<String> {
        self.done.back().map(|a| a.description())
    }

    /// Human-readable label of the action that would be redone next.
    pub fn redo_description(&self) -> Option<String> {
        self.undone.back().map(|a| a.description())
    }

    /// Returns descriptions for all done actions (oldest first).
    /// Used by CommandProcessor::history_summaries() to avoid circular imports.
    pub fn done_descriptions(&self) -> Vec<String> {
        self.done.iter().map(|a| a.description()).collect()
    }
}
