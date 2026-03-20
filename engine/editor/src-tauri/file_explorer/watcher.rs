use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub struct FileWatcherState {
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl FileWatcherState {
    pub fn new() -> Self {
        Self { watcher: Mutex::new(None) }
    }
}

#[tauri::command]
pub fn start_file_watch(
    root: String,
    app: AppHandle,
    state: tauri::State<'_, FileWatcherState>,
) -> Result<(), String> {
    let app_clone = app.clone();
    let root_clone = root.clone();

    // Debounce: track the last event time with an Arc<Mutex<Instant>>.
    // A single background thread re-checks after 300ms; if no newer event
    // has arrived it emits. This avoids spawning unbounded threads under
    // heavy FS activity (e.g. cargo build writing hundreds of files).
    let last_event: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let last_event_watcher = last_event.clone();
    let pending = Arc::new(Mutex::new(false));
    let pending_watcher = pending.clone();

    let mut watcher = RecommendedWatcher::new(
        move |_res: notify::Result<notify::Event>| {
            let mut last = last_event_watcher.lock().unwrap();
            *last = Some(Instant::now());
            let already_pending = {
                let mut p = pending_watcher.lock().unwrap();
                let was = *p;
                *p = true;
                was
            };
            if !already_pending {
                let app2 = app_clone.clone();
                let root2 = root_clone.clone();
                let last2 = last_event_watcher.clone();
                let pending2 = pending_watcher.clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(Duration::from_millis(300));
                    let elapsed = {
                        let guard = last2.lock().unwrap();
                        guard.map(|t| t.elapsed()).unwrap_or(Duration::MAX)
                    };
                    if elapsed >= Duration::from_millis(280) {
                        *pending2.lock().unwrap() = false;
                        let _ = app2.emit("file-tree-changed", serde_json::json!({ "root": root2 }));
                        break;
                    }
                });
            }
        },
        Config::default(),
    )
    .map_err(|e| format!("Failed to create watcher: {e}"))?;

    watcher
        .watch(Path::new(&root), RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch path: {e}"))?;

    *state.watcher.lock().unwrap() = Some(watcher);
    tracing::info!(root = %root, "File watcher started");
    Ok(())
}

#[tauri::command]
pub fn stop_file_watch(state: tauri::State<'_, FileWatcherState>) {
    *state.watcher.lock().unwrap() = None;
    tracing::info!("File watcher stopped");
}
