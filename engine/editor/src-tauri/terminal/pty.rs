// engine/editor/src-tauri/terminal/pty.rs
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, State};
use tracing::{debug, warn};

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
            debug!(shell = ?path, "resolved shell");
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
                Ok(0) => {
                    app_handle
                        .emit(&format!("terminal-exit:{tab_id_clone}"), ())
                        .ok();
                    // Session map cleanup is the responsibility of terminal_close_tab,
                    // which the frontend must call after receiving terminal-exit.
                    break;
                }
                Err(e) => {
                    warn!(tab_id = %tab_id_clone, error = %e, "PTY read error, closing tab");
                    app_handle
                        .emit(&format!("terminal-exit:{tab_id_clone}"), ())
                        .ok();
                    // Session map cleanup is the responsibility of terminal_close_tab,
                    // which the frontend must call after receiving terminal-exit.
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

    debug!(tab_id = %tab_id, "terminal tab opened");
    Ok(tab_id)
}

/// Writes keystroke data to the PTY session.
/// Silent no-op if session already closed (tab close race — expected).
#[tauri::command]
pub fn terminal_write(
    tab_id: String,
    data: String,
    state: State<TerminalState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(session_arc) = sessions.get(&tab_id) {
        let mut session = session_arc.lock().map_err(|e| e.to_string())?;
        session.writer.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Resizes the PTY for the given tab.
#[tauri::command]
pub fn terminal_resize(
    tab_id: String,
    cols: u16,
    rows: u16,
    state: State<TerminalState>,
) -> Result<(), String> {
    let sessions = state.sessions.lock().map_err(|e| e.to_string())?;
    if let Some(session_arc) = sessions.get(&tab_id) {
        let session = session_arc.lock().map_err(|e| e.to_string())?;
        session
            .master
            .resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Kills the shell process and removes the session from the map.
/// No-op if tab already closed.
#[tauri::command]
pub fn terminal_close_tab(tab_id: String, state: State<TerminalState>) {
    // Extract the Arc while holding the outer lock, then release it
    // before acquiring the inner lock and calling kill() — avoids
    // blocking all other tab operations during the kill syscall.
    let session_arc = {
        let mut sessions = state.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.remove(&tab_id)
    };
    if let Some(arc) = session_arc {
        if let Ok(mut session) = arc.lock() {
            let _ = session.child.kill();
        }
        // If the inner mutex is poisoned the child goes un-killed until process exit.
        debug!(tab_id = %tab_id, "terminal tab closed");
    }
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
