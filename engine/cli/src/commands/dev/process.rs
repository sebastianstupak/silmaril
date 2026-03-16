//! Process lifecycle management for `silm dev`.
//!
//! [`ProcessManager`] starts, monitors, and restarts game processes.
//! [`ProcessKiller`] trait abstracts graceful termination per platform.
//!
//! # Example
//! ```no_run
//! use silm::commands::dev::process::{ProcessManager, ProcessState};
//!
//! let manager = ProcessManager::new("my-server".to_string(), 9000);
//! assert!(matches!(manager.state(), ProcessState::Stopped));
//! ```

#![allow(dead_code)]

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::process::Child;
use tracing::{info, warn};

use crate::commands::dev::output::{OutputSender, Source};

// ────────────────────────────────────────────────────────────────────────────
// ProcessKiller trait
// ────────────────────────────────────────────────────────────────────────────

/// Platform-abstracted graceful process termination.
///
/// Call [`kill_graceful`] to send a termination signal and wait for exit.
/// If the process doesn't exit within `timeout`, it is force-killed.
#[async_trait]
pub trait ProcessKiller: Send + Sync {
    /// Send a graceful termination signal to `child`, then wait up to `timeout`.
    ///
    /// If the process doesn't exit in time, force-kills it.
    async fn kill_graceful(&self, child: &mut Child, timeout: Duration) -> Result<()>;
}

// ────────────────────────────────────────────────────────────────────────────
// Unix implementation
// ────────────────────────────────────────────────────────────────────────────

#[cfg(unix)]
struct UnixKiller;

#[cfg(unix)]
#[async_trait]
impl ProcessKiller for UnixKiller {
    async fn kill_graceful(&self, child: &mut Child, timeout: Duration) -> Result<()> {
        use std::os::unix::process::ExitStatusExt;

        // Send SIGTERM via libc to allow graceful shutdown.
        if let Some(pid) = child.id() {
            // SAFETY: pid is a valid process ID from a live child.
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }

        match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => warn!(error = ?e, "error waiting for child after SIGTERM"),
            Err(_elapsed) => {
                warn!("graceful shutdown timed out, force-killing");
                child.kill().await.ok();
            }
        }
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Windows implementation
// ────────────────────────────────────────────────────────────────────────────

#[cfg(windows)]
struct WindowsKiller;

#[cfg(windows)]
#[async_trait]
impl ProcessKiller for WindowsKiller {
    async fn kill_graceful(&self, child: &mut Child, timeout: Duration) -> Result<()> {
        // TODO: Use GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid) from the
        // `windows` crate for a true graceful shutdown on Windows.
        // For now, attempt a timed wait then force-kill.
        match tokio::time::timeout(timeout, child.wait()).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => warn!(error = ?e, "error waiting for child process"),
            Err(_elapsed) => {
                warn!("graceful shutdown timed out, force-killing");
                child.kill().await.ok();
            }
        }
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Fallback (neither unix nor windows)
// ────────────────────────────────────────────────────────────────────────────

#[cfg(not(any(unix, windows)))]
struct FallbackKiller;

#[cfg(not(any(unix, windows)))]
#[async_trait]
impl ProcessKiller for FallbackKiller {
    async fn kill_graceful(&self, child: &mut Child, _timeout: Duration) -> Result<()> {
        child.kill().await.ok();
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Factory
// ────────────────────────────────────────────────────────────────────────────

/// Returns the platform-appropriate [`ProcessKiller`].
pub fn create_killer() -> Box<dyn ProcessKiller> {
    #[cfg(unix)]
    return Box::new(UnixKiller);

    #[cfg(windows)]
    return Box::new(WindowsKiller);

    #[cfg(not(any(unix, windows)))]
    return Box::new(FallbackKiller);
}

// ────────────────────────────────────────────────────────────────────────────
// ProcessState
// ────────────────────────────────────────────────────────────────────────────

/// State of a [`ProcessManager`]-managed process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// No child process is running.
    Stopped,
    /// Process is being launched (cargo run in progress).
    Starting,
    /// Process is running and healthy.
    Running,
    /// Process is being stopped prior to a restart.
    Restarting,
}

// ────────────────────────────────────────────────────────────────────────────
// ProcessManager
// ────────────────────────────────────────────────────────────────────────────

/// Manages a single game process (server or client) for `silm dev`.
///
/// Handles start, stop, and rebuild-then-restart lifecycle with structured
/// logging and output forwarding via [`OutputSender`].
pub struct ProcessManager {
    package_name: String,
    dev_port: u16,
    state: ProcessState,
    child: Option<Child>,
    killer: Box<dyn ProcessKiller>,
}

impl ProcessManager {
    /// Create a new manager for `package_name` that will bind to `dev_port`.
    pub fn new(package_name: String, dev_port: u16) -> Self {
        Self {
            package_name,
            dev_port,
            state: ProcessState::Stopped,
            child: None,
            killer: create_killer(),
        }
    }

