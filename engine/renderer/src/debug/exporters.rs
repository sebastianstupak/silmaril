//! Export Infrastructure (R.3)
//!
//! Machine-readable export formats for rendering debug data:
//! - JSONL (JSON Lines): Streaming export for events and snapshots
//! - SQLite: Queryable database for historical analysis
//! - PNG: Frame capture and visual comparison
//!
//! # Example: JSONL Export
//!
//! ```no_run
//! use engine_renderer::debug::{RenderDebugSnapshot, JsonlExporter};
//! use std::path::Path;
//!
//! let mut exporter = JsonlExporter::create(Path::new("debug.jsonl"))?;
//!
//! // Export snapshots as they're created
//! let snapshot = RenderDebugSnapshot::new(1, 0.016);
//! exporter.write_snapshot(&snapshot)?;
//!
//! exporter.flush()?;
//! let count = exporter.finish()?;
//! println!("Exported {} objects", count);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Example: SQLite Export
//!
//! ```no_run
//! use engine_renderer::debug::{RenderDebugSnapshot, SqliteExporter};
//! use std::path::Path;
//!
//! let mut exporter = SqliteExporter::create(Path::new("debug.db"))?;
//!
//! // Export snapshots to queryable database
//! let snapshot = RenderDebugSnapshot::new(1, 0.016);
//! exporter.write_snapshot(&snapshot)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

#![allow(missing_docs)] // Debug infrastructure - comprehensive docs not required

use crate::debug::RenderDebugSnapshot;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

define_error! {
    pub enum ExportError {
        Io { details: String } = ErrorCode::DebugExportIo, ErrorSeverity::Error,
        Serialization { details: String } = ErrorCode::DebugExportSerialization, ErrorSeverity::Error,
        Database { details: String } = ErrorCode::DebugExportDatabase, ErrorSeverity::Error,
        PngEncoding { details: String } = ErrorCode::DebugExportPngEncoding, ErrorSeverity::Error,
    }
}

// Implement From conversions for convenience
impl From<std::io::Error> for ExportError {
    fn from(err: std::io::Error) -> Self {
        ExportError::io(err.to_string())
    }
}

impl From<serde_json::Error> for ExportError {
    fn from(err: serde_json::Error) -> Self {
        ExportError::serialization(err.to_string())
    }
}

impl From<rusqlite::Error> for ExportError {
    fn from(err: rusqlite::Error) -> Self {
        ExportError::database(err.to_string())
    }
}

/// JSONL (JSON Lines) exporter for streaming event export
///
/// Each line is a complete JSON object. Supports both snapshots and events.
/// Uses buffered writes for performance.
pub struct JsonlExporter {
    writer: BufWriter<File>,
    objects_written: usize,
}

impl JsonlExporter {
    /// Create new JSONL file (overwrites if exists)
    pub fn create(path: &Path) -> Result<Self, ExportError> {
        let file = File::create(path)?;
        Ok(Self { writer: BufWriter::new(file), objects_written: 0 })
    }

    /// Open existing JSONL file for appending
    pub fn append(path: &Path) -> Result<Self, ExportError> {
        let file = OpenOptions::new().append(true).create(true).open(path)?;
        Ok(Self { writer: BufWriter::new(file), objects_written: 0 })
    }

    /// Write snapshot as JSONL entry
    pub fn write_snapshot(&mut self, snapshot: &RenderDebugSnapshot) -> Result<(), ExportError> {
        let json = serde_json::to_string(snapshot)?;
        writeln!(self.writer, "{}", json)?;
        self.objects_written += 1;
        Ok(())
    }

    /// Write event as JSONL entry (generic JSON serialization)
    pub fn write_event<T: Serialize>(&mut self, event: &T) -> Result<(), ExportError> {
        let json = serde_json::to_string(event)?;
        writeln!(self.writer, "{}", json)?;
        self.objects_written += 1;
        Ok(())
    }

