# Phase 2.4: UDP Packets (Unreliable)

**Status:** ⚪ Not Started
**Estimated Time:** 3-4 days
**Priority:** Critical (low-latency position updates)

---

## 🎯 **Objective**

Implement UDP packet handling for unreliable but fast transmission of time-sensitive data (player input, position updates). UDP trades reliability for speed, making it ideal for real-time game state.

**UDP Used For:**
- Player input (WASD, mouse)
- Position updates (continuous movement)
- Fast events (shooting, jumping)
- Non-critical state updates

**Key Features:**
- Packet loss tolerance
- No head-of-line blocking
- Sub-millisecond send times
- High throughput (1000+ packets/sec)

---

## 📋 **Detailed Tasks**

### **1. UDP Server** (Day 1-2)

**File:** `engine/networking/src/udp/server.rs`

```rust
use tokio::net::UdpSocket;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

/// UDP server for unreliable fast packets
pub struct UdpServer {
    socket: Arc<UdpSocket>,
    client_mappings: Arc<RwLock<HashMap<SocketAddr, u64>>>, // addr -> client_id
    addr_mappings: Arc<RwLock<HashMap<u64, SocketAddr>>>,   // client_id -> addr
}

impl UdpServer {
    /// Create UDP server
    pub async fn new(bind_addr: &str) -> Result<Self, NetworkError> {
        let socket = UdpSocket::bind(bind_addr)
            .await
            .map_err(|e| NetworkError::UdpBindFailed {
                details: e.to_string(),
            })?;

        tracing::info!("UDP server listening on {}", bind_addr);

        Ok(Self {
            socket: Arc::new(socket),
            client_mappings: Arc::new(RwLock::new(HashMap::new())),
            addr_mappings: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register client address mapping
    pub async fn register_client(&self, client_id: u64, addr: SocketAddr) {
        let mut client_mappings = self.client_mappings.write().await;
        let mut addr_mappings = self.addr_mappings.write().await;

        client_mappings.insert(addr, client_id);
        addr_mappings.insert(client_id, addr);

        tracing::debug!("Registered UDP mapping: client {} <-> {}", client_id, addr);
    }

    /// Unregister client
    pub async fn unregister_client(&self, client_id: u64) {
        let mut client_mappings = self.client_mappings.write().await;
        let mut addr_mappings = self.addr_mappings.write().await;

        if let Some(addr) = addr_mappings.remove(&client_id) {
            client_mappings.remove(&addr);
            tracing::debug!("Unregistered UDP mapping for client {}", client_id);
        }
    }

    /// Receive packet (non-blocking)
    pub async fn try_recv(&self) -> Result<(u64, Vec<u8>), NetworkError> {
        let mut buf = vec![0u8; MAX_UDP_PACKET_SIZE];

        match self.socket.try_recv_from(&mut buf) {
            Ok((len, addr)) => {
                buf.truncate(len);

                // Look up client ID
                let client_mappings = self.client_mappings.read().await;
                if let Some(&client_id) = client_mappings.get(&addr) {
                    Ok((client_id, buf))
                } else {
                    Err(NetworkError::UnknownClient { addr })
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Err(NetworkError::WouldBlock)
            }
            Err(e) => Err(NetworkError::UdpReceiveFailed {
                details: e.to_string(),
            }),
        }
    }

    /// Send packet to client
    pub async fn send_to_client(&self, client_id: u64, data: &[u8]) -> Result<(), NetworkError> {
        let addr_mappings = self.addr_mappings.read().await;

        if let Some(&addr) = addr_mappings.get(&client_id) {
            self.socket
                .send_to(data, addr)
                .await
                .map_err(|e| NetworkError::UdpSendFailed {
                    details: e.to_string(),
                })?;
            Ok(())
        } else {
            Err(NetworkError::ClientNotFound { client_id })
        }
    }

    /// Broadcast to all clients
    pub async fn broadcast(&self, data: &[u8]) -> Result<(), NetworkError> {
        let addr_mappings = self.addr_mappings.read().await;

        for (client_id, addr) in addr_mappings.iter() {
            if let Err(e) = self.socket.send_to(data, addr).await {
                tracing::warn!("Failed to send UDP packet to client {}: {}", client_id, e);
                // Continue broadcasting to other clients
            }
        }

        Ok(())
    }

    /// Broadcast to clients except one
    pub async fn broadcast_except(&self, exclude_client_id: u64, data: &[u8]) -> Result<(), NetworkError> {
        let addr_mappings = self.addr_mappings.read().await;

        for (client_id, addr) in addr_mappings.iter() {
            if *client_id != exclude_client_id {
                if let Err(e) = self.socket.send_to(data, addr).await {
                    tracing::warn!("Failed to send UDP packet to client {}: {}", client_id, e);
                }
            }
        }

        Ok(())
    }

    /// Get socket for cloning
    pub fn socket(&self) -> Arc<UdpSocket> {
        Arc::clone(&self.socket)
    }
}

const MAX_UDP_PACKET_SIZE: usize = 1400; // Safe MTU size (< 1500 to avoid fragmentation)
```

