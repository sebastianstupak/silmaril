//! Channel priority and reliability benchmarks
//!
//! Benchmarks for:
//! - Ordered vs unordered delivery overhead
//! - Reliable vs unreliable channel performance
//! - Priority queue operations (<1µs)
//! - Head-of-line blocking measurement
//! - Reliability tracking overhead

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::time::Instant;

// ============================================================================
// Channel Types
// ============================================================================

/// Message reliability guarantee
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reliability {
    /// Fire-and-forget (UDP-like)
    Unreliable,
    /// Guaranteed delivery (TCP-like)
    Reliable,
}

/// Message ordering guarantee
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ordering {
    /// Messages may arrive out of order
    Unordered,
    /// Messages arrive in send order
    Ordered,
}

/// Message priority for queue scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Network message with metadata
#[derive(Debug, Clone, PartialEq, Eq)]
struct Message {
    sequence: u64,
    priority: Priority,
    reliability: Reliability,
    ordering: Ordering,
    data: Vec<u8>,
    timestamp: u64,
}

// ============================================================================
// Ordered Channel
// ============================================================================

/// Channel that guarantees message ordering
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
            // Expected message, deliver immediately
            let mut deliverable = vec![msg];
            self.recv_sequence += 1;

            // Check if we can deliver buffered messages
            while let Some(buffered) = self.out_of_order_buffer.remove(&self.recv_sequence) {
                deliverable.push(buffered);
                self.recv_sequence += 1;
            }

            Some(deliverable)
        } else if msg.sequence > self.recv_sequence {
            // Future message, buffer it
            self.out_of_order_buffer.insert(msg.sequence, msg);
            None
        } else {
            // Duplicate or old message, discard
            None
        }
    }
}

// ============================================================================
// Unordered Channel
// ============================================================================

/// Channel that allows out-of-order delivery
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
        // Deliver immediately, no ordering guarantee
        Some(msg)
    }
}

// ============================================================================
// Reliable Channel
// ============================================================================

/// Reliable message tracker
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
            rtt_ms: 50, // Initial estimate
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
            // Update RTT estimate
            let rtt = sent_time.elapsed().as_millis() as u64;
            self.rtt_ms = (self.rtt_ms * 7 + rtt) / 8; // Exponential moving average

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
}

// ============================================================================
// Unreliable Channel
// ============================================================================

/// Unreliable fire-and-forget channel
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

    fn receive(&mut self, msg: Message) -> Option<Message> {
        // No reliability tracking, just deliver
        Some(msg)
    }
}

// ============================================================================
// Priority Queue
// ============================================================================

/// Priority message entry for heap
#[derive(Debug, Clone, Eq, PartialEq)]
struct PriorityMessage {
    priority: Priority,
    sequence: u64,
    message: Message,
}

impl Ord for PriorityMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first, then older messages
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