    /// Flush buffered data to disk
    pub fn flush(&mut self) -> Result<(), ExportError> {
        self.writer.flush()?;
        Ok(())
    }

    /// Finish export and return number of objects written
    pub fn finish(mut self) -> Result<usize, ExportError> {
        self.writer.flush()?;
        Ok(self.objects_written)
    }

    /// Get number of objects written so far
    pub fn objects_written(&self) -> usize {
        self.objects_written
    }
}

/// SQLite exporter for queryable database
///
/// Creates a relational database with tables for snapshots, draw calls,
/// textures, and events. Optimized for common queries with indices.
pub struct SqliteExporter {
    conn: Connection,
}

impl SqliteExporter {
    /// Create new SQLite database (overwrites if exists)
    pub fn create(path: &Path) -> Result<Self, ExportError> {
        let conn = Connection::open(path)?;
        let exporter = Self { conn };
        exporter.init_schema()?;
        Ok(exporter)
    }

    /// Initialize database schema with tables and indices
    fn init_schema(&self) -> Result<(), ExportError> {
        // Snapshots table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                frame INTEGER PRIMARY KEY,
                timestamp REAL NOT NULL,
                active_pipeline TEXT,
                gpu_memory_total INTEGER,
                draw_call_count INTEGER
            )",
            [],
        )?;

        // Draw calls table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS draw_calls (
                draw_call_id INTEGER PRIMARY KEY,
                frame INTEGER NOT NULL,
                mesh_id INTEGER,
                material_id INTEGER,
                pipeline_id INTEGER,
                vertex_count INTEGER,
                index_count INTEGER,
                instance_count INTEGER,
                draw_time_gpu_ns INTEGER,
                vertices_processed INTEGER,
                fragments_processed INTEGER,
                FOREIGN KEY (frame) REFERENCES snapshots(frame)
            )",
            [],
        )?;

        // Textures table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS textures (
                texture_id INTEGER PRIMARY KEY,
                frame_created INTEGER NOT NULL,
                frame_destroyed INTEGER,
                width INTEGER,
                height INTEGER,
                format TEXT,
                mip_levels INTEGER,
                memory_size INTEGER,
                FOREIGN KEY (frame_created) REFERENCES snapshots(frame)
            )",
            [],
        )?;

        // Events table (generic events stored as JSON blobs)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                frame INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL,
                FOREIGN KEY (frame) REFERENCES snapshots(frame)
            )",
            [],
        )?;

        // Indices for common queries
        self.conn
            .execute("CREATE INDEX IF NOT EXISTS idx_draw_calls_frame ON draw_calls(frame)", [])?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_draw_calls_time ON draw_calls(draw_time_gpu_ns)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_textures_created ON textures(frame_created)",
            [],
        )?;
        self.conn
            .execute("CREATE INDEX IF NOT EXISTS idx_events_frame ON events(frame)", [])?;
        self.conn
            .execute("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)", [])?;

        Ok(())
    }

    /// Export snapshot and all related data (transaction-based)
    pub fn write_snapshot(&mut self, snapshot: &RenderDebugSnapshot) -> Result<(), ExportError> {
        let tx = self.conn.transaction()?;

        // Insert snapshot metadata
        tx.execute(
            "INSERT OR REPLACE INTO snapshots (frame, timestamp, active_pipeline, gpu_memory_total, draw_call_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                snapshot.frame,
                snapshot.timestamp,
                snapshot.active_pipeline,
                snapshot.gpu_memory.total_allocated,
                snapshot.draw_calls.len(),
            ],
        )?;

        // Insert draw calls
        for draw_call in &snapshot.draw_calls {
            tx.execute(
                "INSERT OR REPLACE INTO draw_calls
                 (draw_call_id, frame, mesh_id, material_id, pipeline_id, vertex_count,
                  index_count, instance_count, draw_time_gpu_ns, vertices_processed, fragments_processed)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    draw_call.draw_call_id,
                    snapshot.frame,
                    draw_call.mesh_id,
                    draw_call.material_id,
                    draw_call.pipeline_id,
                    draw_call.vertex_count,
                    draw_call.index_count,
                    draw_call.instance_count,
                    draw_call.draw_time_gpu_ns,
                    draw_call.vertices_processed,
                    draw_call.fragments_processed,
                ],
            )?;
        }

        // Insert textures
        for texture in &snapshot.textures {
            tx.execute(
                "INSERT OR REPLACE INTO textures
                 (texture_id, frame_created, width, height, format, mip_levels, memory_size)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    texture.texture_id,
                    texture.created_frame,
                    texture.width,
                    texture.height,
                    texture.format,
                    texture.mip_levels,
                    texture.memory_size,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Export event (generic JSON serialization)
    pub fn write_event<T: Serialize>(
        &mut self,
        frame: u64,
        event_type: &str,
        event: &T,
    ) -> Result<(), ExportError> {
        let event_data = serde_json::to_string(event)?;
        self.conn.execute(
            "INSERT INTO events (frame, event_type, event_data) VALUES (?1, ?2, ?3)",
            params![frame, event_type, event_data],
        )?;
        Ok(())
    }

    /// Get database connection for custom queries
    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}

/// PNG frame capture exporter
///
/// Exports frame buffers as PNG images for visual inspection and comparison.
pub struct PngExporter;

impl PngExporter {
    /// Export frame buffer as PNG image
    ///
    /// # Arguments
    ///
    /// * `path` - Output PNG file path
    /// * `color_data` - RGBA8 color data (row-major, top-to-bottom)
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    pub fn export_frame(
        path: &Path,
        color_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), ExportError> {
        // Validate data size
        let expected_size = (width * height * 4) as usize;
        if color_data.len() != expected_size {
            return Err(ExportError::pngencoding(format!(
                "Invalid data size: expected {} bytes for {}x{} RGBA8, got {} bytes",
                expected_size,
                width,
                height,
                color_data.len()
            )));
        }

        // Create PNG encoder
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        let mut encoder = png::Encoder::new(writer, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder
            .write_header()
            .map_err(|e: png::EncodingError| ExportError::pngencoding(e.to_string()))?;

        writer
            .write_image_data(color_data)
            .map_err(|e: png::EncodingError| ExportError::pngencoding(e.to_string()))?;

        Ok(())
    }

    /// Export side-by-side comparison of expected, actual, and diff images
    ///
    /// Creates a single PNG with three images horizontally arranged.
    /// Useful for visual regression testing.
    pub fn export_comparison(
        path: &Path,
        expected: &[u8],
        actual: &[u8],
        diff: &[u8],
        width: u32,
        height: u32,
    ) -> Result<(), ExportError> {
        // Validate input sizes
        let expected_size = (width * height * 4) as usize;
        if expected.len() != expected_size
            || actual.len() != expected_size
            || diff.len() != expected_size
        {
            return Err(ExportError::pngencoding(format!(
                "Invalid comparison data size: expected {} bytes per image",
                expected_size
            )));
        }

        // Create combined image (3 images side-by-side)
        let combined_width = width * 3;
        let mut combined_data = vec![0u8; (combined_width * height * 4) as usize];

        // Copy images side-by-side
        for y in 0..height {
            let src_row_start = (y * width * 4) as usize;
            let src_row_end = src_row_start + (width * 4) as usize;

            let dst_row_start = (y * combined_width * 4) as usize;

            // Expected (left)
            let expected_dst_start = dst_row_start;
            let expected_dst_end = expected_dst_start + (width * 4) as usize;
            combined_data[expected_dst_start..expected_dst_end]
                .copy_from_slice(&expected[src_row_start..src_row_end]);

            // Actual (middle)
            let actual_dst_start = expected_dst_start + (width * 4) as usize;
            let actual_dst_end = actual_dst_start + (width * 4) as usize;
            combined_data[actual_dst_start..actual_dst_end]
                .copy_from_slice(&actual[src_row_start..src_row_end]);

            // Diff (right)
            let diff_dst_start = actual_dst_start + (width * 4) as usize;
            let diff_dst_end = diff_dst_start + (width * 4) as usize;
            combined_data[diff_dst_start..diff_dst_end]
                .copy_from_slice(&diff[src_row_start..src_row_end]);
        }

        // Export combined image
        Self::export_frame(path, &combined_data, combined_width, height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::debug::{DrawCallInfo, RenderDebugSnapshot, TextureInfo};
    use tempfile::TempDir;

    #[test]
    fn test_jsonl_exporter_create_and_write() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.jsonl");

        let mut exporter = JsonlExporter::create(&path).unwrap();

        // Create test snapshot
        let snapshot = RenderDebugSnapshot::new(1, 0.016);
        exporter.write_snapshot(&snapshot).unwrap();

        let count = exporter.finish().unwrap();
        assert_eq!(count, 1);

        // Verify file was created and has content
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"frame\":1"));
        assert!(content.contains("\"timestamp\":0.016"));
    }

    #[test]
    fn test_jsonl_exporter_append_mode() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_append.jsonl");

        // Write first snapshot
        {
            let mut exporter = JsonlExporter::create(&path).unwrap();
            let snapshot = RenderDebugSnapshot::new(1, 0.016);
            exporter.write_snapshot(&snapshot).unwrap();
            exporter.finish().unwrap();
        }

        // Append second snapshot
        {
            let mut exporter = JsonlExporter::append(&path).unwrap();
            let snapshot = RenderDebugSnapshot::new(2, 0.032);
            exporter.write_snapshot(&snapshot).unwrap();
            exporter.finish().unwrap();
        }

        // Verify both snapshots are present
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"frame\":1"));
        assert!(lines[1].contains("\"frame\":2"));
    }

    #[test]
    fn test_jsonl_export_import_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("roundtrip.jsonl");

        // Create snapshot with data
        let mut snapshot = RenderDebugSnapshot::new(42, 1.234);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 100,
            fragments_processed: 1000,
        });

        // Export
        {
            let mut exporter = JsonlExporter::create(&path).unwrap();
            exporter.write_snapshot(&snapshot).unwrap();
            exporter.finish().unwrap();
        }

        // Import and verify
        let content = std::fs::read_to_string(&path).unwrap();
        let imported: RenderDebugSnapshot =
            serde_json::from_str(content.lines().next().unwrap()).unwrap();

        assert_eq!(imported.frame, 42);
        assert_eq!(imported.timestamp, 1.234);
        assert_eq!(imported.draw_calls.len(), 1);
        assert_eq!(imported.draw_calls[0].mesh_id, 1);
        assert_eq!(imported.draw_calls[0].vertex_count, 100);
    }

    #[test]
    fn test_sqlite_exporter_create_database() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.db");

        let exporter = SqliteExporter::create(&path).unwrap();

        // Verify database file exists
        assert!(path.exists());

        // Verify tables exist
        let conn = exporter.connection();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |row: &rusqlite::Row| row.get(0))
            .unwrap()
            .map(|r: Result<String, _>| r.unwrap())
            .collect();

        assert!(tables.contains(&"snapshots".to_string()));
        assert!(tables.contains(&"draw_calls".to_string()));
        assert!(tables.contains(&"textures".to_string()));
        assert!(tables.contains(&"events".to_string()));
    }

    #[test]
    fn test_sqlite_exporter_write_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_snapshot.db");

        let mut exporter = SqliteExporter::create(&path).unwrap();

        // Create test snapshot
        let mut snapshot = RenderDebugSnapshot::new(1, 0.016);
        snapshot.draw_calls.push(DrawCallInfo {
            draw_call_id: 0,
            mesh_id: 1,
            material_id: 2,
            pipeline_id: 3,
            vertex_count: 100,
            index_count: 150,
            instance_count: 1,
            transform: [1.0; 16],
            draw_time_gpu_ns: 50000,
            vertices_processed: 100,
            fragments_processed: 1000,
        });
        snapshot.textures.push(TextureInfo {
            texture_id: 10,
            width: 1024,
            height: 1024,
            depth: 1,
            format: "RGBA8".to_string(),
            mip_levels: 1,
            sample_count: 1,
            memory_size: 4194304,
            created_frame: 1,
        });

        exporter.write_snapshot(&snapshot).unwrap();

        // Query and verify data
        let conn = exporter.connection();

        // Check snapshot
        let frame: u64 = conn
            .query_row("SELECT frame FROM snapshots WHERE frame = 1", [], |row: &rusqlite::Row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(frame, 1);

        // Check draw call
        let mesh_id: u64 = conn
            .query_row(
                "SELECT mesh_id FROM draw_calls WHERE draw_call_id = 0",
                [],
                |row: &rusqlite::Row| row.get(0),
            )
            .unwrap();
        assert_eq!(mesh_id, 1);

        // Check texture
        let texture_width: u32 = conn
            .query_row(
                "SELECT width FROM textures WHERE texture_id = 10",
                [],
                |row: &rusqlite::Row| row.get(0),
            )
            .unwrap();
        assert_eq!(texture_width, 1024);
    }

    #[test]
    fn test_sqlite_exporter_query_indices() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_indices.db");

        let exporter = SqliteExporter::create(&path).unwrap();

        // Verify indices exist
        let conn = exporter.connection();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' ORDER BY name")
            .unwrap();
        let indices: Vec<String> = stmt
            .query_map([], |row: &rusqlite::Row| row.get(0))
            .unwrap()
            .map(|r: Result<String, _>| r.unwrap())
            .collect();

        assert!(indices.contains(&"idx_draw_calls_frame".to_string()));
        assert!(indices.contains(&"idx_draw_calls_time".to_string()));
        assert!(indices.contains(&"idx_textures_created".to_string()));
        assert!(indices.contains(&"idx_events_frame".to_string()));
        assert!(indices.contains(&"idx_events_type".to_string()));
    }

    #[test]
    fn test_png_exporter_basic() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.png");

        // Create simple 2x2 RGBA8 image (red, green, blue, white)
        let color_data = vec![
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            255, 255, 255, 255, // White
        ];

        PngExporter::export_frame(&path, &color_data, 2, 2).unwrap();

        // Verify file was created
        assert!(path.exists());

        // Verify file size is reasonable (PNG header + data)
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 50); // PNG files have overhead
    }

    #[test]
    fn test_png_exporter_invalid_size() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("invalid.png");

        // Wrong size data
        let color_data = vec![255, 0, 0, 255]; // Only 1 pixel for 2x2 image

        let result = PngExporter::export_frame(&path, &color_data, 2, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_png_exporter_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("comparison.png");

        // Create three 2x2 images (4 pixels each, RGBA8)
        let expected = vec![
            255, 0, 0, 255, // Red
            255, 0, 0, 255, // Red
            255, 0, 0, 255, // Red
            255, 0, 0, 255, // Red
        ];
        let actual = vec![
            0, 255, 0, 255, // Green
            0, 255, 0, 255, // Green
            0, 255, 0, 255, // Green
            0, 255, 0, 255, // Green
        ];
        let diff = vec![
            255, 255, 255, 255, // White
            255, 255, 255, 255, // White
            255, 255, 255, 255, // White
            255, 255, 255, 255, // White
        ];

        PngExporter::export_comparison(&path, &expected, &actual, &diff, 2, 2).unwrap();

        // Verify file was created
        assert!(path.exists());

        // Verify file size (should be ~3x larger than single image)
        let metadata = std::fs::metadata(&path).unwrap();
        assert!(metadata.len() > 100);
    }
}
