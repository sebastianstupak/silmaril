# Admin Telnet Console Guide

Remote administration interface for Agent Game Engine servers.

## Quick Start

### 1. Enable Admin Feature

```toml
# In your Cargo.toml
[dependencies]
engine-observability = { path = "engine/observability", features = ["admin"] }
```

### 2. Start Admin Console in Server

```rust
use engine_observability::admin::{AdminConsole, AdminCommand};

#[tokio::main]
async fn main() {
    // Create admin console
    let mut console = AdminConsole::new("127.0.0.1:8888");
    let mut command_rx = console.command_receiver();

    // Start console in background
    tokio::spawn(async move {
        console.start().await.unwrap();
    });

    // Game loop
    loop {
        // Process admin commands
        while let Ok((command, response_tx)) = command_rx.try_recv() {
            let response = handle_command(&command);
            let _ = response_tx.send(response);
        }

        // Game tick...
    }
}
```

### 3. Connect via Telnet

```bash
telnet localhost 8888
```

## Available Commands

### Help & Status

**`help` or `?`**
Show all available commands

```
> help
Available Commands:
==================
  help, ?              - Show this help message
  status               - Show server status
  metrics              - Show current metrics
  ...
```

**`status`**
Show server status (tick rate, entity count, connected clients)

```
> status
Server Status:
- Tick: 12345
- Entities: 5000
- Paused: false
- Connected Clients: 25
```

**`metrics`**
Show current performance metrics

```
> metrics
Metrics:
- entity_count: 5000
- tick_rate_tps: 59.8
- connected_clients: 25
- network_bytes_sent: 1048576
```

### Server Control

**`pause`**
Pause server tick (for debugging)

```
> pause
Server paused
```

**`resume`**
Resume server tick

```
> resume
Server resumed
```

### Entity Management

**`spawn <count>`**
Spawn N entities

```
> spawn 100
Spawned 100 entities (total: 5100)
```

**`despawn <count>`**
Despawn N entities

```
> despawn 50
Despawned 50 entities (total: 5050)
```

### Configuration

**`set <key> <value>`**
Set configuration value at runtime

```
> set max_clients 1000
Set max_clients = 1000

> set tick_rate 30
Set tick_rate = 30
```

**`get <key>`**
Get configuration value

```
> get max_clients
max_clients = 1000
```

**Common config keys:**
- `max_clients` - Maximum connected clients
- `tick_rate` - Server tick rate (TPS)
- `log_level` - Logging verbosity
- `max_entities` - Maximum entity count

### Session Control

**`quit` or `exit`**
Disconnect from console

```
> quit
Goodbye!
```

## Integration Examples

### Basic Server Integration

```rust
use engine_observability::admin::{AdminCommand, AdminConsole};
use tokio::sync::mpsc;

struct GameServer {
    entity_count: i32,
    paused: bool,
    max_clients: u32,
}

impl GameServer {
    async fn run(mut self) {
        let mut console = AdminConsole::new("0.0.0.0:8888");
        let mut command_rx = console.command_receiver();

        tokio::spawn(async move {
            console.start().await.unwrap();
        });

        loop {
            // Process commands
            while let Ok((cmd, resp_tx)) = command_rx.try_recv() {
                let response = self.handle_command(&cmd);
                let _ = resp_tx.send(response);
            }

            // Game tick
            if !self.paused {
                self.tick();
            }
        }
    }

    fn handle_command(&mut self, cmd: &AdminCommand) -> String {
        match cmd {
            AdminCommand::Status => {
                format!("Entities: {}, Paused: {}",
                    self.entity_count, self.paused)
            }
            AdminCommand::Pause => {
                self.paused = true;
                "Server paused".to_string()
            }
            AdminCommand::Resume => {
                self.paused = false;
                "Server resumed".to_string()
            }
            AdminCommand::Spawn(n) => {
                self.entity_count += *n as i32;
                format!("Spawned {} entities", n)
            }
            _ => "Command not implemented".to_string(),
        }
    }

    fn tick(&mut self) {
        // Game logic...
    }
}
```

### With Metrics Integration

```rust
use engine_observability::admin::{AdminCommand, AdminConsole};
use engine_observability::metrics::MetricsRegistry;

async fn server_loop(metrics: MetricsRegistry) {
    let mut console = AdminConsole::new("0.0.0.0:8888");
    let mut command_rx = console.command_receiver();

    tokio::spawn(async move {
        console.start().await.unwrap();
    });

    let mut entity_count = 0;

    loop {
        // Handle commands
        while let Ok((cmd, resp_tx)) = command_rx.try_recv() {
            match cmd {
                AdminCommand::Spawn(n) => {
                    entity_count += n as i64;
                    metrics.increment_entity_count(n as i64);
                    let _ = resp_tx.send(format!("Spawned {}", n));
                }
                AdminCommand::Metrics => {
                    let response = format!(
                        "entity_count: {}\nfps: {}",
                        entity_count,
                        metrics.get_fps()
                    );
                    let _ = resp_tx.send(response);
                }
                _ => {}
            }
        }

        // Game tick...
    }
}
```