---

### **2. UDP Client** (Day 2)

**File:** `engine/networking/src/udp/client.rs`

```rust
use tokio::net::UdpSocket;
use std::net::SocketAddr;

/// UDP client
pub struct UdpClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl UdpClient {
    /// Create UDP client
    pub async fn new() -> Result<Self, NetworkError> {
        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| NetworkError::UdpBindFailed {
                details: e.to_string(),
            })?;

        tracing::info!("UDP client created on {}", socket.local_addr().unwrap());

        Ok(Self {
            socket,
            server_addr: "0.0.0.0:0".parse().unwrap(), // Will be set in connect
        })
    }

    /// Connect to server
    pub async fn connect(&mut self, server_addr: SocketAddr) -> Result<(), NetworkError> {
        self.server_addr = server_addr;

        // Send connection packet
        let handshake = b"UDP_HANDSHAKE";
        self.socket
            .send_to(handshake, server_addr)
            .await
            .map_err(|e| NetworkError::UdpSendFailed {
                details: e.to_string(),
            })?;

        tracing::info!("UDP connected to server at {}", server_addr);

        Ok(())
    }

    /// Send packet
    pub async fn send(&self, data: &[u8]) -> Result<(), NetworkError> {
        if data.len() > MAX_UDP_PACKET_SIZE {
            return Err(NetworkError::PacketTooLarge {
                size: data.len(),
                max: MAX_UDP_PACKET_SIZE,
            });
        }

        self.socket
            .send_to(data, self.server_addr)
            .await
            .map_err(|e| NetworkError::UdpSendFailed {
                details: e.to_string(),
            })?;

        Ok(())
    }

    /// Receive packet (non-blocking)
    pub async fn try_recv(&self) -> Result<Vec<u8>, NetworkError> {
        let mut buf = vec![0u8; MAX_UDP_PACKET_SIZE];

        match self.socket.try_recv(&mut buf) {
            Ok(len) => {
                buf.truncate(len);
                Ok(buf)
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Err(NetworkError::WouldBlock)
            }
            Err(e) => Err(NetworkError::UdpReceiveFailed {
                details: e.to_string(),
            }),
        }
    }

    /// Receive packet (blocking with timeout)
    pub async fn recv_timeout(&self, timeout: Duration) -> Result<Vec<u8>, NetworkError> {
        let mut buf = vec![0u8; MAX_UDP_PACKET_SIZE];

        match tokio::time::timeout(timeout, self.socket.recv(&mut buf)).await {
            Ok(Ok(len)) => {
                buf.truncate(len);
                Ok(buf)
            }
            Ok(Err(e)) => Err(NetworkError::UdpReceiveFailed {
                details: e.to_string(),
            }),
            Err(_) => Err(NetworkError::RecvTimeout),
        }
    }

    /// Get local address
    pub fn local_addr(&self) -> Result<SocketAddr, NetworkError> {
        self.socket
            .local_addr()
            .map_err(|e| NetworkError::SocketError {
                details: e.to_string(),
            })
    }
}

const MAX_UDP_PACKET_SIZE: usize = 1400;
```

---

### **3. Packet Loss Handling** (Day 2-3)

