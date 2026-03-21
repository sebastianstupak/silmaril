//! Undo/redo stack for scene-level actions in the editor.
//!
//! `SceneUndoStack` records discrete `SceneAction`s.  Calling [`SceneUndoStack::pop_undo`]
//! removes the most recent action from the undo stack and pushes its inverse onto the
//! redo stack, returning the original action so the caller can apply its `before` state.
//! Calling [`SceneUndoStack::pop_redo`] does the symmetric operation.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A serialisable snapshot of a Transform component sufficient for undo/redo.
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct SerializedTransform {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// A discrete, reversible scene operation.
pub enum SceneAction {
    /// A transform was changed on an entity.
    ///
    /// `before` is the state **prior** to the change; `after` is the state
    /// **after** the change.  Undo applies `before`; redo applies `after`.
    SetTransform {
        entity_id: u64,
        before: SerializedTransform,
        after: SerializedTransform,
    },
}

// ---------------------------------------------------------------------------
// Stack implementation
// ---------------------------------------------------------------------------

/// Undo/redo stack for the live editor scene.
///
/// This type is intended to be stored in Tauri managed state wrapped in a
/// `std::sync::Mutex`:
///
/// ```ignore
/// .manage(std::sync::Mutex::new(SceneUndoStack::new()))
/// ```
pub struct SceneUndoStack {
    undo: Vec<SceneAction>,
    redo: Vec<SceneAction>,
}

impl SceneUndoStack {
    /// Creates an empty stack.
    pub fn new() -> Self {
        Self { undo: Vec::new(), redo: Vec::new() }
    }

    /// Returns `true` if there is at least one action that can be undone.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Returns `true` if there is at least one action that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Push a new action onto the undo stack.
    ///
    /// The redo stack is cleared — branching history is not supported.
    pub fn push(&mut self, action: SceneAction) {
        self.redo.clear();
        self.undo.push(action);
    }

    /// Pop the most recent action for undoing.
    ///
    /// The caller should apply the `before` state of the returned action.
    /// The action is pushed unchanged onto the redo stack so a subsequent
    /// [`pop_redo`](Self::pop_redo) re-applies the original `after` state.
    ///
    /// Returns `None` if the undo stack is empty.
    pub fn pop_undo(&mut self) -> Option<SceneAction> {
        let action = self.undo.pop()?;
        // Clone the action onto redo — the same record stores both before/after,
        // so redo can re-apply `after` without any field swapping.
        match &action {
            SceneAction::SetTransform { entity_id, before, after } => {
                self.redo.push(SceneAction::SetTransform {
                    entity_id: *entity_id,
                    before: before.clone(),
                    after: after.clone(),
                });
            }
        }
        Some(action)
    }

    /// Pop the most recent undone action for redoing.
    ///
    /// The caller should apply the `after` state of the returned action.
    /// The action is pushed back onto the undo stack unchanged.
    ///
    /// Returns `None` if the redo stack is empty.
    pub fn pop_redo(&mut self) -> Option<SceneAction> {
        let action = self.redo.pop()?;
        match &action {
            SceneAction::SetTransform { entity_id, before, after } => {
                self.undo.push(SceneAction::SetTransform {
                    entity_id: *entity_id,
                    before: before.clone(),
                    after: after.clone(),
                });
            }
        }
        Some(action)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_transform(px: f32) -> SerializedTransform {
        SerializedTransform {
            position: [px, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn undo_restores_previous_transform() {
        let mut stack = SceneUndoStack::new();
        stack.push(SceneAction::SetTransform {
            entity_id: 1,
            before: make_transform(0.0),
            after: make_transform(5.0),
        });
        assert!(stack.can_undo());
        let action = stack.pop_undo().unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
        let SceneAction::SetTransform { before, .. } = action;
        assert_eq!(before.position[0], 0.0);
    }

    #[test]
    fn push_clears_redo_stack() {
        let mut stack = SceneUndoStack::new();
        stack.push(SceneAction::SetTransform {
            entity_id: 1,
            before: SerializedTransform::default(),
            after: SerializedTransform::default(),
        });
        stack.pop_undo(); // moves to redo
        stack.push(SceneAction::SetTransform {
            entity_id: 2,
            before: SerializedTransform::default(),
            after: SerializedTransform::default(),
        });
        assert!(!stack.can_redo()); // redo was cleared
    }

    #[test]
    fn redo_reapplies_undone_action() {
        let mut stack = SceneUndoStack::new();
        stack.push(SceneAction::SetTransform {
            entity_id: 1,
            before: make_transform(0.0),
            after: make_transform(5.0),
        });
        stack.pop_undo();
        let redo_action = stack.pop_redo().unwrap();
        // After redo, the undo stack should have the original action reconstructed
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        // The redo action should apply `after` (5.0), so its `after` field is 5.0
        let SceneAction::SetTransform { after, .. } = redo_action;
        assert_eq!(after.position[0], 5.0);
    }

    #[test]
    fn empty_stack_returns_none() {
        let mut stack = SceneUndoStack::new();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert!(stack.pop_undo().is_none());
        assert!(stack.pop_redo().is_none());
    }
}
