// Stub — implemented in Task 4

pub struct FileWatcherState;

#[tauri::command]
pub fn start_file_watch(_path: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub fn stop_file_watch() {}