**File:** `engine/networking/src/udp/loss_tolerance.rs`

```rust
use std::collections::VecDeque;

/// Packet loss statistics tracker
pub struct PacketLossTracker {
    /// Sequence numbers received
    received_sequences: VecDeque<u32>,

    /// Window size for tracking
    window_size: usize,

    /// Expected sequence number
    expected_sequence: u32,

    /// Total packets received
    total_received: u64,

    /// Total packets lost (estimated)
    total_lost: u64,
}

impl PacketLossTracker {
    pub fn new(window_size: usize) -> Self {
        Self {
            received_sequences: VecDeque::with_capacity(window_size),
            window_size,
            expected_sequence: 0,
            total_received: 0,
            total_lost: 0,
        }
    }

    /// Record received packet
    pub fn record_received(&mut self, sequence: u32) {
        // Add to received list
        self.received_sequences.push_back(sequence);

        // Trim window
        if self.received_sequences.len() > self.window_size {
            self.received_sequences.pop_front();
        }

        // Update stats
        self.total_received += 1;

        // Estimate lost packets
        if sequence > self.expected_sequence {
            let lost = sequence - self.expected_sequence;
            self.total_lost += lost as u64;
        }

        self.expected_sequence = sequence + 1;
    }

    /// Get packet loss percentage (0.0 - 1.0)
    pub fn loss_percentage(&self) -> f32 {
        let total = self.total_received + self.total_lost;
        if total == 0 {
            return 0.0;
        }

        self.total_lost as f32 / total as f32
    }

    /// Get recent loss percentage (within window)
    pub fn recent_loss_percentage(&self) -> f32 {
        if self.received_sequences.is_empty() {
            return 0.0;
        }

        // Check for gaps in received sequences
        let mut received_count = self.received_sequences.len();
        let min_seq = *self.received_sequences.front().unwrap();
        let max_seq = *self.received_sequences.back().unwrap();

        if max_seq <= min_seq {
            return 0.0;
        }

        let expected_count = (max_seq - min_seq + 1) as usize;
        let lost_count = expected_count.saturating_sub(received_count);

        lost_count as f32 / expected_count as f32
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        self.received_sequences.clear();
        self.expected_sequence = 0;
        self.total_received = 0;
        self.total_lost = 0;
    }
}

/// Redundant packet sender (sends critical info multiple times)
pub struct RedundantSender {
    /// Number of times to send each packet
    redundancy: usize,
}

impl RedundantSender {
    pub fn new(redundancy: usize) -> Self {
        Self { redundancy }
    }

    /// Send packet with redundancy
    pub async fn send_redundant<F>(&self, data: &[u8], mut send_fn: F) -> Result<(), NetworkError>
    where
        F: FnMut(&[u8]) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), NetworkError>> + '_>>,
    {
        for _ in 0..self.redundancy {
            send_fn(data).await?;
        }
        Ok(())
    }

    /// Adjust redundancy based on loss rate
    pub fn adjust_redundancy(&mut self, loss_rate: f32) {
        if loss_rate > 0.1 {
            // High loss: increase redundancy
            self.redundancy = (self.redundancy + 1).min(5);
        } else if loss_rate < 0.01 {
            // Low loss: decrease redundancy
            self.redundancy = (self.redundancy.saturating_sub(1)).max(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loss_tracker_no_loss() {
        let mut tracker = PacketLossTracker::new(100);

        for seq in 0..100 {
            tracker.record_received(seq);
        }

        assert_eq!(tracker.loss_percentage(), 0.0);
        assert_eq!(tracker.recent_loss_percentage(), 0.0);
    }

    #[test]
    fn test_loss_tracker_with_loss() {
        let mut tracker = PacketLossTracker::new(100);

        // Receive packets: 0, 1, 3, 4, 6 (missing 2, 5)
        tracker.record_received(0);
        tracker.record_received(1);
        tracker.record_received(3); // Lost 2
        tracker.record_received(4);
        tracker.record_received(6); // Lost 5

        // Should detect ~28% loss (2 lost out of 7 total)
        let loss = tracker.loss_percentage();
        assert!(loss > 0.25 && loss < 0.35, "Loss was {}", loss);
    }
}
```

