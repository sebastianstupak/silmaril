// engine/editor/src-tauri/terminal/output.rs
use std::process::Child;
use std::sync::{Arc, Mutex};

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
