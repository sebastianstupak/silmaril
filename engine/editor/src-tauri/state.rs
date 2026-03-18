//! Editor state — selection, mode, open project.

/// Current editor interaction mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    /// Normal editing mode.
    Edit,
    /// Running the game within the editor.
    Play,
    /// Game is paused (can inspect state).
    Pause,
}

/// Top-level editor state.
pub struct EditorState {
    /// Current editor mode.
    pub mode: EditorMode,
    /// Currently selected entity, if any.
    pub selected_entity: Option<u64>,
    /// Path to the open project.
    pub project_path: Option<std::path::PathBuf>,
    /// Display name of the open project.
    pub project_name: Option<String>,
}

impl EditorState {
    /// Creates a new editor state with defaults.
    pub fn new() -> Self {
        Self {
            mode: EditorMode::Edit,
            selected_entity: None,
            project_path: None,
            project_name: None,
        }
    }
}