/// Priority-based message queue
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

    #[allow(dead_code)]
    fn len(&self) -> usize {
        self.heap.len()
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn get_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn create_test_message(size: usize) -> Vec<u8> {
    vec![0u8; size]
}

// ============================================================================
// Benchmarks
// ============================================================================

fn bench_ordered_channel(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel/ordered");

    group.bench_function("send", |b| {
        let mut channel = OrderedChannel::new();
        let data = create_test_message(100);

        b.iter(|| {
            let msg = channel.send(black_box(data.clone()));
            black_box(msg);
        });
    });

    group.bench_function("receive_in_order", |b| {
        let mut send_channel = OrderedChannel::new();
        let messages: Vec<Message> =
            (0..100).map(|_| send_channel.send(create_test_message(100))).collect();

        b.iter(|| {
            let mut recv_channel = OrderedChannel::new();
            for msg in messages.iter() {
                let result = recv_channel.receive(msg.clone());
                black_box(result);
            }
        });
    });

    group.bench_function("receive_out_of_order", |b| {
        let mut send_channel = OrderedChannel::new();
        let mut messages: Vec<Message> =
            (0..100).map(|_| send_channel.send(create_test_message(100))).collect();

        // Shuffle messages to simulate out-of-order delivery
        for i in (0..messages.len()).step_by(2) {
            if i + 1 < messages.len() {
                messages.swap(i, i + 1);
            }
        }

        b.iter(|| {
            let mut recv_channel = OrderedChannel::new();
            for msg in messages.iter() {
                let result = recv_channel.receive(msg.clone());
                black_box(result);
            }
        });
    });

    group.bench_function("head_of_line_blocking", |b| {
        b.iter(|| {
            let mut recv_channel = OrderedChannel::new();

            // Receive messages 1-10, skip message 0 (simulates packet loss)
            for seq in 1..11 {
                let msg = Message {
                    sequence: seq,
                    priority: Priority::Normal,
                    reliability: Reliability::Reliable,
                    ordering: Ordering::Ordered,
                    data: create_test_message(100),
                    timestamp: get_timestamp_ms(),
                };

                let result = recv_channel.receive(msg);
                black_box(result);
            }

            // All messages should be buffered due to missing message 0
            assert_eq!(recv_channel.out_of_order_buffer.len(), 10);
        });
    });

    group.finish();
}

fn bench_unordered_channel(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel/unordered");

    group.bench_function("send", |b| {
        let mut channel = UnorderedChannel::new();
        let data = create_test_message(100);

        b.iter(|| {
            let msg = channel.send(black_box(data.clone()));
            black_box(msg);
        });
    });

    group.bench_function("receive", |b| {
        let mut send_channel = UnorderedChannel::new();
        let messages: Vec<Message> =
            (0..100).map(|_| send_channel.send(create_test_message(100))).collect();

        b.iter(|| {
            let mut recv_channel = UnorderedChannel::new();
            for msg in messages.iter() {
                let result = recv_channel.receive(msg.clone());
                black_box(result);
            }
        });
    });

    group.finish();
}

fn bench_reliable_channel(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel/reliable");

    group.bench_function("send", |b| {
        let mut channel = ReliableChannel::new();
        let data = create_test_message(100);

        b.iter(|| {
            let msg = channel.send(black_box(data.clone()));
            black_box(msg);
        });
    });

    group.bench_function("acknowledge", |b| {
        let mut channel = ReliableChannel::new();
        for _ in 0..100 {
            channel.send(create_test_message(100));
        }

        let mut seq = 0u64;
        b.iter(|| {
            channel.acknowledge(black_box(seq));
            seq += 1;
            if seq >= 100 {
                seq = 0;
                channel = ReliableChannel::new();
                for _ in 0..100 {
                    channel.send(create_test_message(100));
                }
            }
        });
    });

    group.bench_function("check_timeouts", |b| {
        let mut channel = ReliableChannel::new();

        b.iter(|| {
            // Send messages
            for _ in 0..100 {
                channel.send(create_test_message(100));
            }

            // Check for timeouts
            let retransmit = channel.check_timeouts(black_box(1000));
            black_box(retransmit);

            channel = ReliableChannel::new();
        });
    });

    group.bench_function("reliability_tracking_overhead", |b| {
        b.iter(|| {
            let mut channel = ReliableChannel::new();

            // Send 1000 messages
            for _ in 0..1000 {
                channel.send(create_test_message(100));
            }

            // Acknowledge half
            for seq in (0..500).step_by(2) {
                channel.acknowledge(seq);
            }

            // Check timeouts
            let retransmit = channel.check_timeouts(1000);

            black_box(retransmit);
        });
    });

    group.finish();
}

fn bench_unreliable_channel(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel/unreliable");

    group.bench_function("send", |b| {
        let mut channel = UnreliableChannel::new();
        let data = create_test_message(100);

        b.iter(|| {
            let msg = channel.send(black_box(data.clone()));
            black_box(msg);
        });
    });

    group.bench_function("receive", |b| {
        let mut send_channel = UnreliableChannel::new();
        let messages: Vec<Message> =
            (0..100).map(|_| send_channel.send(create_test_message(100))).collect();

        b.iter(|| {
            let mut recv_channel = UnreliableChannel::new();
            for msg in messages.iter() {
                let result = recv_channel.receive(msg.clone());
                black_box(result);
            }
        });
    });

    group.finish();
}

fn bench_priority_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel/priority");

    group.bench_function("insert_normal", |b| {
        let mut queue = PriorityQueue::new();
        let msg = Message {
            sequence: 0,
            priority: Priority::Normal,
            reliability: Reliability::Reliable,
            ordering: Ordering::Unordered,
            data: create_test_message(100),
            timestamp: get_timestamp_ms(),
        };

        b.iter(|| {
            queue.insert(black_box(msg.clone()), Priority::Normal);
        });
    });

    group.bench_function("insert_mixed_priority", |b| {
        let msg = Message {
            sequence: 0,
            priority: Priority::Normal,
            reliability: Reliability::Reliable,
            ordering: Ordering::Unordered,
            data: create_test_message(100),
            timestamp: get_timestamp_ms(),
        };

        let priorities = vec![Priority::Low, Priority::Normal, Priority::High, Priority::Critical];

        b.iter(|| {
            let mut queue = PriorityQueue::new();
            for (_i, prio) in priorities.iter().cycle().take(100).enumerate() {
                queue.insert(msg.clone(), *prio);
            }
            black_box(queue);
        });
    });

    group.bench_function("dequeue", |b| {
        b.iter(|| {
            let mut queue = PriorityQueue::new();
            let msg = Message {
                sequence: 0,
                priority: Priority::Normal,
                reliability: Reliability::Reliable,
                ordering: Ordering::Unordered,
                data: create_test_message(100),
                timestamp: get_timestamp_ms(),
            };

            // Insert messages with different priorities
            queue.insert(msg.clone(), Priority::Low);
            queue.insert(msg.clone(), Priority::Normal);
            queue.insert(msg.clone(), Priority::High);
            queue.insert(msg.clone(), Priority::Critical);

            // Dequeue all (should be in priority order)
            while let Some(dequeued) = queue.dequeue() {
                black_box(dequeued);
            }
        });
    });

    group.bench_function("priority_reordering", |b| {
        b.iter(|| {
            let mut queue = PriorityQueue::new();
            let msg = Message {
                sequence: 0,
                priority: Priority::Normal,
                reliability: Reliability::Reliable,
                ordering: Ordering::Unordered,
                data: create_test_message(100),
                timestamp: get_timestamp_ms(),
            };

            // Insert 100 normal priority
            for _ in 0..100 {
                queue.insert(msg.clone(), Priority::Normal);
            }

            // Insert 1 critical (should jump to front)
            queue.insert(msg.clone(), Priority::Critical);

            // First dequeue should be critical
            let first = queue.dequeue().unwrap();
            assert_eq!(first.priority, Priority::Critical);

            black_box(queue);
        });
    });

    group.finish();
}

