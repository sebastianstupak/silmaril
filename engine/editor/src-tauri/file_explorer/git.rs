// Stub — implemented in Task 3

#[tauri::command]
pub fn get_git_status(_path: String) -> Result<Vec<(String, String)>, String> {
    Ok(vec![])
}
