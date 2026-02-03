//! Rendering Query API (R.4)
//!
//! High-level query API for AI agents to analyze rendering debug data.
//!
//! # Example
//! ```no_run
//! use engine_renderer::debug::RenderingQueryAPI;
//!
//! let api = RenderingQueryAPI::open("debug.db")?;
//!
//! // Find leaked resources
//! let leaks = api.find_leaked_resources()?;
//! for leak in leaks {
//!     println!("Leaked {}: {} bytes", leak.resource_type, leak.memory_size);
//! }
//!
//! // Find slow draw calls
//! let slow = api.slow_draw_calls(1.0, 0, 1000)?;
//! println!("Found {} slow draw calls", slow.len());
//! # Ok::<(), engine_renderer::debug::QueryError>(())
//! ```

#![allow(missing_docs)]

use crate::debug::events::RenderEvent;
use crate::debug::snapshot::{DrawCallInfo, GpuMemoryStats};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use rusqlite::{params, Connection, Row};
use std::collections::HashMap;
use std::path::Path;

define_error! {
    pub enum QueryError {
        Database { details: String } = ErrorCode::DebugExportDatabase, ErrorSeverity::Error,
        InvalidQuery { query: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
        ResourceNotFound { resource_id: u64 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
        NoData {} = ErrorCode::InvalidFormat, ErrorSeverity::Warning,
        Deserialization { details: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
    }
}

impl From<rusqlite::Error> for QueryError {
    fn from(err: rusqlite::Error) -> Self {
        QueryError::database(err.to_string())
    }
}

/// High-level query API for AI agents
pub struct RenderingQueryAPI {
    conn: Connection,
}

impl RenderingQueryAPI {
    /// Open existing debug database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, QueryError> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    /// Query texture lifecycle (creation to destruction)
    pub fn texture_lifecycle(&self, texture_id: u64) -> Result<TextureLifecycle, QueryError> {
        let mut stmt = self.conn.prepare(
            "SELECT texture_id, frame_created, frame_destroyed, width, height, format, memory_size
             FROM textures WHERE texture_id = ?1",
        )?;

        let lifecycle = stmt
            .query_row([texture_id], |row: &Row| {
                Ok(TextureLifecycle {
                    texture_id: row.get(0)?,
                    created_frame: row.get(1)?,
                    destroyed_frame: row.get(2)?,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    format: row.get(5)?,
                    memory_size: row.get(6)?,
                    usage_count: 0, // TODO: Count draw calls using this texture
                })
            })
            .map_err(|_| QueryError::resourcenotfound(texture_id))?;

        Ok(lifecycle)
    }

    /// Find resources that were created but never destroyed (memory leaks)
    pub fn find_leaked_resources(&self) -> Result<Vec<LeakedResource>, QueryError> {
        let mut stmt = self.conn.prepare(
            "SELECT texture_id, frame_created, memory_size
             FROM textures WHERE frame_destroyed IS NULL",
        )?;

        let mut leaks = Vec::new();
        let rows = stmt.query_map([], |row: &Row| {
            Ok(LeakedResource {
                resource_type: "texture".to_string(),
                resource_id: row.get(0)?,
                created_frame: row.get(1)?,
                memory_size: row.get(2)?,
                last_used_frame: row.get(1)?, // Use created frame as fallback
            })
        })?;

        for row in rows {
            leaks.push(row?);
        }

        Ok(leaks)
    }

    /// Query buffer lifecycle (stub - would need buffers table)
    pub fn buffer_lifecycle(&self, buffer_id: u64) -> Result<BufferLifecycle, QueryError> {
        Err(QueryError::resourcenotfound(buffer_id))
    }

    /// Find draw calls slower than threshold
    pub fn slow_draw_calls(
        &self,
        threshold_ms: f32,
        start_frame: u64,
        end_frame: u64,
    ) -> Result<Vec<DrawCallInfo>, QueryError> {
        let threshold_ns = (threshold_ms * 1_000_000.0) as i64;

        let mut stmt = self.conn.prepare(
            "SELECT draw_call_id, mesh_id, material_id, pipeline_id,
                    vertex_count, index_count, instance_count, draw_time_gpu_ns
             FROM draw_calls
             WHERE frame >= ?1 AND frame <= ?2 AND draw_time_gpu_ns > ?3
             ORDER BY draw_time_gpu_ns DESC",
        )?;

        let mut slow_calls = Vec::new();
        let rows = stmt.query_map(params![start_frame, end_frame, threshold_ns], |row: &Row| {
            Ok(DrawCallInfo {
                draw_call_id: row.get(0)?,
                mesh_id: row.get(1)?,
                material_id: row.get(2)?,
                pipeline_id: row.get(3)?,
                vertex_count: row.get(4)?,
                index_count: row.get(5)?,
                instance_count: row.get(6)?,
                transform: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ], // Identity
                draw_time_gpu_ns: row.get(7)?,
                vertices_processed: 0,
                fragments_processed: 0,
            })
        })?;

        for row in rows {
            slow_calls.push(row?);
        }

        Ok(slow_calls)
    }

    /// Get frame times (frame number, time in ms)
    pub fn frame_times(
        &self,
        start_frame: u64,
        end_frame: u64,
    ) -> Result<Vec<(u64, f32)>, QueryError> {
        let mut stmt = self.conn.prepare(
            "SELECT frame, timestamp FROM snapshots
             WHERE frame >= ?1 AND frame <= ?2
             ORDER BY frame",
        )?;

        let mut frame_times = Vec::new();
        let mut prev_timestamp: Option<f64> = None;

        let rows = stmt.query_map(params![start_frame, end_frame], |row: &Row| {
            Ok((row.get::<_, u64>(0)?, row.get::<_, f64>(1)?))
        })?;

        for row in rows {
            let (frame, timestamp) = row?;
            if let Some(prev) = prev_timestamp {
                let frame_time_ms = ((timestamp - prev) * 1000.0) as f32;
                frame_times.push((frame, frame_time_ms));
            }
            prev_timestamp = Some(timestamp);
        }

        Ok(frame_times)
    }

    /// Get GPU memory usage over time
    pub fn gpu_memory_over_time(
        &self,
        start_frame: u64,
        end_frame: u64,
    ) -> Result<Vec<(u64, GpuMemoryStats)>, QueryError> {
        let mut stmt = self.conn.prepare(
            "SELECT frame, gpu_memory_total FROM snapshots
             WHERE frame >= ?1 AND frame <= ?2
             ORDER BY frame",
        )?;

        let mut memory_timeline = Vec::new();
        let rows = stmt.query_map(params![start_frame, end_frame], |row: &Row| {
            let frame: u64 = row.get(0)?;
            let total_bytes: u64 = row.get(1)?;

            Ok((
                frame,
                GpuMemoryStats {
                    total_allocated: total_bytes as usize,
                    textures: total_bytes as usize,
                    buffers: 0,
                    framebuffers: 0,
                    device_local: total_bytes as usize,
                    host_visible: 0,
                },
            ))
        })?;

        for row in rows {
            memory_timeline.push(row?);
        }

        Ok(memory_timeline)
    }

    /// Get shader compilation errors
    pub fn shader_compilation_errors(&self) -> Result<Vec<ShaderError>, QueryError> {
        let mut stmt = self.conn.prepare(
            "SELECT frame, event_data FROM events
             WHERE event_type = 'ShaderCompilationFailed'
             ORDER BY frame",
        )?;

        let mut errors = Vec::new();
        let rows = stmt.query_map([], |row: &Row| {
            let frame: u64 = row.get(0)?;
            let json_data: String = row.get(1)?;
            Ok((frame, json_data))
        })?;

        #[allow(clippy::collapsible_match)]
        for row in rows {
            let (frame, json_data) = row?;
            if let Ok(event) = serde_json::from_str::<RenderEvent>(&json_data) {
                if let RenderEvent::ShaderCompilationFailed { shader_path, error_message, .. } =
                    event
                {
                    errors.push(ShaderError { frame, shader_path, error_message });
                }
            }
        }

        Ok(errors)
    }

    /// Get swapchain recreation events
    pub fn swapchain_recreations(&self) -> Result<Vec<SwapchainEvent>, QueryError> {
        let mut stmt = self.conn.prepare(
            "SELECT frame, event_data FROM events
             WHERE event_type = 'SwapchainRecreated'
             ORDER BY frame",
        )?;

        let mut events = Vec::new();
        let rows = stmt.query_map([], |row: &Row| {
            let frame: u64 = row.get(0)?;
            let json_data: String = row.get(1)?;
            Ok((frame, json_data))
        })?;

        #[allow(clippy::collapsible_match)]
        for row in rows {
            let (frame, json_data) = row?;
            if let Ok(event) = serde_json::from_str::<RenderEvent>(&json_data) {
                if let RenderEvent::SwapchainRecreated {
                    reason,
                    old_width,
                    old_height,
                    new_width,
                    new_height,
                    ..
                } = event
                {
                    events.push(SwapchainEvent {
                        frame,
                        reason,
                        old_width,
                        old_height,
                        new_width,
                        new_height,
                    });
                }
            }
        }

        Ok(events)
    }

    /// Get draw call failure events
    pub fn draw_call_failures(&self) -> Result<Vec<DrawCallError>, QueryError> {
        let mut stmt = self.conn.prepare(
            "SELECT frame, event_data FROM events
             WHERE event_type = 'DrawCallFailed'
             ORDER BY frame",
        )?;

        let mut failures = Vec::new();
        let rows = stmt.query_map([], |row: &Row| {
            let frame: u64 = row.get(0)?;
            let json_data: String = row.get(1)?;
            Ok((frame, json_data))
        })?;

        #[allow(clippy::collapsible_match)]
        for row in rows {
            let (frame, json_data) = row?;
            if let Ok(event) = serde_json::from_str::<RenderEvent>(&json_data) {
                if let RenderEvent::DrawCallFailed { draw_call_id, error, .. } = event {
                    failures.push(DrawCallError { frame, draw_call_id, error_message: error });
                }
            }
        }

        Ok(failures)
    }

    /// Execute raw SQL query
    pub fn raw_query(&self, sql: &str) -> Result<Vec<HashMap<String, Value>>, QueryError> {
        let mut stmt = self.conn.prepare(sql)?;
        let column_count = stmt.column_count();
        let column_names: Vec<String> =
            stmt.column_names().iter().map(|&s| s.to_string()).collect();

        let mut results = Vec::new();
        let rows = stmt.query_map([], |row: &Row| {
            let mut map = HashMap::new();
            #[allow(clippy::needless_range_loop)]
            for i in 0..column_count {
                let value = match row.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(v) => Value::Integer(v),
                    rusqlite::types::ValueRef::Real(v) => Value::Real(v),
                    rusqlite::types::ValueRef::Text(v) => {
                        Value::Text(String::from_utf8_lossy(v).to_string())
                    }
                    rusqlite::types::ValueRef::Blob(_) => Value::Null,
                };
                map.insert(column_names[i].clone(), value);
            }
            Ok(map)
        })?;

        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Get database statistics
    pub fn statistics(&self) -> Result<DatabaseStats, QueryError> {
        let total_frames: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM snapshots", [], |row: &Row| row.get(0))?;

        let total_draw_calls: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM draw_calls", [], |row: &Row| row.get(0))?;

        let total_textures: u64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM textures", [], |row: &Row| row.get(0))?;

        let total_events: u64 =
            self.conn.query_row("SELECT COUNT(*) FROM events", [], |row: &Row| row.get(0))?;

        Ok(DatabaseStats {
            total_frames,
            total_draw_calls,
            total_textures,
            total_buffers: 0, // No buffers table yet
            total_events,
            database_size_bytes: 0, // Would need filesystem query
        })
    }

    /// Compare render outputs between two frames (stub)
    pub fn compare_render_outputs(
        &self,
        frame_a: u64,
        frame_b: u64,
    ) -> Result<ImageDiff, QueryError> {
        // This would require frame capture data
        Ok(ImageDiff {
            frame_a,
            frame_b,
            pixels_different: 0,
            percent_different: 0.0,
            max_color_delta: 0.0,
            avg_color_delta: 0.0,
        })
    }
}

// ============================================================================
// Result Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct TextureLifecycle {
    pub texture_id: u64,
    pub created_frame: u64,
    pub destroyed_frame: Option<u64>,
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub memory_size: usize,
    pub usage_count: usize,
}

#[derive(Debug, Clone)]
pub struct BufferLifecycle {
    pub buffer_id: u64,
    pub created_frame: u64,
    pub destroyed_frame: Option<u64>,
    pub size_bytes: usize,
    pub usage: String,
    pub memory_type: String,
}

#[derive(Debug, Clone)]
pub struct LeakedResource {
    pub resource_type: String,
    pub resource_id: u64,
    pub created_frame: u64,
    pub memory_size: usize,
    pub last_used_frame: u64,
}

#[derive(Debug, Clone)]
pub struct ShaderError {
    pub frame: u64,
    pub shader_path: String,
    pub error_message: String,
}

#[derive(Debug, Clone)]
pub struct SwapchainEvent {
    pub frame: u64,
    pub reason: String,
    pub old_width: u32,
    pub old_height: u32,
    pub new_width: u32,
    pub new_height: u32,
}

#[derive(Debug, Clone)]
pub struct DrawCallError {
    pub frame: u64,
    pub draw_call_id: u64,
    pub error_message: String,
}

#[derive(Debug, Clone)]
pub struct ImageDiff {
    pub frame_a: u64,
    pub frame_b: u64,
    pub pixels_different: usize,
    pub percent_different: f32,
    pub max_color_delta: f32,
    pub avg_color_delta: f32,
}

#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_frames: u64,
    pub total_draw_calls: u64,
    pub total_textures: u64,
    pub total_buffers: u64,
    pub total_events: u64,
    pub database_size_bytes: u64,
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::RenderDebugSnapshot;
    use crate::debug::SqliteExporter;
    use tempfile::TempDir;

    fn create_test_database() -> (TempDir, RenderingQueryAPI) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create database and populate with test data
        let mut exporter = SqliteExporter::create(&db_path).unwrap();

        // Add snapshots
        let snapshot = RenderDebugSnapshot::new(1, 0.016);
        exporter.write_snapshot(&snapshot).unwrap();

        let snapshot = RenderDebugSnapshot::new(2, 0.032);
        exporter.write_snapshot(&snapshot).unwrap();

        // Open for querying
        let api = RenderingQueryAPI::open(&db_path).unwrap();

        (temp_dir, api)
    }

    #[test]
    fn test_query_api_open() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create empty database
        SqliteExporter::create(&db_path).unwrap();

        // Open for querying
        let api = RenderingQueryAPI::open(&db_path);
        assert!(api.is_ok());
    }

    #[test]
    fn test_statistics() {
        let (_dir, api) = create_test_database();

        let stats = api.statistics().unwrap();
        assert_eq!(stats.total_frames, 2);
    }

    #[test]
    fn test_frame_times() {
        let (_dir, api) = create_test_database();

        let frame_times = api.frame_times(1, 2).unwrap();
        assert_eq!(frame_times.len(), 1); // 2 frames = 1 delta
        assert!(frame_times[0].1 > 0.0); // Frame time should be positive
    }

    #[test]
    fn test_find_leaked_resources_empty() {
        let (_dir, api) = create_test_database();

        let leaks = api.find_leaked_resources().unwrap();
        assert_eq!(leaks.len(), 0); // No textures created yet
    }

    #[test]
    fn test_slow_draw_calls_empty() {
        let (_dir, api) = create_test_database();

        let slow_calls = api.slow_draw_calls(1.0, 0, 100).unwrap();
        assert_eq!(slow_calls.len(), 0); // No draw calls yet
    }

    #[test]
    fn test_gpu_memory_over_time() {
        let (_dir, api) = create_test_database();

        let memory = api.gpu_memory_over_time(1, 2).unwrap();
        assert_eq!(memory.len(), 2); // 2 frames
    }

    #[test]
    fn test_shader_compilation_errors_empty() {
        let (_dir, api) = create_test_database();

        let errors = api.shader_compilation_errors().unwrap();
        assert_eq!(errors.len(), 0); // No errors recorded
    }

    #[test]
    fn test_swapchain_recreations_empty() {
        let (_dir, api) = create_test_database();

        let events = api.swapchain_recreations().unwrap();
        assert_eq!(events.len(), 0); // No events recorded
    }

    #[test]
    fn test_draw_call_failures_empty() {
        let (_dir, api) = create_test_database();

        let failures = api.draw_call_failures().unwrap();
        assert_eq!(failures.len(), 0); // No failures recorded
    }

    #[test]
    fn test_raw_query() {
        let (_dir, api) = create_test_database();

        let results = api.raw_query("SELECT COUNT(*) as count FROM snapshots").unwrap();
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].get("count"), Some(Value::Integer(2))));
    }

    #[test]
    fn test_texture_lifecycle_not_found() {
        let (_dir, api) = create_test_database();

        let result = api.texture_lifecycle(999);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueryError::ResourceNotFound { .. }));
    }

    #[test]
    fn test_compare_render_outputs_stub() {
        let (_dir, api) = create_test_database();

        let diff = api.compare_render_outputs(1, 2).unwrap();
        assert_eq!(diff.frame_a, 1);
        assert_eq!(diff.frame_b, 2);
    }
}
