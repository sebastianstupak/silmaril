// stub — full implementation in Task 5

/// Start the MCP HTTP server on `port` (tries `port..port+10`).
///
/// Returns the port that was successfully bound.
///
/// # Errors
///
/// Returns an error string if no port in the range could be bound.
pub async fn run(
    _port: u16,
    _channels: crate::AiBridgeChannels,
    _allow_all: bool,
    _permissions: std::sync::Arc<std::sync::Mutex<crate::permissions::PermissionStore>>,
    _shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<u16, String> {
    Ok(_port)
}
