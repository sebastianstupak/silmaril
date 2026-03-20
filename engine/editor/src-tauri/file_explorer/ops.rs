use std::path::Path;

// ── Internal helpers (testable without Tauri) ──────────────────────────────

pub fn do_create_file(path: &str) -> Result<(), String> {
    std::fs::File::create(Path::new(path))
        .map(|_| ())
        .map_err(|e| format!("Cannot create file: {e}"))
}

pub fn do_create_dir(path: &str) -> Result<(), String> {
    std::fs::create_dir_all(Path::new(path))
        .map_err(|e| format!("Cannot create directory: {e}"))
}

pub fn do_rename_path(from: &str, to: &str) -> Result<(), String> {
    std::fs::rename(Path::new(from), Path::new(to))
        .map_err(|e| format!("Cannot rename: {e}"))
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn open_in_editor(path: String) -> Result<(), String> {
    // Try $EDITOR, then $VISUAL, then OS default
    if let Ok(editor) = std::env::var("EDITOR") {
        if std::process::Command::new(&editor).arg(&path).spawn().is_ok() {
            return Ok(());
        }
    }
    if let Ok(editor) = std::env::var("VISUAL") {
        if std::process::Command::new(&editor).arg(&path).spawn().is_ok() {
            return Ok(());
        }
    }

    // OS default open
    #[cfg(target_os = "windows")]
    {
        return std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open file — configure an editor in Settings ({e})"));
    }
    #[cfg(target_os = "macos")]
    {
        return std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open file — configure an editor in Settings ({e})"));
    }
    #[cfg(target_os = "linux")]
    {
        return std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Could not open file — configure an editor in Settings ({e})"));
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    Err("Open in editor not supported on this platform".to_string())
}

#[tauri::command]
pub fn create_file(path: String) -> Result<(), String> {
    do_create_file(&path)
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), String> {
    do_create_dir(&path)
}

#[tauri::command]
pub fn rename_path(from: String, to: String) -> Result<(), String> {
    do_rename_path(&from, &to)
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    trash::delete(Path::new(&path))
        .map_err(|e| format!("Cannot delete: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_do_create_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("new.txt").to_string_lossy().into_owned();
        do_create_file(&path).unwrap();
        assert!(std::path::Path::new(&path).exists());
    }

    #[test]
    fn test_do_create_dir() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("newdir").to_string_lossy().into_owned();
        do_create_dir(&path).unwrap();
        assert!(std::path::Path::new(&path).is_dir());
    }

    #[test]
    fn test_do_rename_path() {
        let dir = TempDir::new().unwrap();
        let from = dir.path().join("old.txt").to_string_lossy().into_owned();
        let to = dir.path().join("new.txt").to_string_lossy().into_owned();
        std::fs::write(&from, "").unwrap();
        do_rename_path(&from, &to).unwrap();
        assert!(!std::path::Path::new(&from).exists());
        assert!(std::path::Path::new(&to).exists());
    }
}
