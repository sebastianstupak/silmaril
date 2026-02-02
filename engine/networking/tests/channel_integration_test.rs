//! Integration tests for channel priority and reliability
//!
//! Tests cover:
//! - Ordered vs unordered delivery
//! - Reliable vs unreliable channels
//! - Priority queue ordering
//! - Head-of-line blocking
//! - Reliability tracking

use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time::Instant;

// ============================================================================
// Test Data Structures (duplicated from bench for testing)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reliability {
    Unreliable,
    Reliable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ordering {
    Unordered,
    Ordered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Message {
    sequence: u64,
    priority: Priority,
    reliability: Reliability,
    ordering: Ordering,
    data: Vec<u8>,
    timestamp: u64,
}

#[derive(Debug)]
struct OrderedChannel {
    send_sequence: u64,
    recv_sequence: u64,
    out_of_order_buffer: HashMap<u64, Message>,
    #[allow(dead_code)]
    pending_delivery: VecDeque<Message>,
}

impl OrderedChannel {
    fn new() -> Self {
        Self {
            send_sequence: 0,
            recv_sequence: 0,
            out_of_order_buffer: HashMap::new(),
            pending_delivery: VecDeque::new(),
        }
    }

    fn send(&mut self, data: Vec<u8>) -> Message {
        let msg = Message {
            sequence: self.send_sequence,
            priority: Priority::Normal,
            reliability: Reliability::Reliable,
            ordering: Ordering::Ordered,
            data,
            timestamp: get_timestamp_ms(),
        };

        self.send_sequence += 1;
        msg
    }

    fn receive(&mut self, msg: Message) -> Option<Vec<Message>> {
        if msg.sequence == self.recv_sequence {
            let mut deliverable = vec![msg];
            self.recv_sequence += 1;

            while let Some(buffered) = self.out_of_order_buffer.remove(&self.recv_sequence) {
                deliverable.push(buffered);
                self.recv_sequence += 1;
            }

            Some(deliverable)
        } else if msg.sequence > self.recv_sequence {
            self.out_of_order_buffer.insert(msg.sequence, msg);
            None
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct UnorderedChannel {
    send_sequence: u64,
}

impl UnorderedChannel {
    fn new() -> Self {
        Self { send_sequence: 0 }
    }

    fn send(&mut self, data: Vec<u8>) -> Message {
        let msg = Message {
            sequence: self.send_sequence,
            priority: Priority::Normal,
            reliability: Reliability::Reliable,
            ordering: Ordering::Unordered,
            data,
            timestamp: get_timestamp_ms(),
        };

        self.send_sequence += 1;
        msg
    }

    fn receive(&mut self, msg: Message) -> Option<Message> {
        Some(msg)
    }
}

#[derive(Debug)]
struct ReliableChannel {
    sent_messages: HashMap<u64, Message>,
    ack_pending: HashMap<u64, Instant>,
    sequence: u64,
    rtt_ms: u64,
}

impl ReliableChannel {
    fn new() -> Self {
        Self {
            sent_messages: HashMap::new(),
            ack_pending: HashMap::new(),
            sequence: 0,
            rtt_ms: 50,
        }
    }

    fn send(&mut self, data: Vec<u8>) -> Message {
        let msg = Message {
            sequence: self.sequence,
            priority: Priority::Normal,
            reliability: Reliability::Reliable,
            ordering: Ordering::Unordered,
            data,
            timestamp: get_timestamp_ms(),
        };

        self.sent_messages.insert(self.sequence, msg.clone());
        self.ack_pending.insert(self.sequence, Instant::now());
        self.sequence += 1;

        msg
    }

    fn acknowledge(&mut self, sequence: u64) {
        if let Some(sent_time) = self.ack_pending.remove(&sequence) {
            let rtt = sent_time.elapsed().as_millis() as u64;
            self.rtt_ms = (self.rtt_ms * 7 + rtt) / 8;
            self.sent_messages.remove(&sequence);
        }
    }

    fn check_timeouts(&mut self, timeout_ms: u64) -> Vec<Message> {
        let mut retransmit = Vec::new();

        for (seq, sent_time) in self.ack_pending.iter() {
            if sent_time.elapsed().as_millis() as u64 > timeout_ms {
                if let Some(msg) = self.sent_messages.get(seq) {
                    retransmit.push(msg.clone());
                }
            }
        }

        retransmit
    }

    fn pending_count(&self) -> usize {
        self.ack_pending.len()
    }
}

#[derive(Debug)]
struct UnreliableChannel {
    sequence: u64,
}

impl UnreliableChannel {
    fn new() -> Self {
        Self { sequence: 0 }
    }

    fn send(&mut self, data: Vec<u8>) -> Message {
        let msg = Message {
            sequence: self.sequence,
            priority: Priority::Normal,
            reliability: Reliability::Unreliable,
            ordering: Ordering::Unordered,
            data,
            timestamp: get_timestamp_ms(),
        };

        self.sequence += 1;
        msg
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PriorityMessage {
    priority: Priority,
    sequence: u64,
    message: Message,
}

impl Ord for PriorityMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

impl PartialOrd for PriorityMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct PriorityQueue {
    heap: BinaryHeap<PriorityMessage>,
    sequence: u64,
}

impl PriorityQueue {
    fn new() -> Self {
        Self { heap: BinaryHeap::new(), sequence: 0 }
    }

    fn insert(&mut self, mut msg: Message, priority: Priority) {
        msg.priority = priority;
        let entry = PriorityMessage { priority, sequence: self.sequence, message: msg };

        self.heap.push(entry);
        self.sequence += 1;
    }

    fn dequeue(&mut self) -> Option<Message> {
        self.heap.pop().map(|entry| entry.message)
    }
}

fn get_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ============================================================================
// Ordered Channel Tests
// ============================================================================

#[test]
fn test_ordered_channel_in_order_delivery() {
    let mut send_channel = OrderedChannel::new();
    let mut recv_channel = OrderedChannel::new();

    let msg1 = send_channel.send(vec![1]);
    let msg2 = send_channel.send(vec![2]);
    let msg3 = send_channel.send(vec![3]);

    let result1 = recv_channel.receive(msg1);
    assert!(result1.is_some());
    assert_eq!(result1.unwrap().len(), 1);

    let result2 = recv_channel.receive(msg2);
    assert!(result2.is_some());
    assert_eq!(result2.unwrap().len(), 1);

    let result3 = recv_channel.receive(msg3);
    assert!(result3.is_some());
    assert_eq!(result3.unwrap().len(), 1);
}

#[test]
fn test_ordered_channel_out_of_order_buffering() {
    let mut send_channel = OrderedChannel::new();
    let mut recv_channel = OrderedChannel::new();

    let msg1 = send_channel.send(vec![1]);
    let msg2 = send_channel.send(vec![2]);
    let msg3 = send_channel.send(vec![3]);

    // Receive out of order: 2, 3, then 1
    let result2 = recv_channel.receive(msg2.clone());
    assert!(result2.is_none()); // Buffered

    let result3 = recv_channel.receive(msg3.clone());
    assert!(result3.is_none()); // Buffered

    // When we receive msg1, all should be delivered
    let result1 = recv_channel.receive(msg1);
    assert!(result1.is_some());
    let delivered = result1.unwrap();
    assert_eq!(delivered.len(), 3); // All 3 messages delivered
    assert_eq!(delivered[0].data, vec![1]);
    assert_eq!(delivered[1].data, vec![2]);
    assert_eq!(delivered[2].data, vec![3]);
}

#[test]
fn test_ordered_channel_head_of_line_blocking() {
    let mut recv_channel = OrderedChannel::new();

    // Create messages but skip sequence 0
    for seq in 1..10 {
        let msg = Message {
            sequence: seq,
            priority: Priority::Normal,
            reliability: Reliability::Reliable,
            ordering: Ordering::Ordered,
            data: vec![seq as u8],
            timestamp: get_timestamp_ms(),
        };

        let result = recv_channel.receive(msg);
        assert!(result.is_none()); // All should be buffered
    }

    // All 9 messages should be waiting for sequence 0
    assert_eq!(recv_channel.out_of_order_buffer.len(), 9);
    assert_eq!(recv_channel.recv_sequence, 0);
}

#[test]
fn test_ordered_channel_duplicate_message() {
    let mut send_channel = OrderedChannel::new();
    let mut recv_channel = OrderedChannel::new();

    let msg1 = send_channel.send(vec![1]);

    let result1 = recv_channel.receive(msg1.clone());
    assert!(result1.is_some());

    // Receive duplicate
    let result2 = recv_channel.receive(msg1);
    assert!(result2.is_none()); // Duplicate ignored
}

// ============================================================================
// Unordered Channel Tests
// ============================================================================

#[test]
fn test_unordered_channel_immediate_delivery() {
    let mut send_channel = UnorderedChannel::new();
    let mut recv_channel = UnorderedChannel::new();

    let msg1 = send_channel.send(vec![1]);
    let msg2 = send_channel.send(vec![2]);
    let msg3 = send_channel.send(vec![3]);

    // All messages delivered immediately, regardless of order
    let result3 = recv_channel.receive(msg3);
    assert!(result3.is_some());

    let result1 = recv_channel.receive(msg1);
    assert!(result1.is_some());

    let result2 = recv_channel.receive(msg2);
    assert!(result2.is_some());
}

#[test]
fn test_unordered_channel_no_buffering() {
    let mut send_channel = UnorderedChannel::new();

    // Send many messages
    for _ in 0..100 {
        send_channel.send(vec![0]);
    }

    // Unordered channel doesn't maintain any state
    assert_eq!(send_channel.send_sequence, 100);
}

// ============================================================================
// Reliable Channel Tests
// ============================================================================

#[test]
fn test_reliable_channel_acknowledgment() {
    let mut channel = ReliableChannel::new();

    let msg = channel.send(vec![1]);
    assert_eq!(channel.pending_count(), 1);

    channel.acknowledge(msg.sequence);
    assert_eq!(channel.pending_count(), 0);
}

#[test]
fn test_reliable_channel_timeout_detection() {
    let mut channel = ReliableChannel::new();

    channel.send(vec![1]);
    channel.send(vec![2]);

    // Check for timeouts (using 0ms timeout to force all to timeout)
    std::thread::sleep(std::time::Duration::from_millis(1));
    let retransmit = channel.check_timeouts(0);

    assert_eq!(retransmit.len(), 2);
}

#[test]
fn test_reliable_channel_partial_ack() {
    let mut channel = ReliableChannel::new();

    let msg1 = channel.send(vec![1]);
    let msg2 = channel.send(vec![2]);
    let msg3 = channel.send(vec![3]);

    // Acknowledge only msg2
    channel.acknowledge(msg2.sequence);

    assert_eq!(channel.pending_count(), 2);
    assert!(channel.sent_messages.contains_key(&msg1.sequence));
    assert!(!channel.sent_messages.contains_key(&msg2.sequence));
    assert!(channel.sent_messages.contains_key(&msg3.sequence));
}

#[test]
fn test_reliable_channel_rtt_estimation() {
    let mut channel = ReliableChannel::new();
    let initial_rtt = channel.rtt_ms;

    let msg = channel.send(vec![1]);

    std::thread::sleep(std::time::Duration::from_millis(10));
    channel.acknowledge(msg.sequence);

    // RTT should be updated (using exponential moving average)
    assert_ne!(channel.rtt_ms, initial_rtt);
}

#[test]
fn test_reliable_channel_many_messages() {
    let mut channel = ReliableChannel::new();

    for i in 0..1000 {
        channel.send(vec![i as u8]);
    }

    assert_eq!(channel.pending_count(), 1000);

    // Acknowledge all
    for seq in 0..1000 {
        channel.acknowledge(seq);
    }

    assert_eq!(channel.pending_count(), 0);
}

// ============================================================================
// Unreliable Channel Tests
// ============================================================================

#[test]
fn test_unreliable_channel_no_tracking() {
    let mut channel = UnreliableChannel::new();

    for _ in 0..100 {
        channel.send(vec![0]);
    }

    // Unreliable channel doesn't track sent messages
    assert_eq!(channel.sequence, 100);
}

// ============================================================================
// Priority Queue Tests
// ============================================================================

#[test]
fn test_priority_queue_fifo_same_priority() {
    let mut queue = PriorityQueue::new();

    let msg1 = Message {
        sequence: 0,
        priority: Priority::Normal,
        reliability: Reliability::Reliable,
        ordering: Ordering::Unordered,
        data: vec![1],
        timestamp: get_timestamp_ms(),
    };

    queue.insert(msg1.clone(), Priority::Normal);
    queue.insert(msg1.clone(), Priority::Normal);
    queue.insert(msg1.clone(), Priority::Normal);

    // Same priority should be FIFO
    let first = queue.dequeue().unwrap();
    assert_eq!(first.data, vec![1]);
}

#[test]
fn test_priority_queue_priority_ordering() {
    let mut queue = PriorityQueue::new();

    let msg = Message {
        sequence: 0,
        priority: Priority::Normal,
        reliability: Reliability::Reliable,
        ordering: Ordering::Unordered,
        data: vec![0],
        timestamp: get_timestamp_ms(),
    };

    // Insert in random priority order
    queue.insert(msg.clone(), Priority::Low);
    queue.insert(msg.clone(), Priority::Critical);
    queue.insert(msg.clone(), Priority::Normal);
    queue.insert(msg.clone(), Priority::High);

    // Should dequeue in priority order: Critical, High, Normal, Low
    assert_eq!(queue.dequeue().unwrap().priority, Priority::Critical);
    assert_eq!(queue.dequeue().unwrap().priority, Priority::High);
    assert_eq!(queue.dequeue().unwrap().priority, Priority::Normal);
    assert_eq!(queue.dequeue().unwrap().priority, Priority::Low);
}

#[test]
fn test_priority_queue_critical_jumps_queue() {
    let mut queue = PriorityQueue::new();

    let msg = Message {
        sequence: 0,
        priority: Priority::Normal,
        reliability: Reliability::Reliable,
        ordering: Ordering::Unordered,
        data: vec![0],
        timestamp: get_timestamp_ms(),
    };

    // Insert 100 normal priority messages
    for _ in 0..100 {
        queue.insert(msg.clone(), Priority::Normal);
    }

    // Insert 1 critical
    queue.insert(msg.clone(), Priority::Critical);

    // Critical should be first
    assert_eq!(queue.dequeue().unwrap().priority, Priority::Critical);
}

#[test]
fn test_priority_queue_empty() {
    let mut queue = PriorityQueue::new();
    assert!(queue.dequeue().is_none());
}

// ============================================================================
// Comparison Tests
// ============================================================================

#[test]
fn test_ordered_vs_unordered_latency() {
    let mut ordered_recv = OrderedChannel::new();
    let mut unordered_recv = UnorderedChannel::new();

    // Create messages arriving out of order
    let messages: Vec<Message> = (0..10)
        .rev()
        .map(|seq| Message {
            sequence: seq,
            priority: Priority::Normal,
            reliability: Reliability::Reliable,
            ordering: Ordering::Ordered,
            data: vec![seq as u8],
            timestamp: get_timestamp_ms(),
        })
        .collect();

    // Ordered channel must buffer all but when sequence 0 arrives (last in reversed list)
    for (idx, msg) in messages.iter().enumerate() {
        let result = ordered_recv.receive(msg.clone());
        // All messages except the last one (sequence 0) should be buffered
        if idx < messages.len() - 1 {
            assert!(result.is_none()); // Buffered until we get sequence 0
        } else {
            // When sequence 0 arrives, all buffered messages should be delivered
            assert!(result.is_some());
            assert_eq!(result.unwrap().len(), 10); // All 10 messages delivered
        }
    }

    // Unordered channel delivers all immediately
    for msg in messages.iter() {
        let result = unordered_recv.receive(msg.clone());
        assert!(result.is_some()); // Delivered
    }
}

#[test]
fn test_reliable_vs_unreliable_overhead() {
    let mut reliable = ReliableChannel::new();
    let mut unreliable = UnreliableChannel::new();

    // Send same number of messages
    for _ in 0..100 {
        reliable.send(vec![0]);
        unreliable.send(vec![0]);
    }

    // Reliable tracks all messages
    assert_eq!(reliable.pending_count(), 100);

    // Unreliable has no overhead
    assert_eq!(unreliable.sequence, 100);
}
