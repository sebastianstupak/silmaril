//! Admin telnet console for server management
//!
//! Provides a telnet-based administrative interface for:
//! - Live metrics inspection
//! - Server control commands
//! - Entity management
//! - Configuration changes
//! - Debugging and diagnostics
//!
//! # Security
//!
//! **WARNING**: The admin console has full control over the server.
//! Only expose on trusted networks or with proper authentication.
//!
//! # Examples
//!
//! ```no_run
//! use engine_observability::admin::{AdminConsole, AdminCommand};
//!
//! #[tokio::main]
//! async fn main() {
//!     let console = AdminConsole::new("127.0.0.1:8888");
//!
//!     // Start console server
//!     tokio::spawn(async move {
//!         console.start().await.unwrap();
//!     });
//! }
//! ```

#[cfg(feature = "admin")]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(feature = "admin")]
use tokio::net::{TcpListener, TcpStream};
#[cfg(feature = "admin")]
use tokio::sync::mpsc;

#[cfg(feature = "admin")]
use std::net::SocketAddr;

#[cfg(feature = "admin")]
use tracing::{error, info};

#[cfg(not(feature = "admin"))]
use tracing::warn;

/// Admin command types
#[derive(Debug, Clone, PartialEq)]
pub enum AdminCommand {
    /// Show help message
    Help,
    /// Show server status
    Status,
    /// Show metrics
    Metrics,
    /// Pause server tick
    Pause,
    /// Resume server tick
    Resume,
    /// Spawn N entities
    Spawn(u32),
    /// Despawn N entities
    Despawn(u32),
    /// Set configuration value
    SetConfig {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    /// Get configuration value
    GetConfig {
        /// Configuration key
        key: String,
    },
    /// Disconnect client
    Quit,
    /// Unknown command
    Unknown(String),
}

impl AdminCommand {
    /// Parse command from string
    pub fn parse(input: &str) -> Self {
        let input = input.trim();
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Self::Unknown(String::new());
        }

        match parts[0].to_lowercase().as_str() {
            "help" | "?" => Self::Help,
            "status" => Self::Status,
            "metrics" => Self::Metrics,
            "pause" => Self::Pause,
            "resume" => Self::Resume,
            "spawn" => {
                if parts.len() < 2 {
                    return Self::Unknown("spawn requires count argument".to_string());
                }
                match parts[1].parse::<u32>() {
                    Ok(count) => Self::Spawn(count),
                    Err(_) => Self::Unknown("spawn count must be a number".to_string()),
                }
            }
            "despawn" => {
                if parts.len() < 2 {
                    return Self::Unknown("despawn requires count argument".to_string());
                }
                match parts[1].parse::<u32>() {
                    Ok(count) => Self::Despawn(count),
                    Err(_) => Self::Unknown("despawn count must be a number".to_string()),
                }
            }
            "set" => {
                if parts.len() < 3 {
                    return Self::Unknown("set requires key and value".to_string());
                }
                Self::SetConfig {
                    key: parts[1].to_string(),
                    value: parts[2..].join(" "),
                }
            }
            "get" => {
                if parts.len() < 2 {
                    return Self::Unknown("get requires key".to_string());
                }
                Self::GetConfig {
                    key: parts[1].to_string(),
                }
            }
            "quit" | "exit" => Self::Quit,
            _ => Self::Unknown(input.to_string()),
        }
    }
}

/// Admin console server
#[cfg(feature = "admin")]
pub struct AdminConsole {
    addr: String,
    command_tx: mpsc::UnboundedSender<(AdminCommand, mpsc::UnboundedSender<String>)>,
    command_rx: mpsc::UnboundedReceiver<(AdminCommand, mpsc::UnboundedSender<String>)>,
}

#[cfg(feature = "admin")]
impl AdminConsole {
    /// Create a new admin console
    ///
    /// # Arguments
    /// * `addr` - Address to bind to (e.g., "127.0.0.1:8888")
    pub fn new(addr: &str) -> Self {
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        Self {
            addr: addr.to_string(),
            command_tx,
            command_rx,
        }
    }

    /// Start the admin console server
    ///
    /// This will bind to the configured address and accept telnet connections.
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        let addr = listener.local_addr()?;