---

### **4. Position Update Optimization** (Day 3)

**File:** `engine/networking/src/udp/position_updates.rs`

```rust
/// Optimized position update packet (minimal size)
#[repr(C, packed)]
pub struct PositionUpdate {
    /// Entity ID (16-bit for compactness)
    pub entity_id: u16,

    /// Sequence number
    pub sequence: u32,

    /// Position (quantized to 16-bit per axis)
    pub pos_x: i16,
    pub pos_y: i16,
    pub pos_z: i16,

    /// Velocity (quantized to 8-bit per axis)
    pub vel_x: i8,
    pub vel_y: i8,
    pub vel_z: i8,

    /// Rotation (compressed quaternion)
    pub rot_compressed: u32, // Smallest-three quaternion compression
}

impl PositionUpdate {
    /// Create from transform and velocity
    pub fn from_components(
        entity_id: u16,
        sequence: u32,
        position: glam::Vec3,
        velocity: glam::Vec3,
        rotation: glam::Quat,
    ) -> Self {
        Self {
            entity_id,
            sequence,
            pos_x: Self::quantize_position(position.x),
            pos_y: Self::quantize_position(position.y),
            pos_z: Self::quantize_position(position.z),
            vel_x: Self::quantize_velocity(velocity.x),
            vel_y: Self::quantize_velocity(velocity.y),
            vel_z: Self::quantize_velocity(velocity.z),
            rot_compressed: Self::compress_quaternion(rotation),
        }
    }

    /// Convert to transform and velocity
    pub fn to_components(&self) -> (glam::Vec3, glam::Vec3, glam::Quat) {
        let position = glam::Vec3::new(
            Self::dequantize_position(self.pos_x),
            Self::dequantize_position(self.pos_y),
            Self::dequantize_position(self.pos_z),
        );

        let velocity = glam::Vec3::new(
            Self::dequantize_velocity(self.vel_x),
            Self::dequantize_velocity(self.vel_y),
            Self::dequantize_velocity(self.vel_z),
        );

        let rotation = Self::decompress_quaternion(self.rot_compressed);

        (position, velocity, rotation)
    }

    /// Quantize position to i16 (range: -327.68 to 327.67 with 0.01 precision)
    fn quantize_position(value: f32) -> i16 {
        (value * 100.0).clamp(-32768.0, 32767.0) as i16
    }

    /// Dequantize position from i16
    fn dequantize_position(value: i16) -> f32 {
        value as f32 / 100.0
    }

    /// Quantize velocity to i8 (range: -1.28 to 1.27 with 0.01 precision)
    fn quantize_velocity(value: f32) -> i8 {
        (value * 100.0).clamp(-128.0, 127.0) as i8
    }

    /// Dequantize velocity from i8
    fn dequantize_velocity(value: i8) -> f32 {
        value as f32 / 100.0
    }

    /// Compress quaternion to u32 (smallest-three encoding)
    fn compress_quaternion(quat: glam::Quat) -> u32 {
        // Find largest component
        let components = [quat.x, quat.y, quat.z, quat.w];
        let abs_components = components.map(|c| c.abs());
        let largest_idx = abs_components
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        // Encode other three components (10 bits each) + 2 bits for largest index
        let mut result = (largest_idx as u32) << 30;

        let mut j = 0;
        for i in 0..4 {
            if i != largest_idx {
                let value = components[i];
                let quantized = ((value + 1.0) * 511.5).clamp(0.0, 1023.0) as u32;
                result |= quantized << (j * 10);
                j += 1;
            }
        }

        result
    }

    /// Decompress quaternion from u32
    fn decompress_quaternion(compressed: u32) -> glam::Quat {
        // Extract largest index
        let largest_idx = (compressed >> 30) as usize;

        // Extract other components
        let mut components = [0.0f32; 4];
        let mut j = 0;
        for i in 0..4 {
            if i != largest_idx {
                let quantized = (compressed >> (j * 10)) & 0x3FF;
                components[i] = (quantized as f32 / 511.5) - 1.0;
                j += 1;
            }
        }

        // Reconstruct largest component
        let sum_of_squares: f32 = components.iter().map(|c| c * c).sum();
        components[largest_idx] = (1.0 - sum_of_squares).max(0.0).sqrt();

        glam::Quat::from_xyzw(components[0], components[1], components[2], components[3])
            .normalize()
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 19] {
        unsafe { std::mem::transmute(*self) }
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 19]) -> Self {
        unsafe { std::mem::transmute(*bytes) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_quantization() {
        let original = 123.45f32;
        let quantized = PositionUpdate::quantize_position(original);
        let dequantized = PositionUpdate::dequantize_position(quantized);

        // Should be within 0.01 precision
        assert!((original - dequantized).abs() < 0.01);
    }

    #[test]
    fn test_quaternion_compression() {
        let original = glam::Quat::from_rotation_y(1.5);
        let compressed = PositionUpdate::compress_quaternion(original);
        let decompressed = PositionUpdate::decompress_quaternion(compressed);

        // Should be close (small precision loss)
        assert!(original.dot(decompressed).abs() > 0.99);
    }

    #[test]
    fn test_packet_size() {
        // Should be exactly 19 bytes
        assert_eq!(std::mem::size_of::<PositionUpdate>(), 19);
    }
}
```