    /// Current lifecycle state.
    pub fn state(&self) -> ProcessState {
        self.state
    }

    /// Package name this manager was created for.
    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    /// Launch the process with `cargo run --features dev`.
    ///
    /// Stdout and stderr are forwarded to `output_tx` with the given `source`
    /// label. The `.silmaril/` directory is created under `project_root` if
    /// it doesn't exist yet.
    pub async fn start(
        &mut self,
        project_root: &std::path::Path,
        output_tx: OutputSender,
        source: Source,
    ) -> Result<()> {
        use tokio::process::Command;

        self.state = ProcessState::Starting;

        let silmaril_dir = project_root.join(".silmaril");
        std::fs::create_dir_all(&silmaril_dir)
            .with_context(|| format!("could not create {}", silmaril_dir.display()))?;

        info!(package = %self.package_name, port = self.dev_port, "starting process");

        let mut cmd = Command::new("cargo");
        cmd.arg("run")
            .arg("--features")
            .arg("dev")
            .arg("--package")
            .arg(&self.package_name)
            .current_dir(project_root)
            .env("SILMARIL_DEV_PORT", self.dev_port.to_string())
            .env(
                "SILMARIL_PROJECT_ROOT",
                project_root.to_string_lossy().as_ref(),
            )
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = cmd.spawn().with_context(|| {
            format!(
                "failed to spawn `cargo run --package {}`",
                self.package_name
            )
        })?;

        if let Some(stdout) = child.stdout.take() {
            let tx = output_tx.clone();
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tx.send(source, line).await;
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let tx = output_tx.clone();
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tx.send(source, line).await;
                }
            });
        }

        self.child = Some(child);
        self.state = ProcessState::Running;
        Ok(())
    }

    /// Gracefully stop the running process (2-second timeout).
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            self.state = ProcessState::Restarting;
            info!(package = %self.package_name, "stopping process");
            self.killer
                .kill_graceful(&mut child, Duration::from_secs(2))
                .await?;
        }
        self.state = ProcessState::Stopped;
        Ok(())
    }

    /// Build the package, then restart the process.
    ///
    /// Stops the running process first. If `cargo build` fails, the manager
    /// remains in [`ProcessState::Stopped`] and a warning is logged.
    pub async fn rebuild_and_restart(
        &mut self,
        project_root: &std::path::Path,
        output_tx: OutputSender,
        source: Source,
    ) -> Result<()> {
        use tokio::process::Command;

        self.stop().await?;

        info!(package = %self.package_name, "rebuilding");

        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .arg("--features")
            .arg("dev")
            .arg("--package")
            .arg(&self.package_name)
            .current_dir(project_root)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut build_child = cmd.spawn().context("failed to spawn `cargo build`")?;

        if let Some(stdout) = build_child.stdout.take() {
            let tx = output_tx.clone();
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tx.send(Source::Build, line).await;
                }
            });
        }

        if let Some(stderr) = build_child.stderr.take() {
            let tx = output_tx.clone();
            tokio::spawn(async move {
                let mut lines = tokio::io::BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    tx.send(Source::Build, line).await;
                }
            });
        }

        let status = build_child
            .wait()
            .await
            .context("`cargo build` wait failed")?;

        if !status.success() {
            warn!(package = %self.package_name, "build failed — not restarting");
            self.state = ProcessState::Stopped;
            return Ok(());
        }

        self.start(project_root, output_tx, source).await
    }
}
