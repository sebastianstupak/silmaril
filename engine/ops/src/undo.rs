//! Undo/redo system — command pattern with snapshot checkpoints.
//!
//! Provides a reversible action stack for editor operations. Each [`EditorAction`]
//! captures enough state to both apply and reverse the operation.

use serde_json::Value;
use anyhow::{bail, Result};

/// Opaque entity identifier used across the undo system.
pub type EntityId = u64;

/// Snapshot of an entity's complete state for restoration.
#[derive(Debug, Clone)]
pub struct EntitySnapshot {
    pub id: EntityId,
    pub components: Vec<(String, Value)>,
}

/// A reversible editor action.
///
/// Each variant stores enough data to both apply (redo) and reverse (undo)
/// the operation without consulting external state.
#[derive(Debug, Clone)]
pub enum EditorAction {
    /// Overwrite a component value.
    SetComponent {
        entity: EntityId,
        name: String,
        old: Value,
        new: Value,
    },
    /// Attach a new component to an entity.
    AddComponent {
        entity: EntityId,
        name: String,
    },
    /// Detach a component, keeping a snapshot for restoration.
    RemoveComponent {
        entity: EntityId,
        name: String,
        snapshot: Value,
    },
    /// Spawn a new entity.
    CreateEntity {
        id: EntityId,
    },
    /// Destroy an entity, keeping a full snapshot for restoration.
    DeleteEntity {
        id: EntityId,
        snapshot: EntitySnapshot,
    },
    /// Rename an entity.
    RenameEntity {
        id: EntityId,
        old_name: String,
        new_name: String,
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
            EditorAction::SetComponent { entity, name, .. } => {
                format!("Set {name} on Entity {entity}")
            }
            EditorAction::AddComponent { entity, name } => {
                format!("Add {name} to Entity {entity}")
            }
            EditorAction::RemoveComponent { entity, name, .. } => {
                format!("Remove {name} from Entity {entity}")
            }
            EditorAction::CreateEntity { id } => {
                format!("Create Entity {id}")
            }
            EditorAction::DeleteEntity { id, .. } => {
                format!("Delete Entity {id}")
            }
            EditorAction::RenameEntity { id, new_name, .. } => {
                format!("Rename Entity {id} to {new_name}")
            }
            EditorAction::Batch { label, .. } => label.clone(),
        }
    }
}

/// Undo/redo stack with configurable depth.
///
/// Actions are pushed onto the *done* stack. Undoing moves an action to the
/// *undone* stack; redoing moves it back. Any new push clears the redo stack.
pub struct UndoStack {
    done: Vec<EditorAction>,
    undone: Vec<EditorAction>,
    max_depth: usize,
}

impl UndoStack {
    /// Create a new stack that retains at most `max_depth` actions.
    pub fn new(max_depth: usize) -> Self {
        Self {
            done: Vec::new(),
            undone: Vec::new(),
            max_depth,
        }
    }

    /// Record a new action. Clears the redo stack and enforces max depth.
    pub fn push(&mut self, action: EditorAction) {
        self.undone.clear();
        self.done.push(action);
        while self.done.len() > self.max_depth {
            self.done.remove(0);
        }
    }

    /// Pop the most recent action from done, move it to undone, and return it
    /// so the caller can reverse it.
    pub fn undo(&mut self) -> Result<EditorAction> {
        match self.done.pop() {
            Some(action) => {
                self.undone.push(action.clone());
                Ok(action)
            }
            None => bail!("Nothing to undo"),
        }
    }

    /// Pop from undone, push to done, and return the action for re-execution.
    pub fn redo(&mut self) -> Result<EditorAction> {
        match self.undone.pop() {
            Some(action) => {
                self.done.push(action.clone());
                Ok(action)
            }
            None => bail!("Nothing to redo"),
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
        self.done.last().map(|a| a.description())
    }

    /// Human-readable label of the action that would be redone next.
    pub fn redo_description(&self) -> Option<String> {
        self.undone.last().map(|a| a.description())
    }
}
