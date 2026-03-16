//! Watches project directories for file changes and classifies them.
//!
//! Uses `notify-debouncer-full` with debounce windows:
//! - Code changes (.rs, Cargo.toml): 500ms
//! - Asset/config changes: 200ms

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::warn;

/// The kind of change detected.
#[derive(Debug, Clone)]
pub enum ChangeKind {
    /// A source file changed in a known crate.
    Code { crate_name: String },
    /// An asset file changed.
    Asset { path: PathBuf },
    /// A config file changed.
    Config { path: PathBuf },
}

/// A file change event.
#[derive(Debug, Clone)]
pub struct FileChange {
    pub kind: ChangeKind,
    pub path: PathBuf,
    pub timestamp: std::time::Instant,
}

/// Classify a path by its location and extension.
///
/// Returns `None` if the path is not in a watched directory or has an
/// unrecognized extension.
///
/// # Example
/// ```
/// use std::path::PathBuf;
/// use silm::commands::dev::watcher::{classify_path, ChangeKind};
///
/// let path = PathBuf::from("shared/src/lib.rs");
/// assert!(matches!(classify_path(&path), Some(ChangeKind::Code { .. })));
/// ```
pub fn classify_path(path: &Path) -> Option<ChangeKind> {
    let path_str = path.to_string_lossy();
    let path_str = path_str.replace('\\', "/");

    // Code: .rs files or Cargo.toml in src directories
    let is_code_dir = path_str.starts_with("shared/")
        || path_str.starts_with("server/")
        || path_str.starts_with("client/");
    let is_code_ext = path_str.ends_with(".rs") || path_str.ends_with("Cargo.toml");

    if is_code_dir && is_code_ext {
        let crate_name = if path_str.starts_with("shared/") {
            "shared"
        } else if path_str.starts_with("server/") {
            "server"
        } else {
            "client"
        };
        return Some(ChangeKind::Code {
            crate_name: crate_name.to_string(),
        });
    }

    // Assets
    if path_str.starts_with("assets/") {
        let asset_exts = [
            ".png", ".jpg", ".jpeg", ".obj", ".gltf", ".glb", ".ogg", ".wav", ".mp3",
        ];
        if asset_exts.iter().any(|ext| path_str.ends_with(ext)) {
            return Some(ChangeKind::Asset {
                path: path.to_path_buf(),
            });
        }
    }

    // Config
    if path_str.starts_with("config/") && path_str.ends_with(".ron") {
        return Some(ChangeKind::Config {
            path: path.to_path_buf(),
        });
    }

    None
}

/// Watches project directories for changes.
///
/// Call [`FileWatcher::start`] to begin watching. Events are sent to the
/// [`mpsc::Receiver<FileChange>`] returned from [`FileWatcher::new`].
pub struct FileWatcher {
    project_root: PathBuf,
}

impl FileWatcher {
    /// Create a new `FileWatcher` rooted at `project_root`.
    ///
    /// Returns the watcher and a channel receiver for [`FileChange`] events.
    pub fn new(project_root: PathBuf) -> (Self, mpsc::Receiver<FileChange>) {
        let (_tx, rx) = mpsc::channel(256);
        let watcher = Self { project_root };
        (watcher, rx)
    }

    /// Start watching in a background thread.
    ///
    /// The `tx` sender should be obtained from a separate `mpsc::channel` call
    /// or cloned from the one returned by [`FileWatcher::new`].
    ///
    /// Non-existent directories are skipped with a warning (graceful degradation).
    pub fn start(self, tx: mpsc::Sender<FileChange>) -> anyhow::Result<()> {
        use notify_debouncer_full::{
            new_debouncer,
            notify::{RecursiveMode, Watcher},
        };

        let project_root = self.project_root.clone();
        let tx_code = tx.clone();
        let tx_ac = tx;

        std::thread::spawn(move || {
            let (std_tx_code, std_rx_code) = std::sync::mpsc::channel();
            let (std_tx_ac, std_rx_ac) = std::sync::mpsc::channel();

            let mut code_debouncer =
                match new_debouncer(Duration::from_millis(500), None, std_tx_code) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!(error = ?e, "failed to create code file watcher");
                        return;
                    }
                };

            let mut ac_debouncer =
                match new_debouncer(Duration::from_millis(200), None, std_tx_ac) {
                    Ok(d) => d,
                    Err(e) => {
                        warn!(error = ?e, "failed to create asset/config file watcher");
                        return;
                    }
                };

            // Watch code directories
            for dir in ["shared", "server", "client"] {
                let path = project_root.join(dir);
                if path.exists() {
                    if let Err(e) = code_debouncer
                        .watcher()
                        .watch(&path, RecursiveMode::Recursive)
                    {
                        warn!(error = ?e, dir, "could not watch code directory");
                    }
                }
            }

            // Watch assets and config
            for dir in ["assets", "config"] {
                let path = project_root.join(dir);
                if path.exists() {
                    if let Err(e) =
                        ac_debouncer.watcher().watch(&path, RecursiveMode::Recursive)
                    {
                        warn!(error = ?e, dir, "could not watch assets/config directory");
                    }
                }
            }

            // Bridge std::sync::mpsc → tokio::sync::mpsc via polling loop
            loop {
                // Drain code events
                while let Ok(result) = std_rx_code.try_recv() {
                    match result {
                        Ok(events) => {
                            for event in events {
                                for path in &event.paths {
                                    let rel =
                                        path.strip_prefix(&project_root).unwrap_or(path.as_path());
                                    if let Some(kind) = classify_path(rel) {
                                        let change = FileChange {
                                            kind,
                                            path: path.clone(),
                                            timestamp: std::time::Instant::now(),
                                        };
                                        let _ = tx_code.blocking_send(change);
                                    }
                                }
                            }
                        }
                        Err(errors) => {
                            for e in errors {
                                warn!(error = ?e, "code file watcher error");
                            }
                        }
                    }
                }

                // Drain asset/config events
                while let Ok(result) = std_rx_ac.try_recv() {
                    match result {
                        Ok(events) => {
                            for event in events {
                                for path in &event.paths {
                                    let rel =
                                        path.strip_prefix(&project_root).unwrap_or(path.as_path());
                                    if let Some(kind) = classify_path(rel) {
                                        let change = FileChange {
                                            kind,
                                            path: path.clone(),
                                            timestamp: std::time::Instant::now(),
                                        };
                                        let _ = tx_ac.blocking_send(change);
                                    }
                                }
                            }
                        }
                        Err(errors) => {
                            for e in errors {
                                warn!(error = ?e, "asset/config file watcher error");
                            }
                        }
                    }
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        });

        Ok(())
    }
}
