//! Silmaril shared operation layer.
//!
//! This crate contains all game project operations shared between the CLI (`silm`)
//! and the editor. Both frontends are thin wrappers over these operations.
//!
//! # Modules
//!
//! - [`project`] — Project creation, discovery, and configuration
//! - [`codegen`] — Component and system code generation
//! - [`module`] — Module management (add, remove, list)
//! - [`build`] — Platform builds and packaging
//! - [`undo`] — Undo/redo command pattern
//! - [`scene`] — Scene save/load (YAML + Bincode)

pub mod build;
pub mod codegen;
pub mod module;
pub mod project;
pub mod scene;
pub mod undo;

/// Trait for reporting operation progress to frontends.
///
/// CLI implements this with terminal spinners (indicatif).
/// Editor implements this by emitting Tauri events.
/// Tests can use [`NoopProgress`] or [`CollectorProgress`].
pub trait ProgressSink: Send + Sync {
    fn on_start(&self, operation: &str, total_steps: usize);
    fn on_step(&self, operation: &str, step: usize, message: &str);
    fn on_done(&self, operation: &str, success: bool);
}

/// No-op progress sink for tests and silent operations.
pub struct NoopProgress;

impl ProgressSink for NoopProgress {
    fn on_start(&self, _operation: &str, _total_steps: usize) {}
    fn on_step(&self, _operation: &str, _step: usize, _message: &str) {}
    fn on_done(&self, _operation: &str, _success: bool) {}
}

/// Collecting progress sink for tests — stores all events.
#[cfg(test)]
pub struct CollectorProgress {
    pub events: std::sync::Mutex<Vec<(String, String)>>,
}

#[cfg(test)]
impl CollectorProgress {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[cfg(test)]
impl ProgressSink for CollectorProgress {
    fn on_start(&self, operation: &str, _total_steps: usize) {
        self.events
            .lock()
            .unwrap()
            .push((operation.to_string(), "start".to_string()));
    }
    fn on_step(&self, operation: &str, _step: usize, message: &str) {
        self.events
            .lock()
            .unwrap()
            .push((operation.to_string(), message.to_string()));
    }
    fn on_done(&self, operation: &str, success: bool) {
        self.events
            .lock()
            .unwrap()
            .push((operation.to_string(), format!("done:{}", success)));
    }
}
