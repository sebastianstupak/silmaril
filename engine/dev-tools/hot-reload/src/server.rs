//! In-process TCP server that receives reload signals from `silm dev`.
//!
//! Compiled unconditionally; the `dev` feature controls whether `start()`
//! actually binds a port. Call sites require no `#[cfg]`.

use crate::force_reload::ForceReloader;
use std::sync::Arc;

/// TCP server that handles reload signals inside a game process.
///
/// Start with `DevReloadServer::start(reloader).await`.
/// No-op when the `dev` feature is disabled.
pub struct DevReloadServer;

impl DevReloadServer {
    /// Start the reload server.
    ///
    /// - With `dev` feature: binds `SILMARIL_DEV_PORT` (default 9999), serves until process exit.
    /// - Without `dev` feature: returns immediately (no-op).
    ///
    /// `reloader` is `Some` when hot-reload is active, `None` for a no-op start.
    pub async fn start(reloader: Option<Arc<ForceReloader>>) {
        #[cfg(feature = "dev")]
        {
            if let Some(r) = reloader {
                if let Err(e) = Self::serve(r).await {
                    tracing::warn!(error = ?e, "DevReloadServer failed to start");
                }
            }
        }
        #[cfg(not(feature = "dev"))]
        {
            let _ = reloader;
        }
    }

    #[cfg(feature = "dev")]
    async fn serve(reloader: Arc<ForceReloader>) -> Result<(), crate::error::DevError> {
        use crate::messages::ReloadMessage;
        use tokio::io::{AsyncBufReadExt, BufReader};
        use tokio::net::TcpListener;
        use tracing::{debug, info, warn};

        let port: u16 = std::env::var("SILMARIL_DEV_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(9999);

        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .map_err(|_| crate::error::DevError::PortBindFailed { port })?;

        info!(port, "DevReloadServer listening");

        loop {
            let (stream, addr) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = ?e, "DevReloadServer accept error");
                    continue;
                }
            };
            debug!(%addr, "DevReloadServer connection");
            let reloader = reloader.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut lines = BufReader::new(reader).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    match serde_json::from_str::<ReloadMessage>(&line) {
                        Ok(msg) => Self::handle(msg, &reloader, &mut writer).await,
                        Err(e) => warn!(error = ?e, line, "invalid reload message"),
                    }
                }
            });
        }
    }

    #[cfg(feature = "dev")]
    async fn handle(
        msg: crate::messages::ReloadMessage,
        reloader: &Arc<ForceReloader>,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
    ) {
        use crate::messages::ReloadMessage;
        use std::path::PathBuf;
        use tokio::io::AsyncWriteExt;
        use tracing::{debug, info, warn};

        match msg {
            ReloadMessage::ReloadAsset { path } => {
                if let Err(e) = reloader.reload(&path) {
                    warn!(error = ?e, path, "asset reload failed");
                } else {
                    info!(path, "asset reload queued");
                }
            }
            ReloadMessage::ReloadConfig { path } => {
                info!(path, "config reload requested");
            }
            ReloadMessage::SerializeState => {
                let project_root = std::env::var("SILMARIL_PROJECT_ROOT")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
                // Handoff object created here for future Task 12 wiring (world serialization).
                // The path is captured now so StateHandoff::save() can be called once the
                // game loop channel is plumbed in.
                let _handoff = crate::handoff::StateHandoff::new(&project_root);
                info!("SerializeState requested — world serialization via game loop channel (wired in Task 12)");
                let ack = serde_json::to_string(&ReloadMessage::Ack).unwrap() + "\n";
                let _ = writer.write_all(ack.as_bytes()).await;
            }
            ReloadMessage::Ack => {
                debug!("received Ack (unexpected in server)");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_none_returns_immediately() {
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            DevReloadServer::start(None),
        )
        .await;
        assert!(result.is_ok(), "start(None) should return immediately within 100ms");
    }
}
