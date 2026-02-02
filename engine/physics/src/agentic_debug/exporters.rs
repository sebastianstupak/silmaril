//! Export physics snapshots and events to various formats
//!
//! Supports:
//! - JSONL (JSON Lines) - Streaming, human-readable, line-by-line processing
//! - SQLite - Queryable time-series database for AI agent analysis
//! - CSV - Simple metrics for spreadsheet/pandas analysis

use crate::agentic_debug::{PhysicsDebugSnapshot, PhysicsEvent};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

define_error! {
    pub enum ExportError {
        IoError { path: String, reason: String } = ErrorCode::FileSystemError, ErrorSeverity::Error,
        SerializationError { reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
        DatabaseError { reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
    }
}

/// JSONL (JSON Lines) exporter
///
/// Exports one JSON object per line, suitable for streaming and line-by-line processing.
/// Format: Each line is a complete JSON object (snapshot or event).
///
/// Example output:
/// ```jsonl
/// {"frame":0,"timestamp":0.0,"entities":[...],"colliders":[...],...}
/// {"frame":1,"timestamp":0.016,"entities":[...],"colliders":[...],...}
/// ```
pub struct JsonlExporter {
    writer: BufWriter<File>,
    objects_written: usize,
}

impl JsonlExporter {
    /// Create a new JSONL exporter (overwrites existing file)
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, ExportError> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let file = File::create(path)
            .map_err(|e| ExportError::IoError { path: path_str.clone(), reason: e.to_string() })?;

        Ok(Self { writer: BufWriter::new(file), objects_written: 0 })
    }

    /// Open JSONL file in append mode
    pub fn append<P: AsRef<Path>>(path: P) -> Result<Self, ExportError> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let file =
            OpenOptions::new().create(true).append(true).open(path).map_err(|e| {
                ExportError::IoError { path: path_str.clone(), reason: e.to_string() }
            })?;

        Ok(Self { writer: BufWriter::new(file), objects_written: 0 })
    }

    /// Write a snapshot to the JSONL file
    pub fn write_snapshot(&mut self, snapshot: &PhysicsDebugSnapshot) -> Result<(), ExportError> {
        let json = serde_json::to_string(snapshot)
            .map_err(|e| ExportError::SerializationError { reason: e.to_string() })?;

        writeln!(self.writer, "{}", json).map_err(|e| ExportError::IoError {
            path: "JSONL file".to_string(),
            reason: e.to_string(),
        })?;

        self.objects_written += 1;
        Ok(())
    }

    /// Write an event to the JSONL file
    pub fn write_event(&mut self, event: &PhysicsEvent) -> Result<(), ExportError> {
        let json = serde_json::to_string(event)
            .map_err(|e| ExportError::SerializationError { reason: e.to_string() })?;

        writeln!(self.writer, "{}", json).map_err(|e| ExportError::IoError {
            path: "JSONL file".to_string(),
            reason: e.to_string(),
        })?;

        self.objects_written += 1;
        Ok(())
    }

    /// Write multiple events at once (more efficient)
    pub fn write_events(&mut self, events: &[PhysicsEvent]) -> Result<(), ExportError> {
        for event in events {
            self.write_event(event)?;
        }
        Ok(())
    }

    /// Get number of objects written
    pub fn objects_written(&self) -> usize {
        self.objects_written
    }

    /// Flush buffered data to disk
    pub fn flush(&mut self) -> Result<(), ExportError> {
        self.writer.flush().map_err(|e| ExportError::IoError {
            path: "JSONL file".to_string(),
            reason: e.to_string(),
        })
    }

    /// Finish writing and close file (flushes automatically)
    pub fn finish(mut self) -> Result<usize, ExportError> {
        self.flush()?;
        Ok(self.objects_written)
    }
}

/// SQLite time-series database exporter
///
/// Creates a queryable database with tables:
/// - snapshots: Frame metadata
/// - entity_states: Per-frame entity data
/// - events: Event log
/// - contact_manifolds: Collision details (future)
/// - islands: Solver partitioning (future)
pub struct SqliteExporter {
    conn: rusqlite::Connection,
    snapshots_inserted: usize,
    events_inserted: usize,
}

impl SqliteExporter {
    /// Create a new SQLite database (overwrites existing)
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, ExportError> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let conn = rusqlite::Connection::open(path).map_err(|e| ExportError::DatabaseError {
            reason: format!("Failed to open {}: {}", path_str, e),
        })?;

        let mut exporter = Self { conn, snapshots_inserted: 0, events_inserted: 0 };

