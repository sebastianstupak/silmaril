//! Event system for ECS
//!
//! Allows systems to send and receive typed events without tight coupling.
//! Events are stored in ring buffers and can be read by multiple systems.

use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

/// Maximum events to store per type before oldest are dropped
const MAX_EVENTS_PER_TYPE: usize = 1024;

/// Event that can be sent through the event system
pub trait Event: Send + Sync + 'static {}

/// Storage for a single event type
struct EventQueue {
    /// Ring buffer of events (boxed for type erasure)
    events: VecDeque<Box<dyn Any + Send + Sync>>,
    /// Maximum capacity
    capacity: usize,
}

impl EventQueue {
    fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push<E: Event>(&mut self, event: E) {
        // Drop oldest if at capacity
        if self.events.len() >= self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(Box::new(event));
    }

    fn clear(&mut self) {
        self.events.clear();
    }
}

/// Central event storage for the ECS World
pub struct Events {
    /// Map from TypeId to event queue
    queues: HashMap<TypeId, EventQueue>,
}

impl Events {
    /// Create new event storage
    pub fn new() -> Self {
        Self {
            queues: HashMap::new(),
        }
    }

    /// Send an event
    pub fn send<E: Event>(&mut self, event: E) {
        let type_id = TypeId::of::<E>();

        let queue = self.queues.entry(type_id).or_insert_with(|| {
            EventQueue::new(MAX_EVENTS_PER_TYPE)
        });

        queue.push(event);
    }

    /// Get event reader for a specific event type
    pub fn get_reader<E: Event>(&self) -> EventReader<E> {
        let type_id = TypeId::of::<E>();
        let count = self.queues
            .get(&type_id)
            .map(|q| q.events.len())
            .unwrap_or(0);

        EventReader {
            last_read: 0,
            _phantom: PhantomData,
            total_events: count,
        }
    }

    /// Read events with a reader (returns iterator)
    pub fn read<'a, E: Event>(&'a self, reader: &mut EventReader<E>) -> EventIter<'a, E> {
        let type_id = TypeId::of::<E>();

        let queue = self.queues.get(&type_id);
        let start = reader.last_read;

        if let Some(queue) = queue {
            reader.last_read = queue.events.len();
            EventIter {
                events: &queue.events,
                current: start,
                end: queue.events.len(),
                _phantom: PhantomData,
            }
        } else {
            EventIter {
                events: &VecDeque::new(),
                current: 0,
                end: 0,
                _phantom: PhantomData,
            }
        }
    }

    /// Clear all events of a specific type
    pub fn clear<E: Event>(&mut self) {
        let type_id = TypeId::of::<E>();
        if let Some(queue) = self.queues.get_mut(&type_id) {
            queue.clear();
        }
    }

    /// Clear all events
    pub fn clear_all(&mut self) {
        for queue in self.queues.values_mut() {
            queue.clear();
        }
    }
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

/// Event reader tracks which events have been read
///
/// Multiple readers can exist for the same event type,
/// each tracking their own read position.
pub struct EventReader<E: Event> {
    last_read: usize,
    total_events: usize,
    _phantom: PhantomData<E>,
}

impl<E: Event> EventReader<E> {
    /// Create new event reader (starts at current position)
    pub fn new() -> Self {
        Self {
            last_read: 0,
            total_events: 0,
            _phantom: PhantomData,
        }
    }

    /// Reset reader to read all events from the beginning
    pub fn reset(&mut self) {
        self.last_read = 0;
    }

    /// Number of events that haven't been read yet
    pub fn len(&self) -> usize {
        self.total_events.saturating_sub(self.last_read)
    }

    /// Check if there are unread events
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<E: Event> Default for EventReader<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterator over events
pub struct EventIter<'a, E: Event> {
    events: &'a VecDeque<Box<dyn Any + Send + Sync>>,
    current: usize,
    end: usize,
    _phantom: PhantomData<E>,
}