---

### **5. Performance Optimization** (Day 3-4)

**File:** `engine/networking/src/udp/batching.rs`

```rust
/// Batch multiple position updates into single UDP packet
pub struct PacketBatcher {
    /// Maximum packet size
    max_packet_size: usize,

    /// Current batch
    batch: Vec<u8>,
}

impl PacketBatcher {
    pub fn new(max_packet_size: usize) -> Self {
        Self {
            max_packet_size,
            batch: Vec::with_capacity(max_packet_size),
        }
    }

    /// Add update to batch
    pub fn add_update(&mut self, update: &PositionUpdate) -> bool {
        let update_bytes = update.to_bytes();

        if self.batch.len() + update_bytes.len() > self.max_packet_size {
            return false; // Batch full
        }

        self.batch.extend_from_slice(&update_bytes);
        true
    }

    /// Get batch and reset
    pub fn take_batch(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.batch)
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }

    /// Get batch size
    pub fn len(&self) -> usize {
        self.batch.len()
    }
}

/// Extract updates from batched packet
pub fn unbatch_updates(data: &[u8]) -> Vec<PositionUpdate> {
    let mut updates = Vec::new();
    let update_size = std::mem::size_of::<PositionUpdate>();

    for chunk in data.chunks_exact(update_size) {
        if let Ok(bytes) = chunk.try_into() {
            updates.push(PositionUpdate::from_bytes(bytes));
        }
    }

    updates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batching() {
        let mut batcher = PacketBatcher::new(1400);

        // Create dummy update
        let update = PositionUpdate::from_components(
            1,
            100,
            glam::Vec3::ZERO,
            glam::Vec3::ZERO,
            glam::Quat::IDENTITY,
        );

        // Should fit many updates in one packet
        for _ in 0..70 {
            assert!(batcher.add_update(&update));
        }

        // Should reject when full
        assert!(!batcher.add_update(&update));

        let batch = batcher.take_batch();
        assert_eq!(batch.len(), 70 * 19);

        // Unbatch
        let unbatched = unbatch_updates(&batch);
        assert_eq!(unbatched.len(), 70);
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] UDP server accepts packets from clients
- [ ] UDP client sends packets to server
- [ ] Packet loss detection works
- [ ] Position updates optimized to 19 bytes
- [ ] Batching reduces packet count
- [ ] Send time consistently < 1ms
- [ ] Can handle 1000+ packets/second
- [ ] Packet loss < 5% on normal networks
- [ ] No fragmentation (packets < 1400 bytes)
- [ ] Client address mappings maintained

---

## 🎯 **Performance Targets**

| Operation | Target | Critical |
|-----------|--------|----------|
| Send single packet | < 0.5ms | < 1ms |
| Receive single packet | < 0.3ms | < 1ms |
| Batch 50 updates | < 1ms | < 2ms |
| Unbatch 50 updates | < 0.5ms | < 1ms |
| Throughput (server) | > 1000 pkt/s | > 500 pkt/s |
| Packet size (position update) | 19 bytes | < 50 bytes |
| Packet loss tolerance | < 10% | < 20% |

**Latency Targets:**
- One-way latency: < 50ms (LAN), < 100ms (WAN)
- Round-trip time: < 100ms (LAN), < 200ms (WAN)

---

## 🧪 **Tests**

```rust
#[tokio::test]
async fn test_udp_send_recv() {
    // Start server
    let server = UdpServer::new("127.0.0.1:8888").await.unwrap();

    // Create client
    let mut client = UdpClient::new().await.unwrap();
    client.connect("127.0.0.1:8888".parse().unwrap()).await.unwrap();

    // Register client on server
    let client_addr = client.local_addr().unwrap();
    server.register_client(1, client_addr).await;

    // Send from client
    let data = b"Hello, Server!";
    client.send(data).await.unwrap();

    // Wait for packet
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Receive on server
    let (client_id, received) = server.try_recv().await.unwrap();
    assert_eq!(client_id, 1);
    assert_eq!(&received, data);
}

