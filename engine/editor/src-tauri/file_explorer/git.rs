// Stub — implemented in Task 3

use std::collections::HashMap;
use crate::file_explorer::tree::GitStatus;

#[tauri::command]
pub fn get_git_status(_root: String) -> HashMap<String, GitStatus> {
    HashMap::new()
}
