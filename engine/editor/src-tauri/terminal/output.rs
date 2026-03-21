// engine/editor/src-tauri/terminal/output.rs
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, State};
use tracing::{debug, warn};

/// Shared state for the output panel process runner.
pub struct OutputState {
    /// Currently running child process, if any.
    /// Wrapped in Arc so it can be shared with the waiter thread.
    pub child: Arc<Mutex<Option<Child>>>,
}

impl OutputState {
    pub fn new() -> Self {
        Self { child: Arc::new(Mutex::new(None)) }
    }
}

impl Default for OutputState {
    fn default() -> Self { Self::new() }
}

/// Spawns a cargo command with piped stdio.
///
/// Returns Err("already running") if a process is already active.
/// Streams stdout/stderr lines as `output-data` events with `{ line, stream }`.
/// Emits `output-exit` with `{ code, cancelled }` when the process finishes.
#[tauri::command]
pub fn output_run(
    command: String,
    args: Vec<String>,
    app: AppHandle,
    state: State<OutputState>,
) -> Result<(), String> {
    // Check not already running
    {
        let guard = state.child.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Err("already running".into());
        }
    }

    // Use current dir as working directory (project not yet tracked in managed state)
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;

    debug!(command = %command, args = ?args, cwd = ?cwd, "output_run starting");

    // Spawn process
    let mut child = Command::new(&command)
        .args(&args)
        .current_dir(&cwd)
        .env("CARGO_TERM_COLOR", "always")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Store child in shared Arc for waiter thread + output_cancel
    let child_arc = Arc::clone(&state.child);
    {
        let mut guard = child_arc.lock().map_err(|e| e.to_string())?;
        *guard = Some(child);
    }

    // Stdout reader thread
    let app2 = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    app2.emit("output-data", serde_json::json!({ "line": l, "stream": "stdout" })).ok();
                }
                Err(_) => break,
            }
        }
    });

    // Stderr reader thread
    let app3 = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    app3.emit("output-data", serde_json::json!({ "line": l, "stream": "stderr" })).ok();
                }
                Err(_) => break,
            }
        }
    });

    // Waiter thread: polls child exit, emits output-exit when done
    let waiter_arc = Arc::clone(&state.child);
    let app4 = app.clone();
    std::thread::spawn(move || {
        loop {
            {
                let mut guard = waiter_arc.lock().unwrap();
                if guard.is_none() {
                    // output_cancel took the child — it already emitted output-exit
                    break;
                }
                if let Some(c) = guard.as_mut() {
                    match c.try_wait() {
                        Ok(Some(status)) => {
                            *guard = None;
                            drop(guard);
                            debug!(code = ?status.code(), "output process exited");
                            app4.emit("output-exit", serde_json::json!({
                                "code": status.code(),
                                "cancelled": false
                            })).ok();
                            break;
                        }
                        Ok(None) => {} // still running
                        Err(e) => {
                            warn!(error = %e, "output process wait failed");
                            *guard = None;
                            drop(guard);
                            app4.emit("output-exit", serde_json::json!({
                                "code": null,
                                "cancelled": false
                            })).ok();
                            return;
                        }
                    }
                }
            } // lock released here
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    Ok(())
}

/// Kills the running cargo process if any.
/// No-op if nothing is running.
/// Emits output-exit with cancelled=true only if a process was actually killed.
#[tauri::command]
pub fn output_cancel(
    app: AppHandle,
    state: State<OutputState>,
) {
    let child_opt = {
        let mut guard = state.child.lock().unwrap_or_else(|e| e.into_inner());
        guard.take()
    };
    if let Some(mut child) = child_opt {
        let _ = child.kill();
        debug!("output process cancelled");
        app.emit("output-exit", serde_json::json!({
            "code": null,
            "cancelled": true
        })).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_state_starts_empty() {
        let state = OutputState::new();
        let guard = state.child.lock().unwrap();
        assert!(guard.is_none());
    }

    #[test]
    fn output_state_can_store_and_take_child() {
        let state = OutputState::new();
        let child = if cfg!(windows) {
            std::process::Command::new("cmd")
                .args(["/C", "echo hello"])
                .spawn()
                .expect("cmd must be available on Windows")
        } else {
            std::process::Command::new("sh")
                .args(["-c", "echo hello"])
                .spawn()
                .expect("sh must be available on Unix")
        };

        {
            let mut guard = state.child.lock().unwrap();
            *guard = Some(child);
        }

        let taken = state.child.lock().unwrap().take();
        assert!(taken.is_some());

        let taken_again = state.child.lock().unwrap().take();
        assert!(taken_again.is_none());
    }
}