#[tokio::test]
async fn test_position_update_roundtrip() {
    let original_pos = glam::Vec3::new(10.5, 20.3, 30.7);
    let original_vel = glam::Vec3::new(1.2, -0.5, 0.8);
    let original_rot = glam::Quat::from_rotation_y(1.5);

    let update = PositionUpdate::from_components(
        123,
        456,
        original_pos,
        original_vel,
        original_rot,
    );

    let (pos, vel, rot) = update.to_components();

    // Check precision
    assert!((pos - original_pos).length() < 0.1);
    assert!((vel - original_vel).length() < 0.05);
    assert!(rot.dot(original_rot) > 0.99);
}

#[tokio::test]
async fn test_packet_loss_tracking() {
    let mut tracker = PacketLossTracker::new(100);

    // Simulate packet loss pattern: receive 0-9, lose 10-12, receive 13-20
    for i in 0..10 {
        tracker.record_received(i);
    }
    for i in 13..=20 {
        tracker.record_received(i);
    }

    // Should detect loss
    let loss = tracker.loss_percentage();
    assert!(loss > 0.1, "Loss rate was {}", loss);
}

#[tokio::test]
async fn test_high_throughput() {
    let server = UdpServer::new("127.0.0.1:9999").await.unwrap();
    let mut client = UdpClient::new().await.unwrap();
    client.connect("127.0.0.1:9999".parse().unwrap()).await.unwrap();

    let client_addr = client.local_addr().unwrap();
    server.register_client(1, client_addr).await;

    let start = Instant::now();
    let packet_count = 1000;

    // Send 1000 packets as fast as possible
    for i in 0..packet_count {
        let update = PositionUpdate::from_components(
            i as u16,
            i,
            glam::Vec3::ZERO,
            glam::Vec3::ZERO,
            glam::Quat::IDENTITY,
        );
        client.send(&update.to_bytes()).await.unwrap();
    }

    let elapsed = start.elapsed();
    let throughput = packet_count as f64 / elapsed.as_secs_f64();

    tracing::info!("UDP throughput: {:.0} packets/second", throughput);
    assert!(throughput > 500.0, "Throughput too low: {}", throughput);
}
```

---

## 📊 **Packet Size Analysis**

**PositionUpdate (19 bytes):**
- Entity ID: 2 bytes
- Sequence: 4 bytes
- Position (quantized): 6 bytes (3 × i16)
- Velocity (quantized): 3 bytes (3 × i8)
- Rotation (compressed): 4 bytes

**Batched Packet (1400 bytes max):**
- Can fit 73 position updates per packet
- Reduces overhead from 73 packets to 1 packet
- 73× reduction in packet count

**Comparison:**
- Uncompressed (FlatBuffers): ~80 bytes per update
- Compressed (this implementation): 19 bytes
- Compression ratio: 76% size reduction

---

**Dependencies:** [phase2-network-protocol.md](phase2-network-protocol.md), [phase2-tcp-connection.md](phase2-tcp-connection.md)
**Next:** [phase2-client-prediction.md](phase2-client-prediction.md)
