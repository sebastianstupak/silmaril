//! TCP client that sends reload signals from `silm dev` to `DevReloadServer`.
//!
//! All operations are best-effort: connection failures are logged and treated
//! as non-fatal. Only `send_serialize_state` logs at `warn!` level on failure
//! (others at `debug!`), since state serialization failure means a clean restart.

use crate::messages::ReloadMessage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use tracing::{debug, warn};

/// Sends `ReloadMessage` commands to a running `DevReloadServer`.
pub struct ReloadClient {
    port: u16,
}

impl ReloadClient {
    /// Create a client targeting `localhost:<port>`.
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    /// Queue an asset reload. Best-effort — logs debug and returns `Ok` on failure.
    pub async fn send_reload_asset(&self, path: &str) -> Result<(), ()> {
        self.send_no_ack(ReloadMessage::ReloadAsset { path: path.to_string() })
            .await
    }

    /// Queue a config reload. Best-effort — logs debug and returns `Ok` on failure.
    pub async fn send_reload_config(&self, path: &str) -> Result<(), ()> {
        self.send_no_ack(ReloadMessage::ReloadConfig { path: path.to_string() })
            .await
    }

    /// Send `SerializeState` and wait for `Ack`. Retries connection up to 3 times.
    ///
    /// On failure (no server / timeout), logs `warn!` and returns `Ok(())` —
    /// the caller proceeds with a clean restart.
    pub async fn send_serialize_state(&self) -> Result<(), ()> {
        let mut stream = match self.connect_with_retry().await {
            Some(s) => s,
            None => {
                warn!(
                    port = self.port,
                    "could not reach process for state serialization — restarting with clean state"
                );
                return Ok(());
            }
        };

        let line = serde_json::to_string(&ReloadMessage::SerializeState).unwrap() + "\n";
        if stream.write_all(line.as_bytes()).await.is_err() {
            warn!(port = self.port, "serialize_state send failed — clean restart");
            return Ok(());
        }

        // Wait for Ack (up to 10s — world serialization may take time)
        let mut reader = BufReader::new(&mut stream);
        let mut response = String::new();
        match timeout(Duration::from_secs(10), reader.read_line(&mut response)).await {
            Ok(Ok(_)) => {
                if let Ok(ReloadMessage::Ack) = serde_json::from_str(response.trim()) {
                    debug!("SerializeState ack received");
                } else {
                    warn!("unexpected response to SerializeState");
                }
            }
            _ => warn!("timed out waiting for SerializeState ack — clean restart"),
        }
        Ok(())
    }

    /// Send a message with no ack expected. Best-effort — logs debug on failure.
    async fn send_no_ack(&self, msg: ReloadMessage) -> Result<(), ()> {
        let Some(mut stream) = self.connect_with_retry().await else {
            debug!(port = self.port, "no server for reload signal — skipping");
            return Ok(());
        };
        let line = serde_json::to_string(&msg).unwrap() + "\n";
        if let Err(e) = stream.write_all(line.as_bytes()).await {
            debug!(error = ?e, "reload send failed");
        }
        Ok(())
    }

    /// Try connecting up to 3 times with 100ms backoff (connection phase only).
    async fn connect_with_retry(&self) -> Option<TcpStream> {
        for attempt in 0..3u32 {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            if let Ok(Ok(stream)) = timeout(
                Duration::from_millis(500),
                TcpStream::connect(("127.0.0.1", self.port)),
            )
            .await
            {
                return Some(stream);
            }
        }
        None
    }
}
