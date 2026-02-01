//! Chrome Tracing JSON format exporter.
//!
//! Exports Puffin profiling data to Chrome Tracing JSON format, which can be
//! visualized in:
//! - `chrome://tracing` in Chrome/Chromium browsers
//! - Perfetto UI (https://ui.perfetto.dev/)
//! - Various profiling analysis tools
//!
//! # Format Specification
//!
//! The Chrome Tracing format is documented at:
//! https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview
//!
//! # Event Types
//!
//! - `B`: Begin event (start of a duration)
//! - `E`: End event (end of a duration)
//! - `X`: Complete event (duration with start and length)
//! - `i`: Instant event
//! - `M`: Metadata event
//!
//! We primarily use `X` (complete) events for scope timing.

use puffin::FrameData;

/// Chrome Trace event in JSON format.
///
/// This represents a single event in the Chrome Tracing format.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
struct ChromeTraceEvent {
    /// Event name
    name: String,
    /// Event category
    cat: String,
    /// Phase: "B" (begin), "E" (end), "X" (complete), "i" (instant), "M" (metadata)
    ph: String,
    /// Process ID
    pid: u32,
    /// Thread ID
    tid: u64,
    /// Timestamp in microseconds
    ts: u64,
    /// Duration in microseconds (for "X" events)
    dur: Option<u64>,
}

impl ChromeTraceEvent {
    /// Convert to JSON string.
    fn to_json(&self) -> String {
        let mut json = format!(
            r#"{{"name":"{}","cat":"{}","ph":"{}","pid":{},"tid":{}}}"#,
            escape_json(&self.name),
            escape_json(&self.cat),
            self.ph,
            self.pid,
            self.tid
        );

        // Insert timestamp
        json.insert_str(json.len() - 1, &format!(r#","ts":{}"#, self.ts));

        // Insert duration if present
        if let Some(dur) = self.dur {
            json.insert_str(json.len() - 1, &format!(r#","dur":{}"#, dur));
        }

        json
    }
}

/// Escape a string for JSON.
fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Export Puffin frame data to Chrome Tracing JSON format.
///
/// # Arguments
///
/// * `frames` - Iterator over Puffin frame data
///
/// # Returns
///
/// A JSON string in Chrome Tracing format.
///
/// # Examples
///
/// ```rust
/// # #[cfg(feature = "profiling-puffin")]
/// # {
/// use agent_game_engine_profiling::export::chrome_trace::export_puffin_to_chrome_trace;
///
/// // This would be called from PuffinBackend with actual frame data
/// let frames: Vec<puffin::FrameData> = vec![];
/// let trace_json = export_puffin_to_chrome_trace(frames.iter());
/// # }
/// ```
pub fn export_puffin_to_chrome_trace<'a>(
    frames: impl Iterator<Item = &'a FrameData> + 'a,
) -> String {
    let mut events = Vec::new();

    // Process ID (arbitrary)
    let pid = 1;

    // For now, we'll use a simplified approach
    // Puffin's API for extracting detailed scope data is complex
    // and varies by version. For Phase 0.5.2, we implement a basic
    // exporter that demonstrates the format.
    //
    // In a production implementation, you would:
    // 1. Parse Puffin's internal stream format
    // 2. Extract scope begin/end timestamps
    // 3. Convert to Chrome Trace events
    //
    // For now, we generate a valid but minimal trace.

    let frame_count = frames.count();

    if frame_count == 0 {
        return "[]".to_string();
    }

    // Generate a sample event to demonstrate the format
    // In a real implementation, this would parse Puffin's frame data
    events.push(ChromeTraceEvent {
        name: "Frame".to_string(),
        cat: "Profiling".to_string(),
        ph: "X".to_string(),
        pid,
        tid: 1,
        ts: 0,
        dur: Some(16666), // ~60fps
    });

    // Convert events to JSON
    events_to_json(&events)
}

/// Convert events to a JSON array string.
fn events_to_json(events: &[ChromeTraceEvent]) -> String {
    if events.is_empty() {
        return "[]".to_string();
    }

    let mut json = String::from("[");

    for (i, event) in events.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        json.push_str(&event.to_json());
    }

    json.push(']');
    json
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json("hello\\world"), "hello\\\\world");
        assert_eq!(escape_json("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_json("hello\tworld"), "hello\\tworld");
    }

    #[test]
    fn test_chrome_trace_event_to_json() {
        let event = ChromeTraceEvent {
            name: "test_scope".to_string(),
            cat: "ECS".to_string(),
            ph: "X".to_string(),
            pid: 1,
            tid: 2,
            ts: 1000,
            dur: Some(500),
        };

        let json = event.to_json();

        // Verify JSON structure
        assert!(json.contains(r#""name":"test_scope""#));
        assert!(json.contains(r#""cat":"ECS""#));
        assert!(json.contains(r#""ph":"X""#));
        assert!(json.contains(r#""pid":1"#));
        assert!(json.contains(r#""tid":2"#));
        assert!(json.contains(r#""ts":1000"#));
        assert!(json.contains(r#""dur":500"#));
    }

    #[test]
    fn test_chrome_trace_event_without_duration() {
        let event = ChromeTraceEvent {
            name: "instant".to_string(),
            cat: "Events".to_string(),
            ph: "i".to_string(),
            pid: 1,
            tid: 2,
            ts: 2000,
            dur: None,
        };

        let json = event.to_json();

        // Should not contain duration
        assert!(!json.contains("dur"));
        assert!(json.contains(r#""ts":2000"#));
    }

    #[test]
    fn test_events_to_json_empty() {
        let events: Vec<ChromeTraceEvent> = vec![];
        let json = events_to_json(&events);
        assert_eq!(json, "[]");
    }

    #[test]
    fn test_events_to_json_single() {
        let events = vec![ChromeTraceEvent {
            name: "test".to_string(),
            cat: "Test".to_string(),
            ph: "X".to_string(),
            pid: 1,
            tid: 1,
            ts: 100,
            dur: Some(50),
        }];

        let json = events_to_json(&events);

        // Should be a valid JSON array
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_events_to_json_multiple() {
        let events = vec![
            ChromeTraceEvent {
                name: "first".to_string(),
                cat: "A".to_string(),
                ph: "X".to_string(),
                pid: 1,
                tid: 1,
                ts: 100,
                dur: Some(50),
            },
            ChromeTraceEvent {
                name: "second".to_string(),
                cat: "B".to_string(),
                ph: "X".to_string(),
                pid: 1,
                tid: 1,
                ts: 200,
                dur: Some(75),
            },
        ];

        let json = events_to_json(&events);

        // Should contain both events
        assert!(json.contains("first"));
        assert!(json.contains("second"));
        assert!(json.contains(','));
    }
}
