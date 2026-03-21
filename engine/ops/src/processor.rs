//! CommandProcessor — single choke-point for all template mutations.
//!
//! Owns a TemplateState + UndoStack. Every TemplateCommand flows through
//! execute(), which mutates in-memory state, writes YAML to disk, pushes
//! an EditorAction, and saves the undo history to <template>.undo.json.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use crate::command::TemplateCommand;
use crate::error::OpsError;
use crate::ipc::{ActionId, ActionSummary, CommandResult};
use crate::template::{TemplateComponent, TemplateEntity, TemplateState};
use crate::undo::{EditorAction, EntityId, EntitySnapshot, UndoStack};

pub struct CommandProcessor {
    state: TemplateState,
    undo_stack: UndoStack,
    // Parallel deque tracking ActionId for each entry in undo_stack.done
    done_ids: VecDeque<ActionId>,
    // Tracks ActionIds of undone actions (mirrors undo_stack.undone)
    undone_ids: VecDeque<ActionId>,
    template_path: PathBuf,
    next_action_id: ActionId,
}

impl CommandProcessor {
    /// Load a template YAML from disk. Loads .undo.json if present.
    pub fn load(path: PathBuf) -> Result<Self, OpsError> {
        let state = TemplateState::load_yaml(&path)?;
        let undo_path = undo_path_for(&path);
        let (undo_stack, done_ids, undone_ids, next_action_id) = if undo_path.exists() {
            load_undo_history(&undo_path)?
        } else {
            (UndoStack::new(100), VecDeque::new(), VecDeque::new(), 0)
        };
        Ok(Self { state, undo_stack, done_ids, undone_ids, template_path: path, next_action_id })
    }

    /// Execute a command. Mutates state, writes YAML, pushes undo action.
    pub fn execute(&mut self, cmd: TemplateCommand) -> Result<CommandResult, OpsError> {
        let action_id = self.next_action_id;
        self.next_action_id += 1;

        let action = self.apply_command(cmd)?;
        // New command clears redo stack — clear undone_ids to match
        self.undone_ids.clear();
        self.done_ids.push_back(action_id);
        // Trim done_ids to match UndoStack depth
        while self.done_ids.len() > 100 {
            self.done_ids.pop_front();
        }
        self.undo_stack.push(action);

        self.write_state()?;
        Ok(CommandResult { action_id, new_state: self.state.clone() })
    }

    /// Undo last action. Returns Ok(None) when nothing to undo.
    pub fn undo(&mut self) -> Result<Option<ActionId>, OpsError> {
        match self.undo_stack.undo() {
            Ok(action) => {
                let action_id = self.done_ids.pop_back();
                if let Some(id) = action_id {
                    self.undone_ids.push_back(id);
                }
                self.apply_inverse(&action)?;
                self.write_state()?;
                Ok(action_id)
            }
            Err(()) => Ok(None),
        }
    }

    /// Redo last undone action. Returns Ok(None) when nothing to redo.
    pub fn redo(&mut self) -> Result<Option<ActionId>, OpsError> {
        match self.undo_stack.redo() {
            Ok(action) => {
                let action_id = self.undone_ids.pop_back();
                if let Some(id) = action_id {
                    self.done_ids.push_back(id);
                }
                self.apply_action(&action)?;
                self.write_state()?;
                Ok(action_id)
            }
            Err(()) => Ok(None),
        }
    }

    pub fn history_summaries(&self) -> Vec<ActionSummary> {
        self.undo_stack.done_descriptions()
            .into_iter()
            .zip(self.done_ids.iter().copied())
            .map(|(description, action_id)| ActionSummary { action_id, description })
            .collect()
    }

    pub fn can_undo(&self) -> bool { self.undo_stack.can_undo() }
    pub fn can_redo(&self) -> bool { self.undo_stack.can_redo() }

