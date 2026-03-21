// engine/editor/src-tauri/terminal/pty.rs
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;

use portable_pty::MasterPty;

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
    pub sessions: Mutex<HashMap<String, PtySession>>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self { sessions: Mutex::new(HashMap::new()) }
    }
}
