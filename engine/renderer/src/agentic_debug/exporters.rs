//! Export rendering debug data to various formats
//!
//! Supports JSONL (streaming), SQLite (queryable), and CSV (simple metrics).

use super::snapshot::RenderingDebugSnapshot;
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

define_error! {
    pub enum ExportError {
        IoError { path: String, reason: String } = ErrorCode::FileSystemError, ErrorSeverity::Error,
        SerializationError { reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
        DatabaseError { reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
    }
}

/// JSONL (JSON Lines) exporter - one JSON object per line
pub struct JsonlExporter {
    path: std::path::PathBuf,
}

impl JsonlExporter {
    /// Create a new JSONL exporter
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf() }
    }

    /// Export a snapshot (appends to file)
    pub fn export(&self, snapshot: &RenderingDebugSnapshot) -> Result<(), ExportError> {
        let mut file =
            OpenOptions::new().create(true).append(true).open(&self.path).map_err(|e| {
                ExportError::IoError {
                    path: self.path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

        let json = serde_json::to_string(snapshot)
            .map_err(|e| ExportError::SerializationError { reason: e.to_string() })?;

        writeln!(file, "{}", json).map_err(|e| ExportError::IoError {
            path: self.path.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }
}

/// SQLite exporter - queryable database
pub struct SqliteExporter {
    conn: rusqlite::Connection,
}

impl SqliteExporter {
    /// Create a new SQLite exporter
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ExportError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| ExportError::DatabaseError { reason: e.to_string() })?;

        // Create tables
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS frames (
                frame INTEGER PRIMARY KEY,
                timestamp_ms INTEGER NOT NULL,
                frame_time_ms REAL NOT NULL,
                gpu_time_ms REAL,
                cpu_time_ms REAL NOT NULL,
                draw_calls INTEGER NOT NULL,
                pipeline_binds INTEGER NOT NULL,
                barriers INTEGER NOT NULL,
                gpu_memory_used INTEGER NOT NULL,
                validation_errors INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS render_passes (
                id TEXT PRIMARY KEY,
                frame INTEGER NOT NULL,
                format TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                draw_calls INTEGER NOT NULL,
                duration_us REAL,
                FOREIGN KEY(frame) REFERENCES frames(frame)
            );

            CREATE TABLE IF NOT EXISTS resources (
                frame INTEGER PRIMARY KEY,
                buffer_count INTEGER NOT NULL,
                image_count INTEGER NOT NULL,
                pipeline_count INTEGER NOT NULL,
                buffers_allocated INTEGER NOT NULL,
                buffers_freed INTEGER NOT NULL,
                images_allocated INTEGER NOT NULL,
                images_freed INTEGER NOT NULL,
                FOREIGN KEY(frame) REFERENCES frames(frame)
            );

            CREATE INDEX IF NOT EXISTS idx_frames_time ON frames(frame_time_ms);
            CREATE INDEX IF NOT EXISTS idx_render_passes_frame ON render_passes(frame);
            ",
        )
        .map_err(|e| ExportError::DatabaseError { reason: e.to_string() })?;

        Ok(Self { conn })
    }

    /// Export a snapshot
    pub fn export(&mut self, snapshot: &RenderingDebugSnapshot) -> Result<(), ExportError> {
        let stats = snapshot.performance_stats();
        let validation_errors =
            snapshot.validation_messages.iter().filter(|m| m.severity == "error").count();

        // Insert frame data
        self.conn
            .execute(
                "INSERT OR REPLACE INTO frames
             (frame, timestamp_ms, frame_time_ms, gpu_time_ms, cpu_time_ms, draw_calls,
              pipeline_binds, barriers, gpu_memory_used, validation_errors)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    snapshot.frame,
                    snapshot.timestamp_ms,
                    snapshot.frame_time_ms,
                    snapshot.gpu_time_ms,
                    snapshot.cpu_time_ms,
                    stats.draw_calls,
                    stats.pipeline_binds,
                    stats.barriers,
                    snapshot.resources.gpu_memory_used,
                    validation_errors,
                ],
            )
            .map_err(|e| ExportError::DatabaseError { reason: e.to_string() })?;

        // Insert render pass data
        for rp in &snapshot.render_passes {
            self.conn
                .execute(
                    "INSERT OR REPLACE INTO render_passes
                 (id, frame, format, width, height, draw_calls, duration_us)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    rusqlite::params![
                        rp.id,
                        snapshot.frame,
                        rp.format,
                        rp.width,
                        rp.height,
                        rp.draw_calls,
                        rp.duration_us,
                    ],
                )
                .map_err(|e| ExportError::DatabaseError { reason: e.to_string() })?;
        }

        // Insert resource data
        self.conn
            .execute(
                "INSERT OR REPLACE INTO resources
             (frame, buffer_count, image_count, pipeline_count, buffers_allocated,
              buffers_freed, images_allocated, images_freed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    snapshot.frame,
                    snapshot.resources.buffer_count,
                    snapshot.resources.image_count,
                    snapshot.resources.pipeline_count,
                    snapshot.resources.buffers_allocated,
                    snapshot.resources.buffers_freed,
                    snapshot.resources.images_allocated,
                    snapshot.resources.images_freed,
                ],
            )
            .map_err(|e| ExportError::DatabaseError { reason: e.to_string() })?;

        Ok(())
    }
}

/// CSV exporter - simple metrics
pub struct CsvExporter {
    path: std::path::PathBuf,
    header_written: bool,
}

impl CsvExporter {
    /// Create a new CSV exporter
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self { path: path.as_ref().to_path_buf(), header_written: false }
    }

    /// Export a snapshot
    pub fn export(&mut self, snapshot: &RenderingDebugSnapshot) -> Result<(), ExportError> {
        let mut file =
            OpenOptions::new().create(true).append(true).open(&self.path).map_err(|e| {
                ExportError::IoError {
                    path: self.path.display().to_string(),
                    reason: e.to_string(),
                }
            })?;

        // Write header if this is the first export
        if !self.header_written {
            writeln!(
                file,
                "frame,timestamp_ms,frame_time_ms,gpu_time_ms,cpu_time_ms,draw_calls,\
                 pipeline_binds,barriers,gpu_memory_used,validation_errors"
            )
            .map_err(|e| ExportError::IoError {
                path: self.path.display().to_string(),
                reason: e.to_string(),
            })?;
            self.header_written = true;
        }

        let stats = snapshot.performance_stats();
        let validation_errors =
            snapshot.validation_messages.iter().filter(|m| m.severity == "error").count();

        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{}",
            snapshot.frame,
            snapshot.timestamp_ms,
            snapshot.frame_time_ms,
            snapshot.gpu_time_ms.unwrap_or(0.0),
            snapshot.cpu_time_ms,
            stats.draw_calls,
            stats.pipeline_binds,
            stats.barriers,
            snapshot.resources.gpu_memory_used,
            validation_errors,
        )
        .map_err(|e| ExportError::IoError {
            path: self.path.display().to_string(),
            reason: e.to_string(),
        })?;

        Ok(())
    }
}
