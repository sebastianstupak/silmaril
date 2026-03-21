//! Registry bridge — exposes the command registry to the MCP server and
//! broadcasts catalog updates to the frontend.

use tokio::sync::watch;
use tauri::Emitter;
use crate::bridge::registry::CommandSpec;

/// Start a background task that forwards command registry updates to the
/// frontend as `editor-catalog-updated` Tauri events.
///
/// Call this once from `lib.rs` during app setup, after all modules are registered.
/// The event payload is the full serialized `Vec<CommandSpec>`.
pub fn setup_registry_watch(
    mut rx: watch::Receiver<Vec<CommandSpec>>,
    app: tauri::AppHandle,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Wait for the next registry update.
            if rx.changed().await.is_err() {
                // Sender dropped — registry is gone, exit the task.
                break;
            }
            let specs = rx.borrow_and_update().clone();
            if let Err(e) = app.emit("editor-catalog-updated", &specs) {
                tracing::warn!(error = ?e, "Failed to emit editor-catalog-updated");
            }
        }
    });
}

/// Returns a standalone watch receiver for the MCP server to subscribe to.
///
/// NOTE: For the MCP Server plan (Plan 2), this function signature will change.
/// It will accept the Arc<Mutex<CommandRegistry>> and clone a receiver from it.
#[allow(dead_code)]
pub fn registry_watch_rx() -> watch::Receiver<Vec<CommandSpec>> {
    let (_tx, rx) = watch::channel(Vec::new());
    rx
}