        info!("Admin console listening on telnet://{}", addr);
        info!("Connect with: telnet {} {}", addr.ip(), addr.port());

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("Admin client connected: {}", peer_addr);
                    let tx = self.command_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, peer_addr, tx).await {
                            error!("Client error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Get command receiver (for server to process commands)
    pub fn command_receiver(
        &mut self,
    ) -> &mut mpsc::UnboundedReceiver<(AdminCommand, mpsc::UnboundedSender<String>)> {
        &mut self.command_rx
    }
}

/// Handle a telnet client connection
#[cfg(feature = "admin")]
async fn handle_client(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    command_tx: mpsc::UnboundedSender<(AdminCommand, mpsc::UnboundedSender<String>)>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Send welcome message
    stream
        .write_all(b"\r\n=== Agent Game Engine Admin Console ===\r\n")
        .await?;
    stream
        .write_all(b"Type 'help' for available commands\r\n\r\n")
        .await?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        // Send prompt
        writer.write_all(b"> ").await?;
        writer.flush().await?;

        // Read line
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            info!("Admin client disconnected: {}", peer_addr);
            break;
        }

        // Parse command
        let command = AdminCommand::parse(&line);

        match command {
            AdminCommand::Quit => {
                writer.write_all(b"Goodbye!\r\n").await?;
                info!("Admin client disconnected: {}", peer_addr);
                break;
            }
            AdminCommand::Help => {
                let help = get_help_text();
                writer.write_all(help.as_bytes()).await?;
            }
            AdminCommand::Unknown(ref msg) if msg.is_empty() => {
                // Empty command, just show prompt again
            }
            AdminCommand::Unknown(ref msg) => {
                let response = format!("Unknown command: {}\r\n", msg);
                writer.write_all(response.as_bytes()).await?;
            }
            _ => {
                // Send command to server and get response
                let (response_tx, mut response_rx) = mpsc::unbounded_channel();
                if let Err(e) = command_tx.send((command.clone(), response_tx)) {
                    error!("Failed to send command: {}", e);
                    writer
                        .write_all(b"Error: Server not responding\r\n")
                        .await?;
                    continue;
                }

                // Wait for response
                if let Some(response) = response_rx.recv().await {
                    writer.write_all(response.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;
                }
            }
        }
    }

    Ok(())
}

/// Get help text
#[cfg(feature = "admin")]
fn get_help_text() -> String {
    r#"
Available Commands:
==================

  help, ?              - Show this help message
  status               - Show server status
  metrics              - Show current metrics
  pause                - Pause server tick
  resume               - Resume server tick
  spawn <count>        - Spawn N entities
  despawn <count>      - Despawn N entities
  set <key> <value>    - Set configuration value
  get <key>            - Get configuration value
  quit, exit           - Disconnect from console

Examples:
=========

  > status
  > metrics
  > spawn 100
  > despawn 50
  > set max_clients 1000
  > get max_clients

"#
    .to_string()
}

/// Stub admin console implementation when admin feature is disabled
///
/// This is a no-op implementation that logs a warning when started.
#[cfg(not(feature = "admin"))]
pub struct AdminConsole;

#[cfg(not(feature = "admin"))]
impl AdminConsole {
    /// Create a new stub admin console
    ///
    /// # Arguments
    /// * `_addr` - Ignored (admin feature is disabled)
    pub fn new(_addr: &str) -> Self {
        Self
    }

    /// Start the stub admin console (no-op)
    ///
    /// Logs a warning that the admin feature is disabled.
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        warn!("Admin console feature is disabled");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        assert_eq!(AdminCommand::parse("help"), AdminCommand::Help);
        assert_eq!(AdminCommand::parse("?"), AdminCommand::Help);
        assert_eq!(AdminCommand::parse("HELP"), AdminCommand::Help);
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(AdminCommand::parse("status"), AdminCommand::Status);
        assert_eq!(AdminCommand::parse("STATUS"), AdminCommand::Status);
    }

    #[test]
    fn test_parse_spawn() {
        assert_eq!(AdminCommand::parse("spawn 100"), AdminCommand::Spawn(100));
        assert_eq!(AdminCommand::parse("spawn 1"), AdminCommand::Spawn(1));
    }

    #[test]
    fn test_parse_spawn_invalid() {
        match AdminCommand::parse("spawn") {
            AdminCommand::Unknown(msg) => assert!(msg.contains("requires count")),
            _ => panic!("Expected Unknown"),
        }

        match AdminCommand::parse("spawn abc") {
            AdminCommand::Unknown(msg) => assert!(msg.contains("must be a number")),
            _ => panic!("Expected Unknown"),
        }
    }

    #[test]
    fn test_parse_set_config() {
        match AdminCommand::parse("set max_clients 1000") {
            AdminCommand::SetConfig { key, value } => {
                assert_eq!(key, "max_clients");
                assert_eq!(value, "1000");
            }
            _ => panic!("Expected SetConfig"),
        }
    }

    #[test]
    fn test_parse_get_config() {
        match AdminCommand::parse("get max_clients") {
            AdminCommand::GetConfig { key } => {
                assert_eq!(key, "max_clients");
            }
            _ => panic!("Expected GetConfig"),
        }
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(AdminCommand::parse("quit"), AdminCommand::Quit);
        assert_eq!(AdminCommand::parse("exit"), AdminCommand::Quit);
    }

    #[test]
    fn test_parse_unknown() {
        match AdminCommand::parse("foobar") {
            AdminCommand::Unknown(cmd) => assert_eq!(cmd, "foobar"),
            _ => panic!("Expected Unknown"),
        }
    }

    #[test]
    fn test_parse_empty() {
        match AdminCommand::parse("") {
            AdminCommand::Unknown(cmd) => assert_eq!(cmd, ""),
            _ => panic!("Expected Unknown"),
        }
    }
}
