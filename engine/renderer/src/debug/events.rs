//! Rendering Event Stream (R.2)
//!
//! Event recording system for tracking resource lifecycle, errors, and
//! performance events in the rendering pipeline.
//!
//! # Overview
//!
//! This module implements a thread-safe event recording system for capturing:
//! - Resource lifecycle events (texture/buffer creation/destruction)
//! - Pipeline events (shader compilation, pipeline creation)
//! - Draw call events (submission, failures)
//! - Synchronization events (fence timeouts, swapchain recreation)
//! - Performance events (frame drops, memory exhaustion)
//!
//! # Example
//!
//! ```no_run
//! use engine_renderer::debug::{EventRecorder, RenderEvent};
//!
//! // Create event recorder
//! let mut recorder = EventRecorder::new();
//! recorder.enable();
//!
//! // Record events
//! recorder.record(RenderEvent::TextureCreated {
//!     texture_id: 42,
//!     width: 1024,
//!     height: 1024,
//!     format: "RGBA8".to_string(),
//!     memory_size: 4 * 1024 * 1024,
//!     frame: 1,
//!     timestamp: 0.016,
//! });
//!
//! // Drain events for analysis
//! let events = recorder.drain();
//! println!("Recorded {} events", events.len());
//! ```

#![allow(missing_docs)] // Debug infrastructure - comprehensive docs not required

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Rendering events for debugging and analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RenderEvent {
    // Resource lifecycle events
    /// Texture resource created
    TextureCreated {
        texture_id: u64,
        width: u32,
        height: u32,
        format: String,
        memory_size: usize,
        frame: u64,
        timestamp: f64,
    },

    /// Texture resource destroyed
    TextureDestroyed { texture_id: u64, frame: u64, timestamp: f64 },

    /// Buffer resource created
    BufferCreated { buffer_id: u64, size: usize, usage: String, frame: u64, timestamp: f64 },

    /// Buffer resource destroyed
    BufferDestroyed { buffer_id: u64, frame: u64, timestamp: f64 },

    // Pipeline events
    /// Pipeline created successfully
    PipelineCreated {
        pipeline_id: u64,
        vertex_shader: String,
        fragment_shader: String,
        frame: u64,
        timestamp: f64,
    },

    /// Shader compilation failed
    ShaderCompilationFailed {
        shader_path: String,
        error_message: String,
        frame: u64,
        timestamp: f64,
    },

    // Draw call events
    /// Draw call submitted successfully
    DrawCallSubmitted {
        draw_call_id: u64,
        mesh_id: u64,
        material_id: u64,
        vertex_count: u32,
        frame: u64,
        timestamp: f64,
    },

    /// Draw call failed
    DrawCallFailed { draw_call_id: u64, error: String, frame: u64, timestamp: f64 },

    // Synchronization events
    /// Fence wait timed out
    FenceWaitTimeout { fence_id: u64, timeout_ms: u64, frame: u64, timestamp: f64 },

    /// Swapchain was recreated (e.g., window resize)
    SwapchainRecreated {
        reason: String,
        old_width: u32,
        old_height: u32,
        new_width: u32,
        new_height: u32,
        frame: u64,
        timestamp: f64,
    },

    // Performance events
    /// Frame took longer than expected (dropped frame)
    FrameDropped {
        expected_frame_time_ms: f32,
        actual_frame_time_ms: f32,
        frame: u64,
        timestamp: f64,
    },

    /// GPU memory exhausted (allocation failed)
    GpuMemoryExhausted { requested_size: usize, available_size: usize, frame: u64, timestamp: f64 },
}

