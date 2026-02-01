# Engine Networking

## Purpose
The networking crate provides robust client-server networking:
- **Network Protocol**: Custom protocol with TCP for reliability and UDP for real-time data
- **TCP Connection**: Connection management, authentication, and reliable messaging
- **UDP Packets**: Unreliable but fast packets for position/rotation updates
- **State Sync**: Delta compression and entity state synchronization
- **Client Prediction**: Client-side prediction with server reconciliation

This crate enables multiplayer gameplay with support for 1000+ concurrent players.

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase2-network-protocol.md](../../docs/phase2-network-protocol.md)** - Protocol design and packet format
2. **[phase2-tcp-connection.md](../../docs/phase2-tcp-connection.md)** - TCP connection management
3. **[phase2-udp-packets.md](../../docs/phase2-udp-packets.md)** - UDP packet design and rate limiting
4. **[phase2-state-sync.md](../../docs/phase2-state-sync.md)** - Entity state synchronization
5. **[phase2-client-prediction.md](../../docs/phase2-client-prediction.md)** - Client prediction and reconciliation

## Related Crates
- **engine-core**: Uses ECS and serialization for network state
- **engine-interest**: Integrates with interest management for bandwidth optimization
- **engine-lod**: Uses LOD system for network traffic reduction

## Quick Example
```rust
use engine_networking::{Server, Client, Message};

// Server-side
let mut server = Server::bind("0.0.0.0:7777")?;
server.on_message(|client_id, msg| {
    match msg {
        Message::PlayerMove(pos) => {
            // Broadcast to all nearby clients
            server.broadcast_nearby(client_id, pos, Message::EntityUpdate);
        }
    }
});

// Client-side
let mut client = Client::connect("127.0.0.1:7777")?;
client.send(Message::PlayerMove(position));
```

## Key Dependencies
- `tokio` - Async runtime
- `quinn` - QUIC implementation (future)
- `engine-core` - ECS and serialization
- `engine-interest` - Interest management integration

## Performance Targets
- 1000+ concurrent connections per server
- <50ms latency for TCP messages
- <20ms latency for UDP packets
- 60Hz update rate for player positions
- <10KB/sec per player bandwidth usage
