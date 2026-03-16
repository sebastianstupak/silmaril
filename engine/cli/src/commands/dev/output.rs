//! Multiplexes server, client, and build output to a single terminal stream.
//! All sources send `OutputLine` over an mpsc channel; a single task drains it.
#![allow(dead_code)]

use colored::Colorize;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Which process produced a line.
#[derive(Debug, Clone, Copy)]
pub enum Source {
    Server,
    Client,
    Build,
    Dev,
}

#[derive(Debug, Clone)]
struct OutputLine {
    source: Source,
    text: String,
}

/// Receives output lines and writes them with prefixes and colors to stdout.
pub struct OutputMux {
    tx: mpsc::Sender<Option<OutputLine>>,
    rx: mpsc::Receiver<Option<OutputLine>>,
}

/// Cloneable sender handle for submitting lines to `OutputMux`.
#[derive(Clone)]
pub struct OutputSender {
    tx: mpsc::Sender<Option<OutputLine>>,
}

impl OutputSender {
    /// Send a line from the given source.
    pub async fn send(&self, source: Source, text: impl Into<String>) {
        let _ = self
            .tx
            .send(Some(OutputLine {
                source,
                text: text.into(),
            }))
            .await;
    }

    /// Signal the mux to stop draining.
    pub async fn close(&self) {
        let _ = self.tx.send(None).await;
    }
}

impl OutputMux {
    /// Create a new mux with a 512-item buffer.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(512);
        Self { tx, rx }
    }

    /// Get a cloneable sender for this mux.
    pub fn sender(&self) -> OutputSender {
        OutputSender {
            tx: self.tx.clone(),
        }
    }

    /// Run the mux, writing prefixed lines to real stdout.
    pub async fn run(mut self) {
        while let Some(item) = self.rx.recv().await {
            match item {
                None => break,
                Some(line) => print!("{}", Self::format(&line)),
            }
        }
    }

    /// Test helper: run and capture formatted lines instead of printing.
    pub async fn run_capturing(mut self, out: Arc<Mutex<Vec<String>>>) {
        while let Some(item) = self.rx.recv().await {
            match item {
                None => break,
                Some(line) => {
                    let formatted = Self::format(&line);
                    out.lock().await.push(formatted);
                }
            }
        }
    }

    fn format(line: &OutputLine) -> String {
        match line.source {
            Source::Server => format!("{} {}\n", "[server]".blue().bold(), line.text),
            Source::Client => format!("{} {}\n", "[client]".green().bold(), line.text),
            Source::Build => format!("{} {}\n", "[build]".yellow().bold(), line.text),
            Source::Dev => format!("{} {}\n", "[dev]".cyan().bold(), line.text),
        }
    }
}

impl Default for OutputMux {
    fn default() -> Self {
        Self::new()
    }
}