impl RenderEvent {
    /// Check if this event is critical (requires immediate attention)
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            RenderEvent::ShaderCompilationFailed { .. }
                | RenderEvent::DrawCallFailed { .. }
                | RenderEvent::FenceWaitTimeout { .. }
                | RenderEvent::GpuMemoryExhausted { .. }
        )
    }

    /// Check if this event is related to resource lifecycle
    pub fn is_resource_event(&self) -> bool {
        matches!(
            self,
            RenderEvent::TextureCreated { .. }
                | RenderEvent::TextureDestroyed { .. }
                | RenderEvent::BufferCreated { .. }
                | RenderEvent::BufferDestroyed { .. }
        )
    }

    /// Check if this event represents an error condition
    pub fn is_error_event(&self) -> bool {
        matches!(
            self,
            RenderEvent::ShaderCompilationFailed { .. }
                | RenderEvent::DrawCallFailed { .. }
                | RenderEvent::FenceWaitTimeout { .. }
                | RenderEvent::GpuMemoryExhausted { .. }
        )
    }

    /// Get the frame number this event occurred on
    pub fn frame(&self) -> u64 {
        match self {
            RenderEvent::TextureCreated { frame, .. }
            | RenderEvent::TextureDestroyed { frame, .. }
            | RenderEvent::BufferCreated { frame, .. }
            | RenderEvent::BufferDestroyed { frame, .. }
            | RenderEvent::PipelineCreated { frame, .. }
            | RenderEvent::ShaderCompilationFailed { frame, .. }
            | RenderEvent::DrawCallSubmitted { frame, .. }
            | RenderEvent::DrawCallFailed { frame, .. }
            | RenderEvent::FenceWaitTimeout { frame, .. }
            | RenderEvent::SwapchainRecreated { frame, .. }
            | RenderEvent::FrameDropped { frame, .. }
            | RenderEvent::GpuMemoryExhausted { frame, .. } => *frame,
        }
    }

    /// Get the timestamp this event occurred at
    pub fn timestamp(&self) -> f64 {
        match self {
            RenderEvent::TextureCreated { timestamp, .. }
            | RenderEvent::TextureDestroyed { timestamp, .. }
            | RenderEvent::BufferCreated { timestamp, .. }
            | RenderEvent::BufferDestroyed { timestamp, .. }
            | RenderEvent::PipelineCreated { timestamp, .. }
            | RenderEvent::ShaderCompilationFailed { timestamp, .. }
            | RenderEvent::DrawCallSubmitted { timestamp, .. }
            | RenderEvent::DrawCallFailed { timestamp, .. }
            | RenderEvent::FenceWaitTimeout { timestamp, .. }
            | RenderEvent::SwapchainRecreated { timestamp, .. }
            | RenderEvent::FrameDropped { timestamp, .. }
            | RenderEvent::GpuMemoryExhausted { timestamp, .. } => *timestamp,
        }
    }

    /// Get resource IDs involved in this event (texture_id, buffer_id, etc.)
    pub fn involved_resources(&self) -> Vec<u64> {
        match self {
            RenderEvent::TextureCreated { texture_id, .. }
            | RenderEvent::TextureDestroyed { texture_id, .. } => vec![*texture_id],
            RenderEvent::BufferCreated { buffer_id, .. }
            | RenderEvent::BufferDestroyed { buffer_id, .. } => vec![*buffer_id],
            RenderEvent::PipelineCreated { pipeline_id, .. } => vec![*pipeline_id],
            RenderEvent::DrawCallSubmitted { draw_call_id, mesh_id, material_id, .. } => {
                vec![*draw_call_id, *mesh_id, *material_id]
            }
            RenderEvent::DrawCallFailed { draw_call_id, .. } => vec![*draw_call_id],
            RenderEvent::FenceWaitTimeout { fence_id, .. } => vec![*fence_id],
            _ => vec![],
        }
    }
}

/// Event recorder for collecting rendering events
pub struct EventRecorder {
    /// Recorded events (thread-safe)
    events: Mutex<Vec<RenderEvent>>,

    /// Whether recording is enabled
    enabled: Mutex<bool>,

    /// Total number of events recorded (including drained)
    total_events: Mutex<usize>,
}

impl EventRecorder {
    /// Create a new event recorder (initially disabled)
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            enabled: Mutex::new(false),
            total_events: Mutex::new(0),
        }
    }

    /// Enable event recording
    pub fn enable(&self) {
        let mut enabled = self.enabled.lock().unwrap();
        *enabled = true;
    }

    /// Disable event recording
    pub fn disable(&self) {
        let mut enabled = self.enabled.lock().unwrap();
        *enabled = false;
    }

    /// Check if recording is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    /// Record a new event (only if enabled)
    pub fn record(&self, event: RenderEvent) {
        let enabled = self.enabled.lock().unwrap();
        if *enabled {
            drop(enabled); // Release lock before acquiring events lock
            let mut events = self.events.lock().unwrap();
            events.push(event);
            let mut total = self.total_events.lock().unwrap();
            *total += 1;
        }
    }

    /// Drain all recorded events (consume and return)
    pub fn drain(&self) -> Vec<RenderEvent> {
        let mut events = self.events.lock().unwrap();
        std::mem::take(&mut *events)
    }

    /// Get number of currently recorded events (not yet drained)
    pub fn event_count(&self) -> usize {
        let events = self.events.lock().unwrap();
        events.len()
    }

    /// Get total number of events recorded (including drained)
    pub fn total_event_count(&self) -> usize {
        *self.total_events.lock().unwrap()
    }

    /// Clear all recorded events without draining
    pub fn clear(&self) {
        let mut events = self.events.lock().unwrap();
        events.clear();
    }
}