    /// Read-only access to current state (for tests).
    pub fn state_ref(&self) -> &TemplateState { &self.state }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn apply_command(&mut self, cmd: TemplateCommand) -> Result<EditorAction, OpsError> {
        match cmd {
            TemplateCommand::CreateEntity { name } => {
                let id = self.next_entity_id();
                self.state.add_entity(TemplateEntity { id, name: name.clone(), components: vec![] });
                Ok(EditorAction::CreateEntity { id, name })
            }
            TemplateCommand::DeleteEntity { id } => {
                let entity = self.state.find_entity(id).ok_or(OpsError::EntityNotFound { id })?.clone();
                let snapshot = EntitySnapshot {
                    id: entity.id,
                    name: entity.name.clone(),
                    components: entity.components.iter().map(|c| (c.type_name.clone(), c.data.clone())).collect(),
                };
                self.state.remove_entity(id);
                Ok(EditorAction::DeleteEntity { id, snapshot })
            }
            TemplateCommand::RenameEntity { id, name: new_name } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                let old_name = entity.name.clone();
                entity.name = new_name.clone();
                Ok(EditorAction::RenameEntity { id, old_name, new_name })
            }
            TemplateCommand::DuplicateEntity { id } => {
                let original = self.state.find_entity(id).ok_or(OpsError::EntityNotFound { id })?.clone();
                let new_id = self.next_entity_id();
                let copy = TemplateEntity {
                    id: new_id,
                    name: original.name.as_deref().map(|n| format!("{n} (copy)")),
                    components: original.components.clone(),
                };
                let sub_actions: Vec<EditorAction> = std::iter::once(EditorAction::CreateEntity {
                    id: new_id,
                    name: copy.name.clone(),
                })
                .chain(copy.components.iter().map(|c| EditorAction::AddComponent {
                    entity: new_id,
                    type_name: c.type_name.clone(),
                    data: c.data.clone(),
                }))
                .collect();
                self.state.add_entity(copy);
                Ok(EditorAction::Batch {
                    label: format!("Duplicate Entity {id}"),
                    actions: sub_actions,
                })
            }
            TemplateCommand::SetComponent { id, type_name, data } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                let old = entity.components.iter()
                    .find(|c| c.type_name == type_name)
                    .map(|c| c.data.clone())
                    .unwrap_or(serde_json::Value::Null);
                if let Some(comp) = entity.components.iter_mut().find(|c| c.type_name == type_name) {
                    comp.data = data.clone();
                } else {
                    entity.components.push(TemplateComponent { type_name: type_name.clone(), data: data.clone() });
                }
                Ok(EditorAction::SetComponent { entity: id, type_name, old, new: data })
            }
            TemplateCommand::AddComponent { id, type_name, data } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                if entity.components.iter().any(|c| c.type_name == type_name) {
                    return Err(OpsError::ComponentAlreadyExists { entity: id, type_name });
                }
                entity.components.push(TemplateComponent { type_name: type_name.clone(), data: data.clone() });
                Ok(EditorAction::AddComponent { entity: id, type_name, data })
            }
            TemplateCommand::RemoveComponent { id, type_name } => {
                let entity = self.state.find_entity_mut(id).ok_or(OpsError::EntityNotFound { id })?;
                let pos = entity.components.iter().position(|c| c.type_name == type_name)
                    .ok_or_else(|| OpsError::ComponentNotFound { entity: id, type_name: type_name.clone() })?;
                let snapshot = entity.components.remove(pos).data;
                Ok(EditorAction::RemoveComponent { entity: id, type_name, snapshot })
            }
        }
    }

    fn apply_inverse(&mut self, action: &EditorAction) -> Result<(), OpsError> {
        match action {
            EditorAction::CreateEntity { id, .. } => {
                self.state.remove_entity(*id);
            }
            EditorAction::DeleteEntity { snapshot, .. } => {
                let entity = TemplateEntity {
                    id: snapshot.id,
                    name: snapshot.name.clone(),
                    components: snapshot.components.iter().map(|(t, d)| TemplateComponent {
                        type_name: t.clone(), data: d.clone(),
                    }).collect(),
                };
                self.state.add_entity(entity);
            }
            EditorAction::RenameEntity { id, old_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*id) { e.name = old_name.clone(); }
            }
            EditorAction::SetComponent { entity, type_name, old, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    if old.is_null() {
                        e.components.retain(|c| &c.type_name != type_name);
                    } else if let Some(c) = e.components.iter_mut().find(|c| &c.type_name == type_name) {
                        c.data = old.clone();
                    }
                }
            }
            EditorAction::AddComponent { entity, type_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.retain(|c| &c.type_name != type_name);
                }
            }
            EditorAction::RemoveComponent { entity, type_name, snapshot } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.push(TemplateComponent { type_name: type_name.clone(), data: snapshot.clone() });
                }
            }
            EditorAction::Batch { actions, .. } => {
                for a in actions.iter().rev() { self.apply_inverse(a)?; }
            }
        }
        Ok(())
    }

    fn apply_action(&mut self, action: &EditorAction) -> Result<(), OpsError> {
        match action {
            EditorAction::CreateEntity { id, name } => {
                self.state.add_entity(TemplateEntity { id: *id, name: name.clone(), components: vec![] });
            }
            EditorAction::DeleteEntity { id, .. } => { self.state.remove_entity(*id); }
            EditorAction::RenameEntity { id, new_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*id) { e.name = new_name.clone(); }
            }
            EditorAction::SetComponent { entity, type_name, new, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    if let Some(c) = e.components.iter_mut().find(|c| &c.type_name == type_name) {
                        c.data = new.clone();
                    } else {
                        e.components.push(TemplateComponent { type_name: type_name.clone(), data: new.clone() });
                    }
                }
            }
            EditorAction::AddComponent { entity, type_name, data } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.push(TemplateComponent { type_name: type_name.clone(), data: data.clone() });
                }
            }
            EditorAction::RemoveComponent { entity, type_name, .. } => {
                if let Some(e) = self.state.find_entity_mut(*entity) {
                    e.components.retain(|c| &c.type_name != type_name);
                }
            }
            EditorAction::Batch { actions, .. } => {
                for a in actions { self.apply_action(a)?; }
            }
        }
        Ok(())
    }

    fn next_entity_id(&self) -> EntityId {
        self.state.entities.iter().map(|e| e.id).max().map(|m| m + 1).unwrap_or(1)
    }

    fn write_state(&self) -> Result<(), OpsError> {
        self.state.save_yaml(&self.template_path)?;
        self.save_undo_history()
    }

    fn save_undo_history(&self) -> Result<(), OpsError> {
        let undo_path = undo_path_for(&self.template_path);
        let data = serde_json::to_string(&UndoHistoryFile {
            undo_stack: &self.undo_stack,
            done_ids: &self.done_ids,
            undone_ids: &self.undone_ids,
            next_action_id: self.next_action_id,
        })
        .map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
        std::fs::write(&undo_path, data)
            .map_err(|e| OpsError::IoFailed { path: undo_path.display().to_string(), reason: e.to_string() })
    }
}

/// Helper struct for serializing undo history to disk.
#[derive(serde::Serialize)]
struct UndoHistoryFile<'a> {
    undo_stack: &'a UndoStack,
    done_ids: &'a VecDeque<ActionId>,
    undone_ids: &'a VecDeque<ActionId>,
    next_action_id: ActionId,
}

/// Helper struct for deserializing undo history from disk.
#[derive(serde::Deserialize)]
struct UndoHistoryFileOwned {
    undo_stack: UndoStack,
    done_ids: VecDeque<ActionId>,
    undone_ids: VecDeque<ActionId>,
    next_action_id: ActionId,
}

fn undo_path_for(template_path: &Path) -> PathBuf {
    template_path.with_extension("undo.json")
}

fn load_undo_history(path: &Path) -> Result<(UndoStack, VecDeque<ActionId>, VecDeque<ActionId>, ActionId), OpsError> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| OpsError::IoFailed { path: path.display().to_string(), reason: e.to_string() })?;
    let file: UndoHistoryFileOwned = serde_json::from_str(&data)
        .map_err(|e| OpsError::SerializeFailed { reason: e.to_string() })?;
    Ok((file.undo_stack, file.done_ids, file.undone_ids, file.next_action_id))
}