fn bench_ordered_vs_unordered_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison/ordered_vs_unordered_latency");

    group.bench_function("ordered_worst_case", |b| {
        // Worst case: all messages arrive out of order
        b.iter(|| {
            let mut recv_channel = OrderedChannel::new();
            let messages: Vec<Message> = (0..100)
                .rev()
                .map(|seq| Message {
                    sequence: seq,
                    priority: Priority::Normal,
                    reliability: Reliability::Reliable,
                    ordering: Ordering::Ordered,
                    data: create_test_message(100),
                    timestamp: get_timestamp_ms(),
                })
                .collect();

            for msg in messages {
                let result = recv_channel.receive(msg);
                black_box(result);
            }
        });
    });

    group.bench_function("unordered_worst_case", |b| {
        // Unordered has no worst case - always same performance
        b.iter(|| {
            let mut recv_channel = UnorderedChannel::new();
            let messages: Vec<Message> = (0..100)
                .rev()
                .map(|seq| Message {
                    sequence: seq,
                    priority: Priority::Normal,
                    reliability: Reliability::Reliable,
                    ordering: Ordering::Unordered,
                    data: create_test_message(100),
                    timestamp: get_timestamp_ms(),
                })
                .collect();

            for msg in messages {
                let result = recv_channel.receive(msg);
                black_box(result);
            }
        });
    });

    group.finish();
}

fn bench_reliable_vs_unreliable_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison/reliable_vs_unreliable");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("reliable_full_cycle", |b| {
        b.iter(|| {
            let mut channel = ReliableChannel::new();

            // Send 1000 messages
            for _ in 0..1000 {
                channel.send(create_test_message(100));
            }

            // Acknowledge all
            for seq in 0..1000 {
                channel.acknowledge(seq);
            }

            // Check for timeouts
            let retransmit = channel.check_timeouts(1000);
            black_box(retransmit);
        });
    });

    group.bench_function("unreliable_full_cycle", |b| {
        b.iter(|| {
            let mut channel = UnreliableChannel::new();

            // Send 1000 messages (no acknowledgment needed)
            for _ in 0..1000 {
                channel.send(create_test_message(100));
            }
        });
    });

    group.finish();
}

fn bench_mixed_priority_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("channel/mixed_priority_throughput");

    for priority_ratio in [0, 25, 50, 75, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("critical_percent", priority_ratio),
            priority_ratio,
            |b, &ratio| {
                let msg = Message {
                    sequence: 0,
                    priority: Priority::Normal,
                    reliability: Reliability::Reliable,
                    ordering: Ordering::Unordered,
                    data: create_test_message(100),
                    timestamp: get_timestamp_ms(),
                };

                b.iter(|| {
                    let mut queue = PriorityQueue::new();

                    for i in 0..1000 {
                        let priority =
                            if (i % 100) < ratio { Priority::Critical } else { Priority::Normal };
                        queue.insert(msg.clone(), priority);
                    }

                    // Dequeue all
                    while let Some(dequeued) = queue.dequeue() {
                        black_box(dequeued);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    channel_benches,
    bench_ordered_channel,
    bench_unordered_channel,
    bench_reliable_channel,
    bench_unreliable_channel,
    bench_priority_queue,
    bench_ordered_vs_unordered_latency,
    bench_reliable_vs_unreliable_overhead,
    bench_mixed_priority_throughput,
);

criterion_main!(channel_benches);