impl Default for EventRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_recorder_creation() {
        let recorder = EventRecorder::new();
        assert!(!recorder.is_enabled());
        assert_eq!(recorder.event_count(), 0);
        assert_eq!(recorder.total_event_count(), 0);
    }

    #[test]
    fn test_enable_disable_recording() {
        let recorder = EventRecorder::new();
        assert!(!recorder.is_enabled());

        recorder.enable();
        assert!(recorder.is_enabled());

        recorder.disable();
        assert!(!recorder.is_enabled());
    }

    #[test]
    fn test_record_when_disabled() {
        let recorder = EventRecorder::new();
        // Recording is disabled by default

        recorder.record(RenderEvent::TextureCreated {
            texture_id: 1,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 1,
            timestamp: 0.016,
        });

        assert_eq!(recorder.event_count(), 0);
        assert_eq!(recorder.total_event_count(), 0);
    }

    #[test]
    fn test_record_when_enabled() {
        let recorder = EventRecorder::new();
        recorder.enable();

        recorder.record(RenderEvent::TextureCreated {
            texture_id: 1,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 1,
            timestamp: 0.016,
        });

        assert_eq!(recorder.event_count(), 1);
        assert_eq!(recorder.total_event_count(), 1);
    }

    #[test]
    fn test_drain_events() {
        let recorder = EventRecorder::new();
        recorder.enable();

        recorder.record(RenderEvent::TextureCreated {
            texture_id: 1,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 1,
            timestamp: 0.016,
        });

        recorder.record(RenderEvent::BufferCreated {
            buffer_id: 2,
            size: 1024,
            usage: "VERTEX".to_string(),
            frame: 1,
            timestamp: 0.017,
        });

        assert_eq!(recorder.event_count(), 2);

        let events = recorder.drain();
        assert_eq!(events.len(), 2);
        assert_eq!(recorder.event_count(), 0); // Events drained
        assert_eq!(recorder.total_event_count(), 2); // Total still counts drained events
    }

    #[test]
    fn test_event_classification_critical() {
        let critical_events = vec![
            RenderEvent::ShaderCompilationFailed {
                shader_path: "test.vert".to_string(),
                error_message: "Syntax error".to_string(),
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::DrawCallFailed {
                draw_call_id: 1,
                error: "Invalid pipeline".to_string(),
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::FenceWaitTimeout {
                fence_id: 1,
                timeout_ms: 1000,
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::GpuMemoryExhausted {
                requested_size: 1024,
                available_size: 512,
                frame: 1,
                timestamp: 0.016,
            },
        ];

        for event in critical_events {
            assert!(event.is_critical());
        }

        let non_critical = RenderEvent::TextureCreated {
            texture_id: 1,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 1,
            timestamp: 0.016,
        };
        assert!(!non_critical.is_critical());
    }

    #[test]
    fn test_event_classification_resource() {
        let resource_events = vec![
            RenderEvent::TextureCreated {
                texture_id: 1,
                width: 1024,
                height: 1024,
                format: "RGBA8".to_string(),
                memory_size: 4 * 1024 * 1024,
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::TextureDestroyed { texture_id: 1, frame: 2, timestamp: 0.032 },
            RenderEvent::BufferCreated {
                buffer_id: 2,
                size: 1024,
                usage: "VERTEX".to_string(),
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::BufferDestroyed { buffer_id: 2, frame: 2, timestamp: 0.032 },
        ];

        for event in resource_events {
            assert!(event.is_resource_event());
        }

        let non_resource = RenderEvent::DrawCallSubmitted {
            draw_call_id: 1,
            mesh_id: 10,
            material_id: 20,
            vertex_count: 1000,
            frame: 1,
            timestamp: 0.016,
        };
        assert!(!non_resource.is_resource_event());
    }

    #[test]
    fn test_event_classification_error() {
        let error_events = vec![
            RenderEvent::ShaderCompilationFailed {
                shader_path: "test.vert".to_string(),
                error_message: "Syntax error".to_string(),
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::DrawCallFailed {
                draw_call_id: 1,
                error: "Invalid pipeline".to_string(),
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::FenceWaitTimeout {
                fence_id: 1,
                timeout_ms: 1000,
                frame: 1,
                timestamp: 0.016,
            },
            RenderEvent::GpuMemoryExhausted {
                requested_size: 1024,
                available_size: 512,
                frame: 1,
                timestamp: 0.016,
            },
        ];

        for event in error_events {
            assert!(event.is_error_event());
        }

        let non_error = RenderEvent::PipelineCreated {
            pipeline_id: 1,
            vertex_shader: "test.vert".to_string(),
            fragment_shader: "test.frag".to_string(),
            frame: 1,
            timestamp: 0.016,
        };
        assert!(!non_error.is_error_event());
    }

    #[test]
    fn test_event_frame_and_timestamp() {
        let event = RenderEvent::TextureCreated {
            texture_id: 1,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 42,
            timestamp: 1.234,
        };

        assert_eq!(event.frame(), 42);
        assert_eq!(event.timestamp(), 1.234);
    }

    #[test]
    fn test_event_involved_resources() {
        let texture_event = RenderEvent::TextureCreated {
            texture_id: 123,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 1,
            timestamp: 0.016,
        };
        assert_eq!(texture_event.involved_resources(), vec![123]);

        let buffer_event = RenderEvent::BufferCreated {
            buffer_id: 456,
            size: 1024,
            usage: "VERTEX".to_string(),
            frame: 1,
            timestamp: 0.016,
        };
        assert_eq!(buffer_event.involved_resources(), vec![456]);

        let draw_call_event = RenderEvent::DrawCallSubmitted {
            draw_call_id: 1,
            mesh_id: 10,
            material_id: 20,
            vertex_count: 1000,
            frame: 1,
            timestamp: 0.016,
        };
        assert_eq!(draw_call_event.involved_resources(), vec![1, 10, 20]);

        let swapchain_event = RenderEvent::SwapchainRecreated {
            reason: "Window resized".to_string(),
            old_width: 800,
            old_height: 600,
            new_width: 1024,
            new_height: 768,
            frame: 1,
            timestamp: 0.016,
        };
        assert_eq!(swapchain_event.involved_resources(), Vec::<u64>::new());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let event = RenderEvent::TextureCreated {
            texture_id: 42,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 1,
            timestamp: 0.016,
        };

        // Serialize to JSON
        let json = serde_json::to_string(&event).unwrap();

        // Deserialize back
        let deserialized: RenderEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.frame(), event.frame());
        assert_eq!(deserialized.timestamp(), event.timestamp());
        assert_eq!(deserialized.involved_resources(), event.involved_resources());
    }

    #[test]
    fn test_clear_events() {
        let recorder = EventRecorder::new();
        recorder.enable();

        recorder.record(RenderEvent::TextureCreated {
            texture_id: 1,
            width: 1024,
            height: 1024,
            format: "RGBA8".to_string(),
            memory_size: 4 * 1024 * 1024,
            frame: 1,
            timestamp: 0.016,
        });

        assert_eq!(recorder.event_count(), 1);

        recorder.clear();
        assert_eq!(recorder.event_count(), 0);
        assert_eq!(recorder.total_event_count(), 1); // Total count not affected by clear
    }

    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let recorder = Arc::new(EventRecorder::new());
        recorder.enable();

        let mut handles = vec![];

        // Spawn multiple threads recording events
        for i in 0..4 {
            let recorder_clone = Arc::clone(&recorder);
            let handle = thread::spawn(move || {
                for j in 0..25 {
                    recorder_clone.record(RenderEvent::TextureCreated {
                        texture_id: (i * 25 + j) as u64,
                        width: 1024,
                        height: 1024,
                        format: "RGBA8".to_string(),
                        memory_size: 4 * 1024 * 1024,
                        frame: i as u64,
                        timestamp: (i * 25 + j) as f64 * 0.016,
                    });
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Should have 100 events (4 threads * 25 events each)
        assert_eq!(recorder.event_count(), 100);
        assert_eq!(recorder.total_event_count(), 100);
    }
}
