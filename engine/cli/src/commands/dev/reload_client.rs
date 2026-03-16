//! Process-aware wrapper around `ReloadClient`.
//!
//! Routes reload signals to the correct process port based on game.toml config.

use engine_dev_tools_hot_reload::client::ReloadClient;
use tracing::debug;

/// Wrapper that knows about server and client ports.
pub struct DevReloadClient {
    server_client: ReloadClient,
    client_client: ReloadClient,
}

impl DevReloadClient {
    /// Create with separate ports for server (e.g. 9999) and client (e.g. 9998).
    pub fn new(server_port: u16, client_port: u16) -> Self {
        Self {
            server_client: ReloadClient::new(server_port),
            client_client: ReloadClient::new(client_port),
        }
    }

    /// Returns true if the config path belongs to the server (matches "server" prefix).
    pub fn is_server_config(&self, path: &str) -> bool {
        std::path::Path::new(path)
            .file_name()
            .and_then(|f| f.to_str())
            .map(|name| name.starts_with("server"))
            .unwrap_or(false)
    }

    /// Send `reload_asset` to both server and client (assets may be needed by either).
    pub async fn reload_asset_to_both(&self, path: &str) {
        debug!(path, "sending asset reload to both processes");
        self.server_client.send_reload_asset(path).await.ok();
        self.client_client.send_reload_asset(path).await.ok();
    }

    /// Send `reload_config` to the correct process based on filename prefix.
    ///
    /// - `config/server*.ron` → server process
    /// - `config/client*.ron` → client process
    /// - others → both
    pub async fn reload_config_smart(&self, path: &str) {
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");

        if name.starts_with("server") {
            debug!(path, "routing config reload to server");
            self.server_client.send_reload_config(path).await.ok();
        } else if name.starts_with("client") {
            debug!(path, "routing config reload to client");
            self.client_client.send_reload_config(path).await.ok();
        } else {
            debug!(path, "routing config reload to both");
            self.server_client.send_reload_config(path).await.ok();
            self.client_client.send_reload_config(path).await.ok();
        }
    }

    /// Send `serialize_state` to the process on the given port.
    pub async fn serialize_state_for_port(&self, port: u16) {
        let client = ReloadClient::new(port);
        client.send_serialize_state().await.ok();
    }

    /// Send `serialize_state` to the server process.
    pub async fn serialize_state_server(&self) {
        self.server_client.send_serialize_state().await.ok();
    }

    /// Send `serialize_state` to the client process.
    pub async fn serialize_state_client(&self) {
        self.client_client.send_serialize_state().await.ok();
    }
}
