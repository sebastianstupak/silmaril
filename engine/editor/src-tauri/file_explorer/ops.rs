// Stub — implemented in Task 5

#[tauri::command]
pub fn create_dir(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn create_file(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn delete_path(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn open_in_editor(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn rename_path(_from: String, _to: String) -> Result<(), String> {
    Ok(())
}
