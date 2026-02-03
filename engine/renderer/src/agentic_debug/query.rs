//! Query API for AI agents to analyze rendering debug data
//!
//! Provides high-level queries for detecting performance issues,
//! resource leaks, and validation errors.

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use rusqlite::Connection;

define_error! {
    pub enum QueryError {
        DatabaseError { reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
        NoDataError { reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Warning,
    }
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub frame: u64,
    pub frame_time_ms: f64,
    pub draw_calls: u32,
    pub gpu_memory_used: u64,
}

/// Rendering query API
pub struct RenderQueryAPI {
    conn: Connection,
}

impl RenderQueryAPI {
    /// Open a database for querying
    pub fn open(path: &str) -> Result<Self, QueryError> {
        let conn = Connection::open(path)
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;
        Ok(Self { conn })
    }

    /// Find frames above a frame time threshold (milliseconds)
    pub fn find_frames_above_threshold(
        &self,
        threshold_ms: f64,
    ) -> Result<Vec<QueryResult>, QueryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, frame_time_ms, draw_calls, gpu_memory_used FROM frames \
                 WHERE frame_time_ms > ?1 ORDER BY frame_time_ms DESC",
            )
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        let results = stmt
            .query_map([threshold_ms], |row| {
                Ok(QueryResult {
                    frame: row.get(0)?,
                    frame_time_ms: row.get(1)?,
                    draw_calls: row.get(2)?,
                    gpu_memory_used: row.get(3)?,
                })
            })
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        Ok(results)
    }

    /// Find frames with high draw call counts
    pub fn find_frames_with_draw_calls_above(
        &self,
        threshold: u32,
    ) -> Result<Vec<QueryResult>, QueryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, frame_time_ms, draw_calls, gpu_memory_used FROM frames \
                 WHERE draw_calls > ?1 ORDER BY draw_calls DESC",
            )
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        let results = stmt
            .query_map([threshold], |row| {
                Ok(QueryResult {
                    frame: row.get(0)?,
                    frame_time_ms: row.get(1)?,
                    draw_calls: row.get(2)?,
                    gpu_memory_used: row.get(3)?,
                })
            })
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        Ok(results)
    }

    /// Detect resource leaks (buffers/images allocated but not freed)
    pub fn detect_resource_leaks(&self) -> Result<Vec<ResourceLeakReport>, QueryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, buffer_count, image_count, buffers_allocated, buffers_freed, \
                 images_allocated, images_freed FROM resources \
                 WHERE (buffers_allocated - buffers_freed > 0) OR (images_allocated - images_freed > 0) \
                 ORDER BY frame",
            )
            .map_err(|e| QueryError::DatabaseError {
                reason: e.to_string(),
            })?;

        let results = stmt
            .query_map([], |row| {
                Ok(ResourceLeakReport {
                    frame: row.get(0)?,
                    buffer_count: row.get(1)?,
                    image_count: row.get(2)?,
                    buffers_allocated: row.get(3)?,
                    buffers_freed: row.get(4)?,
                    images_allocated: row.get(5)?,
                    images_freed: row.get(6)?,
                })
            })
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        Ok(results)
    }

    /// Find frames with validation errors
    pub fn find_validation_errors(&self) -> Result<Vec<ValidationErrorReport>, QueryError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, frame_time_ms, validation_errors FROM frames \
                 WHERE validation_errors > 0 ORDER BY frame",
            )
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        let results = stmt
            .query_map([], |row| {
                Ok(ValidationErrorReport {
                    frame: row.get(0)?,
                    frame_time_ms: row.get(1)?,
                    error_count: row.get(2)?,
                })
            })
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        Ok(results)
    }

    /// Calculate average frame time
    pub fn average_frame_time(&self) -> Result<f64, QueryError> {
        let result: f64 = self
            .conn
            .query_row("SELECT AVG(frame_time_ms) FROM frames", [], |row| row.get(0))
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        Ok(result)
    }

    /// Calculate p95 frame time
    pub fn p95_frame_time(&self) -> Result<f64, QueryError> {
        let result: f64 = self
            .conn
            .query_row(
                "SELECT frame_time_ms FROM frames ORDER BY frame_time_ms \
                 LIMIT 1 OFFSET CAST((SELECT COUNT(*) * 0.95 FROM frames) AS INTEGER)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        Ok(result)
    }

    /// Get frame count
    pub fn frame_count(&self) -> Result<u64, QueryError> {
        let result: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0))
            .map_err(|e| QueryError::DatabaseError { reason: e.to_string() })?;

        Ok(result as u64)
    }
}

/// Resource leak report
#[derive(Debug, Clone)]
pub struct ResourceLeakReport {
    pub frame: u64,
    pub buffer_count: u32,
    pub image_count: u32,
    pub buffers_allocated: u32,
    pub buffers_freed: u32,
    pub images_allocated: u32,
    pub images_freed: u32,
}

impl ResourceLeakReport {
    /// Calculate buffer leak (allocated - freed)
    pub fn buffer_leak(&self) -> i32 {
        self.buffers_allocated as i32 - self.buffers_freed as i32
    }

    /// Calculate image leak (allocated - freed)
    pub fn image_leak(&self) -> i32 {
        self.images_allocated as i32 - self.images_freed as i32
    }
}

/// Validation error report
#[derive(Debug, Clone)]
pub struct ValidationErrorReport {
    pub frame: u64,
    pub frame_time_ms: f64,
    pub error_count: u32,
}