        exporter.create_schema()?;
        Ok(exporter)
    }

    /// Create database schema
    fn create_schema(&mut self) -> Result<(), ExportError> {
        self.conn
            .execute_batch(
                r#"
                -- Frame-by-frame snapshots
                CREATE TABLE IF NOT EXISTS snapshots (
                    frame INTEGER PRIMARY KEY,
                    timestamp REAL NOT NULL,
                    entity_count INTEGER NOT NULL,
                    active_count INTEGER NOT NULL,
                    sleeping_count INTEGER NOT NULL,
                    total_kinetic_energy REAL NOT NULL,
                    state_hash INTEGER NOT NULL
                );

                -- Entity states per frame
                CREATE TABLE IF NOT EXISTS entity_states (
                    frame INTEGER NOT NULL,
                    entity_id INTEGER NOT NULL,
                    pos_x REAL NOT NULL,
                    pos_y REAL NOT NULL,
                    pos_z REAL NOT NULL,
                    rot_x REAL NOT NULL,
                    rot_y REAL NOT NULL,
                    rot_z REAL NOT NULL,
                    rot_w REAL NOT NULL,
                    vel_x REAL NOT NULL,
                    vel_y REAL NOT NULL,
                    vel_z REAL NOT NULL,
                    angvel_x REAL NOT NULL,
                    angvel_y REAL NOT NULL,
                    angvel_z REAL NOT NULL,
                    force_x REAL NOT NULL,
                    force_y REAL NOT NULL,
                    force_z REAL NOT NULL,
                    torque_x REAL NOT NULL,
                    torque_y REAL NOT NULL,
                    torque_z REAL NOT NULL,
                    mass REAL NOT NULL,
                    sleeping INTEGER NOT NULL,
                    is_static INTEGER NOT NULL,
                    ccd_enabled INTEGER NOT NULL,
                    PRIMARY KEY (frame, entity_id),
                    FOREIGN KEY (frame) REFERENCES snapshots(frame)
                );

                -- Events
                CREATE TABLE IF NOT EXISTS events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    frame INTEGER NOT NULL,
                    timestamp REAL NOT NULL,
                    event_type TEXT NOT NULL,
                    entity_a INTEGER,
                    entity_b INTEGER,
                    data TEXT NOT NULL,
                    FOREIGN KEY (frame) REFERENCES snapshots(frame)
                );

                -- Indices for common queries
                CREATE INDEX IF NOT EXISTS idx_entity_states_entity
                    ON entity_states(entity_id, frame);

                CREATE INDEX IF NOT EXISTS idx_entity_states_frame
                    ON entity_states(frame);

                CREATE INDEX IF NOT EXISTS idx_events_type
                    ON events(event_type, frame);

                CREATE INDEX IF NOT EXISTS idx_events_entity
                    ON events(entity_a, frame);

                CREATE INDEX IF NOT EXISTS idx_events_frame
                    ON events(frame);
                "#,
            )
            .map_err(|e| ExportError::DatabaseError {
                reason: format!("Failed to create schema: {}", e),
            })?;

        Ok(())
    }

    /// Write a snapshot to the database
    pub fn write_snapshot(&mut self, snapshot: &PhysicsDebugSnapshot) -> Result<(), ExportError> {
        // Start transaction for performance
        let tx = self.conn.transaction().map_err(|e| ExportError::DatabaseError {
            reason: format!("Failed to start transaction: {}", e),
        })?;

        // Insert snapshot metadata
        tx.execute(
            "INSERT INTO snapshots (frame, timestamp, entity_count, active_count, sleeping_count, total_kinetic_energy, state_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                snapshot.frame as i64,
                snapshot.timestamp,
                snapshot.entity_count() as i64,
                snapshot.active_entity_count() as i64,
                snapshot.sleeping_entity_count() as i64,
                snapshot.total_kinetic_energy(),
                snapshot.compute_hash() as i64,
            ],
        )
        .map_err(|e| ExportError::DatabaseError {
            reason: format!("Failed to insert snapshot: {}", e),
        })?;

        // Insert entity states (batch)
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO entity_states VALUES
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
                )
                .map_err(|e| ExportError::DatabaseError {
                    reason: format!("Failed to prepare entity insert: {}", e),
                })?;

            for entity in &snapshot.entities {
                stmt.execute(rusqlite::params![
                    snapshot.frame as i64,
                    entity.id as i64,
                    entity.position.x,
                    entity.position.y,
                    entity.position.z,
                    entity.rotation.x,
                    entity.rotation.y,
                    entity.rotation.z,
                    entity.rotation.w,
                    entity.linear_velocity.x,
                    entity.linear_velocity.y,
                    entity.linear_velocity.z,
                    entity.angular_velocity.x,
                    entity.angular_velocity.y,
                    entity.angular_velocity.z,
                    entity.forces.x,
                    entity.forces.y,
                    entity.forces.z,
                    entity.torques.x,
                    entity.torques.y,
                    entity.torques.z,
                    entity.mass,
                    entity.sleeping as i32,
                    entity.is_static as i32,
                    entity.ccd_enabled as i32,
                ])
                .map_err(|e| ExportError::DatabaseError {
                    reason: format!("Failed to insert entity {}: {}", entity.id, e),
                })?;
            }
        }

        // Commit transaction
        tx.commit().map_err(|e| ExportError::DatabaseError {
            reason: format!("Failed to commit transaction: {}", e),
        })?;

        self.snapshots_inserted += 1;
        Ok(())
    }

    /// Write an event to the database
    pub fn write_event(&mut self, event: &PhysicsEvent) -> Result<(), ExportError> {
        let entities = event.involved_entities();
        let entity_a = entities.get(0).copied();
        let entity_b = entities.get(1).copied();

        let data_json = serde_json::to_string(event)
            .map_err(|e| ExportError::SerializationError { reason: e.to_string() })?;

        self.conn
            .execute(
                "INSERT INTO events (frame, timestamp, event_type, entity_a, entity_b, data)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    event.frame() as i64,
                    event.timestamp(),
                    event.event_type(),
                    entity_a.map(|e| e as i64),
                    entity_b.map(|e| e as i64),
                    data_json,
                ],
            )
            .map_err(|e| ExportError::DatabaseError {
                reason: format!("Failed to insert event: {}", e),
            })?;

        self.events_inserted += 1;
        Ok(())
    }

    /// Write multiple events at once (more efficient)
    pub fn write_events(&mut self, events: &[PhysicsEvent]) -> Result<(), ExportError> {
        let tx = self.conn.transaction().map_err(|e| ExportError::DatabaseError {
            reason: format!("Failed to start transaction: {}", e),
        })?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO events (frame, timestamp, event_type, entity_a, entity_b, data)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| ExportError::DatabaseError {
                    reason: format!("Failed to prepare event insert: {}", e),
                })?;

            for event in events {
                let entities = event.involved_entities();
                let entity_a = entities.get(0).copied();
                let entity_b = entities.get(1).copied();

                let data_json = serde_json::to_string(event)
                    .map_err(|e| ExportError::SerializationError { reason: e.to_string() })?;

                stmt.execute(rusqlite::params![
                    event.frame() as i64,
                    event.timestamp(),
                    event.event_type(),
                    entity_a.map(|e| e as i64),
                    entity_b.map(|e| e as i64),
                    data_json,
                ])
                .map_err(|e| ExportError::DatabaseError {
                    reason: format!("Failed to insert event: {}", e),
                })?;

                self.events_inserted += 1;
            }
        }

        tx.commit().map_err(|e| ExportError::DatabaseError {
            reason: format!("Failed to commit events: {}", e),
        })?;

        Ok(())
    }

    /// Get statistics
    pub fn statistics(&self) -> (usize, usize) {
        (self.snapshots_inserted, self.events_inserted)
    }

    /// Optimize database (vacuum, analyze)
    pub fn optimize(&mut self) -> Result<(), ExportError> {
        self.conn
            .execute_batch("VACUUM; ANALYZE;")
            .map_err(|e| ExportError::DatabaseError {
                reason: format!("Failed to optimize: {}", e),
            })?;
        Ok(())
    }
}

