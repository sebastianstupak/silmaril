// engine/editor/src-tauri/terminal/pty.rs
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, State};

/// One active PTY session per terminal tab.
pub struct PtySession {
    /// Master PTY — kept for resize calls.
    pub master: Box<dyn MasterPty + Send>,
    /// Write side — forwarded from master via `take_writer`.
    pub writer: Box<dyn Write + Send>,
    /// Shell process handle — for kill on close.
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct TerminalState {
    pub sessions: Mutex<HashMap<String, Arc<Mutex<PtySession>>>>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self { sessions: Mutex::new(HashMap::new()) }
    }
}

impl Default for TerminalState {
    fn default() -> Self { Self::new() }
}

/// Resolves the PowerShell executable path.
/// Tries pwsh (PowerShell 7+) first, then powershell.exe (Windows built-in).
fn resolve_shell() -> PathBuf {
    for candidate in &["pwsh", "powershell.exe"] {
        if let Ok(path) = which::which(candidate) {
            return path;
        }
    }
    // Fallback — will error at spawn if not found
    PathBuf::from("powershell.exe")
}

/// Spawns a new PowerShell PTY session. Returns the tab_id on success.
#[tauri::command]
pub fn terminal_new_tab(
    app: AppHandle,
    state: State<TerminalState>,
) -> Result<String, String> {
    let tab_id = uuid::Uuid::new_v4().to_string();

    // Open PTY pair
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;

    // Spawn shell (use current dir as cwd)
    let mut cmd = CommandBuilder::new(resolve_shell());
    if let Ok(cwd) = std::env::current_dir() {
        cmd.cwd(&cwd);
    }
    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    // Drop slave after spawn so master detects EOF on child exit.
    drop(pair.slave);

    // Clone reader before taking writer (both come from master)
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    // Store session (inner Arc<Mutex<PtySession>>)
    {
        let mut sessions = state.sessions.lock().map_err(|e| e.to_string())?;
        sessions.insert(
            tab_id.clone(),
            Arc::new(Mutex::new(PtySession { master: pair.master, writer, child })),
        );
    }

    // Background reader thread: read PTY output, emit events
    let tab_id_clone = tab_id.clone();
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => {
                    app_handle
                        .emit(&format!("terminal-exit:{tab_id_clone}"), ())
                        .ok();
                    break;
                }
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                    app_handle
                        .emit(&format!("terminal-data:{tab_id_clone}"), data)
                        .ok();
                }
            }
        }
    });

    Ok(tab_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_shell_returns_existing_path() {
        let path = resolve_shell();
        assert!(
            path.exists(),
            "resolve_shell() returned {path:?} which does not exist on this system"
        );
    }
}