## Security

### ⚠️ Important Security Notes

The admin console provides **full control** over the server:
- Can spawn/despawn entities
- Can pause/resume server
- Can modify configuration
- Can execute arbitrary commands (if extended)

**Security recommendations:**

1. **Bind to localhost only in production:**
   ```rust
   AdminConsole::new("127.0.0.1:8888")  // ✅ Safe
   AdminConsole::new("0.0.0.0:8888")    // ⚠️ Dangerous
   ```

2. **Use SSH tunneling for remote access:**
   ```bash
   # On client machine
   ssh -L 8888:localhost:8888 user@game-server

   # Then connect locally
   telnet localhost 8888
   ```

3. **Add authentication (future enhancement):**
   ```rust
   // TODO: Add password authentication
   async fn authenticate(stream: &mut TcpStream) -> bool {
       stream.write_all(b"Password: ").await?;
       let password = read_line(stream).await?;
       password.trim() == env::var("ADMIN_PASSWORD")?
   }
   ```

4. **Firewall rules:**
   ```bash
   # Block port 8888 from external access
   iptables -A INPUT -p tcp --dport 8888 -s 127.0.0.1 -j ACCEPT
   iptables -A INPUT -p tcp --dport 8888 -j DROP
   ```

5. **Use VPN for remote teams:**
   - WireGuard or OpenVPN
   - Admin console only accessible via VPN

## Docker Integration

### Development

Admin console is exposed in docker-compose.dev.yml:

```yaml
services:
  server:
    ports:
      - "7777:7777/tcp"
      - "7778:7778/udp"
      - "8888:8888/tcp"  # Admin console
```

Connect:
```bash
telnet localhost 8888
```

### Production

**Do not expose** admin console port in production:

```yaml
services:
  server:
    ports:
      - "7777:7777/tcp"
      - "7778:7778/udp"
      # NO admin console port exposed
    networks:
      - game-network
```

Use docker exec instead:
```bash
docker exec -it agent-game-server /bin/sh
# Then run admin commands via server binary
```

## Troubleshooting

### Cannot connect to admin console

```bash
# Check if server is listening
netstat -tulpn | grep 8888

# Check Docker port mapping
docker port agent-game-server

# Test connection
telnet localhost 8888
```

### Connection accepted but no prompt

Check if tokio runtime is configured:
```rust
#[tokio::main]
async fn main() {
    // Tokio runtime required for admin console
}
```

### Commands not responding

Ensure you're processing command_rx in the game loop:
```rust
// MUST be called regularly
while let Ok((cmd, resp)) = command_rx.try_recv() {
    // Process command
}
```

### Port already in use

```bash
# Find process using port
lsof -i :8888

# Kill process
kill <PID>
```

## Performance

The admin console has minimal overhead:

**With admin feature disabled:**
- Zero overhead (stub implementation)
- No dependencies compiled

**With admin feature enabled:**
- ~10-20 microseconds per command
- ~1KB memory per connection
- Async I/O (non-blocking)

**Recommendation:**
- Enable in development
- Enable in production (with proper security)
- Disable only for benchmarking

## Advanced Usage

### Custom Commands

Extend AdminCommand enum:

```rust
#[derive(Debug, Clone)]
pub enum AdminCommand {
    // Built-in commands
    Status,
    Metrics,

    // Custom commands
    TeleportPlayer { player_id: u32, x: f32, y: f32 },
    SetWeather { weather: String },
    BroadcastMessage { message: String },
}
```

### Command Logging

Log all admin commands for audit:

```rust
use tracing::warn;

while let Ok((cmd, resp_tx)) = command_rx.try_recv() {
    warn!(
        command = ?cmd,
        "Admin command executed"
    );

    let response = handle_command(&cmd);
    let _ = resp_tx.send(response);
}
```

### Rate Limiting

Prevent command spam:

```rust
use std::time::{Duration, Instant};

let mut last_command = Instant::now();
let cooldown = Duration::from_millis(100);

while let Ok((cmd, resp_tx)) = command_rx.try_recv() {
    if last_command.elapsed() < cooldown {
        let _ = resp_tx.send("Rate limited".to_string());
        continue;
    }

    last_command = Instant::now();
    // Process command...
}
```

## Examples

See:
- `engine/observability/examples/admin_console.rs` - Complete example
- Run: `cargo run --example admin_console --features admin`

## Further Reading

- [Telnet Protocol](https://www.rfc-editor.org/rfc/rfc854)
- [tokio::net Documentation](https://docs.rs/tokio/latest/tokio/net/)
- [Server Administration Best Practices](https://12factor.net/)