impl<'a, E: Event> Iterator for EventIter<'a, E> {
    type Item = &'a E;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.end {
            return None;
        }

        let event = self.events.get(self.current)?;
        self.current += 1;

        // Downcast to concrete type
        event.downcast_ref::<E>()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end.saturating_sub(self.current);
        (remaining, Some(remaining))
    }
}

impl<'a, E: Event> ExactSizeIterator for EventIter<'a, E> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestEvent {
        value: i32,
    }
    impl Event for TestEvent {}

    #[derive(Debug, Clone, PartialEq)]
    struct OtherEvent {
        name: String,
    }
    impl Event for OtherEvent {}

    #[test]
    fn test_send_and_read_events() {
        let mut events = Events::new();
        let mut reader = events.get_reader::<TestEvent>();

        events.send(TestEvent { value: 1 });
        events.send(TestEvent { value: 2 });
        events.send(TestEvent { value: 3 });

        let received: Vec<_> = events.read(&mut reader).collect();
        assert_eq!(received.len(), 3);
        assert_eq!(received[0].value, 1);
        assert_eq!(received[1].value, 2);
        assert_eq!(received[2].value, 3);

        // Reading again should return nothing
        let received: Vec<_> = events.read(&mut reader).collect();
        assert_eq!(received.len(), 0);
    }

    #[test]
    fn test_multiple_readers() {
        let mut events = Events::new();
        let mut reader1 = events.get_reader::<TestEvent>();
        let mut reader2 = events.get_reader::<TestEvent>();

        events.send(TestEvent { value: 1 });
        events.send(TestEvent { value: 2 });

        // Both readers should see all events
        let r1_events: Vec<_> = events.read(&mut reader1).collect();
        let r2_events: Vec<_> = events.read(&mut reader2).collect();

        assert_eq!(r1_events.len(), 2);
        assert_eq!(r2_events.len(), 2);
    }

    #[test]
    fn test_different_event_types() {
        let mut events = Events::new();
        let mut test_reader = events.get_reader::<TestEvent>();
        let mut other_reader = events.get_reader::<OtherEvent>();

        events.send(TestEvent { value: 42 });
        events.send(OtherEvent { name: "test".to_string() });

        let test_events: Vec<_> = events.read(&mut test_reader).collect();
        let other_events: Vec<_> = events.read(&mut other_reader).collect();

        assert_eq!(test_events.len(), 1);
        assert_eq!(test_events[0].value, 42);

        assert_eq!(other_events.len(), 1);
        assert_eq!(other_events[0].name, "test");
    }

    #[test]
    fn test_clear_events() {
        let mut events = Events::new();
        let mut reader = events.get_reader::<TestEvent>();

        events.send(TestEvent { value: 1 });
        events.send(TestEvent { value: 2 });

        events.clear::<TestEvent>();

        let received: Vec<_> = events.read(&mut reader).collect();
        assert_eq!(received.len(), 0);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut events = Events::new();

        // Send more than MAX_EVENTS_PER_TYPE
        for i in 0..(MAX_EVENTS_PER_TYPE + 100) {
            events.send(TestEvent { value: i as i32 });
        }

        let mut reader = EventReader::new();
        let received: Vec<_> = events.read(&mut reader).collect();

        // Should only have MAX_EVENTS_PER_TYPE events
        assert_eq!(received.len(), MAX_EVENTS_PER_TYPE);

        // Should be the most recent events
        assert_eq!(received[0].value, 100);
        assert_eq!(received.last().unwrap().value, (MAX_EVENTS_PER_TYPE + 99) as i32);
    }

    #[test]
    fn test_reader_reset() {
        let mut events = Events::new();
        let mut reader = events.get_reader::<TestEvent>();

        events.send(TestEvent { value: 1 });
        events.send(TestEvent { value: 2 });

        // Read once
        let _: Vec<_> = events.read(&mut reader).collect();

        // Reset and read again
        reader.reset();
        let received: Vec<_> = events.read(&mut reader).collect();
        assert_eq!(received.len(), 2);
    }
}
