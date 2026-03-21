//! Tauri IPC handlers for template CRUD and undo/redo.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use engine_ops::command::TemplateCommand;
use engine_ops::ipc::{ActionId, ActionSummary, CommandResult, IpcError};
use engine_ops::processor::CommandProcessor;
use engine_ops::template::TemplateState;
use tauri::State;

/// Global map of open template files → CommandProcessor.
pub struct EditorState {
    pub processors: HashMap<PathBuf, CommandProcessor>,
}

impl EditorState {
    pub fn new() -> Self {
        Self { processors: HashMap::new() }
    }
}

fn get_processor<'a>(
    map: &'a mut HashMap<PathBuf, CommandProcessor>,
    template_path: &str,
) -> Result<&'a mut CommandProcessor, IpcError> {
    let path = PathBuf::from(template_path);
    map.get_mut(&path).ok_or_else(|| IpcError {
        code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
        message: format!("Template not open: {template_path}"),
    })
}

/// Opens a template file and registers its [`CommandProcessor`].
pub fn template_open_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<TemplateState, IpcError> {
    let path = PathBuf::from(&template_path);
    let processor = CommandProcessor::load(path.clone()).map_err(IpcError::from)?;
    let result = processor.state_ref().clone();
    state.lock().unwrap().processors.insert(path, processor);
    Ok(result)
}

#[tauri::command]
pub fn template_open(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<TemplateState, IpcError> {
    template_open_inner(&state, template_path)
}

/// Closes a template and removes its processor from the active set.
pub fn template_close_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<(), IpcError> {
    let path = PathBuf::from(&template_path);
    state.lock().unwrap().processors.remove(&path);
    Ok(())
}

#[tauri::command]
pub fn template_close(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<(), IpcError> {
    template_close_inner(&state, template_path)
}

/// Executes a [`TemplateCommand`] and records it in the undo history.
pub fn template_execute_inner(
    state: &Mutex<EditorState>,
    template_path: String,
    command: TemplateCommand,
) -> Result<CommandResult, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.execute(command).map_err(IpcError::from)
}

#[tauri::command]
pub fn template_execute(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
    command: TemplateCommand,
) -> Result<CommandResult, IpcError> {
    template_execute_inner(&state, template_path, command)
}

/// Undoes the last command on the given template, returning the undone [`ActionId`].
pub fn template_undo_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.undo().map_err(IpcError::from)
}

#[tauri::command]
pub fn template_undo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    template_undo_inner(&state, template_path)
}

/// Redoes the last undone command on the given template, returning the redone [`ActionId`].
pub fn template_redo_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    let mut guard = state.lock().unwrap();
    let proc = get_processor(&mut guard.processors, &template_path)?;
    proc.redo().map_err(IpcError::from)
}

#[tauri::command]
pub fn template_redo(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Option<ActionId>, IpcError> {
    template_redo_inner(&state, template_path)
}

/// Returns a summary of all recorded actions for the given template.
pub fn template_history_inner(
    state: &Mutex<EditorState>,
    template_path: String,
) -> Result<Vec<ActionSummary>, IpcError> {
    let guard = state.lock().unwrap();
    let path = PathBuf::from(&template_path);
    let proc = guard.processors.get(&path).ok_or_else(|| IpcError {
        code: engine_core::error::ErrorCode::TemplateNoTemplateOpen as u32,
        message: format!("Template not open: {template_path}"),
    })?;
    Ok(proc.history_summaries())
}

#[tauri::command]
pub fn template_history(
    state: State<'_, Mutex<EditorState>>,
    template_path: String,
) -> Result<Vec<ActionSummary>, IpcError> {
    template_history_inner(&state, template_path)
}