/// CSV metrics exporter
///
/// Exports simple time-series data to CSV format for spreadsheet/pandas analysis.
/// Format: frame,entity_id,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,...
pub struct CsvExporter {
    writer: csv::Writer<File>,
    rows_written: usize,
}

impl CsvExporter {
    /// Create a new CSV exporter
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, ExportError> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let file = File::create(path)
            .map_err(|e| ExportError::IoError { path: path_str.clone(), reason: e.to_string() })?;

        let mut writer = csv::Writer::from_writer(file);

        // Write header
        writer
            .write_record(&[
                "frame",
                "entity_id",
                "pos_x",
                "pos_y",
                "pos_z",
                "vel_x",
                "vel_y",
                "vel_z",
                "angvel_x",
                "angvel_y",
                "angvel_z",
                "mass",
                "sleeping",
            ])
            .map_err(|e| ExportError::IoError { path: path_str, reason: e.to_string() })?;

        Ok(Self { writer, rows_written: 0 })
    }

    /// Write entity states from snapshot
    pub fn write_snapshot(&mut self, snapshot: &PhysicsDebugSnapshot) -> Result<(), ExportError> {
        for entity in &snapshot.entities {
            self.writer
                .write_record(&[
                    snapshot.frame.to_string(),
                    entity.id.to_string(),
                    entity.position.x.to_string(),
                    entity.position.y.to_string(),
                    entity.position.z.to_string(),
                    entity.linear_velocity.x.to_string(),
                    entity.linear_velocity.y.to_string(),
                    entity.linear_velocity.z.to_string(),
                    entity.angular_velocity.x.to_string(),
                    entity.angular_velocity.y.to_string(),
                    entity.angular_velocity.z.to_string(),
                    entity.mass.to_string(),
                    (entity.sleeping as i32).to_string(),
                ])
                .map_err(|e| ExportError::IoError {
                    path: "CSV file".to_string(),
                    reason: e.to_string(),
                })?;

            self.rows_written += 1;
        }

        Ok(())
    }

    /// Get number of rows written
    pub fn rows_written(&self) -> usize {
        self.rows_written
    }

    /// Flush buffered data
    pub fn flush(&mut self) -> Result<(), ExportError> {
        self.writer.flush().map_err(|e| ExportError::IoError {
            path: "CSV file".to_string(),
            reason: e.to_string(),
        })
    }

    /// Finish writing and close file
    pub fn finish(mut self) -> Result<usize, ExportError> {
        self.flush()?;
        Ok(self.rows_written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_debug::{EntityState, PhysicsDebugSnapshot};
    use engine_math::{Quat, Vec3};
    use tempfile::NamedTempFile;

    fn create_test_snapshot(frame: u64) -> PhysicsDebugSnapshot {
        let mut snapshot = PhysicsDebugSnapshot::new(frame, frame as f64 * 0.016);

        snapshot.entities.push(EntityState {
            id: 1,
            position: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            linear_velocity: Vec3::new(5.0, 0.0, 0.0),
            angular_velocity: Vec3::ZERO,
            forces: Vec3::ZERO,
            torques: Vec3::ZERO,
            mass: 1.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            gravity_scale: 1.0,
            sleeping: false,
            is_static: false,
            is_kinematic: false,
            can_sleep: true,
            ccd_enabled: false,
        });

        snapshot
    }

    #[test]
    fn test_jsonl_export() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut exporter = JsonlExporter::create(path).unwrap();

        let snapshot1 = create_test_snapshot(0);
        let snapshot2 = create_test_snapshot(1);

        exporter.write_snapshot(&snapshot1).unwrap();
        exporter.write_snapshot(&snapshot2).unwrap();

        exporter.flush().unwrap();

        assert_eq!(exporter.objects_written(), 2);

        // Read back and verify
        let content = std::fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        // Each line should be valid JSON
        let parsed1: PhysicsDebugSnapshot = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed1.frame, 0);
    }

    #[test]
    fn test_jsonl_append() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Write initial data
        {
            let mut exporter = JsonlExporter::create(path).unwrap();
            exporter.write_snapshot(&create_test_snapshot(0)).unwrap();
            exporter.finish().unwrap();
        }

        // Append more data
        {
            let mut exporter = JsonlExporter::append(path).unwrap();
            exporter.write_snapshot(&create_test_snapshot(1)).unwrap();
            exporter.finish().unwrap();
        }

        // Verify both entries
        let content = std::fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_sqlite_export() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut exporter = SqliteExporter::create(path).unwrap();

        let snapshot1 = create_test_snapshot(0);
        let snapshot2 = create_test_snapshot(1);

        exporter.write_snapshot(&snapshot1).unwrap();
        exporter.write_snapshot(&snapshot2).unwrap();

        let (snapshots, _events) = exporter.statistics();
        assert_eq!(snapshots, 2);

        // Verify data was written
        let conn = rusqlite::Connection::open(path).unwrap();
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0)).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_csv_export() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        let mut exporter = CsvExporter::create(path).unwrap();

        let snapshot = create_test_snapshot(0);
        exporter.write_snapshot(&snapshot).unwrap();
        exporter.flush().unwrap();

        assert_eq!(exporter.rows_written(), 1);

        // Verify CSV format
        let content = std::fs::read_to_string(path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2); // Header + 1 data row
    }
}
