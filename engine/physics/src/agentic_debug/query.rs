//! Query API for AI agents to analyze exported physics data
//!
//! Provides high-level queries for common debugging scenarios:
//! - Entity state history over time
//! - Collision detection (find all collisions for entity)
//! - High-velocity detection (find frames where entity exceeded threshold)
//! - Constraint breaks
//! - Solver convergence failures
//! - Determinism violations
//!
//! Backed by SQLite database for efficient querying.

use crate::agentic_debug::{EntityState, PhysicsEvent};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use engine_math::{Quat, Vec3};
use rusqlite::{Connection, Row};
use std::collections::HashMap;
use std::path::Path;

define_error! {
    pub enum QueryError {
        DatabaseError { reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
        QueryFailed { query: String, reason: String } = ErrorCode::SerializationFailed, ErrorSeverity::Error,
        NoData { query: String } = ErrorCode::ComponentNotFound, ErrorSeverity::Warning,
    }
}

/// Query result type
pub type QueryResult<T> = Result<T, QueryError>;

/// Physics query API for AI agent debugging
///
/// Opens a SQLite database exported by physics simulation and provides
/// high-level queries for analyzing physics behavior.
pub struct PhysicsQueryAPI {
    conn: Connection,
}

impl PhysicsQueryAPI {
    /// Open an existing physics database
    pub fn open<P: AsRef<Path>>(path: P) -> QueryResult<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();

        let conn = Connection::open(path).map_err(|e| QueryError::DatabaseError {
            reason: format!("Failed to open {}: {}", path_str, e),
        })?;

        // Enable optimizations
        conn.execute_batch(
            "PRAGMA synchronous = OFF;
             PRAGMA journal_mode = WAL;
             PRAGMA cache_size = -64000;", // 64 MB cache
        )
        .map_err(|e| QueryError::DatabaseError {
            reason: format!("Failed to set pragmas: {}", e),
        })?;

        Ok(Self { conn })
    }

    /// Query: Get entity state history between frames
    ///
    /// Returns all recorded states for an entity in the given frame range.
    pub fn entity_history(
        &self,
        entity_id: u64,
        start_frame: u64,
        end_frame: u64,
    ) -> QueryResult<Vec<EntityState>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT * FROM entity_states
             WHERE entity_id = ?1 AND frame >= ?2 AND frame <= ?3
             ORDER BY frame ASC",
            )
            .map_err(|e| QueryError::DatabaseError {
                reason: format!("Failed to prepare entity_history query: {}", e),
            })?;

        let rows = stmt
            .query_map(
                rusqlite::params![entity_id as i64, start_frame as i64, end_frame as i64],
                row_to_entity_state,
            )
            .map_err(|e| QueryError::QueryFailed {
                query: "entity_history".to_string(),
                reason: e.to_string(),
            })?;

        let mut states = Vec::new();
        for row_result in rows {
            states.push(row_result.map_err(|e| QueryError::QueryFailed {
                query: "entity_history".to_string(),
                reason: e.to_string(),
            })?);
        }

        if states.is_empty() {
            return Err(QueryError::NoData {
                query: format!(
                    "entity_history(entity={}, frames={}-{})",
                    entity_id, start_frame, end_frame
                ),
            });
        }

        Ok(states)
    }

    /// Query: Find all collision events involving an entity
    pub fn entity_collisions(&self, entity_id: u64) -> QueryResult<Vec<CollisionEventData>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, timestamp, entity_a, entity_b, data
             FROM events
             WHERE (event_type = 'CollisionStart' OR event_type = 'CollisionEnd')
               AND (entity_a = ?1 OR entity_b = ?1)
             ORDER BY frame ASC",
            )
            .map_err(|e| QueryError::DatabaseError {
                reason: format!("Failed to prepare entity_collisions query: {}", e),
            })?;

        let rows = stmt
            .query_map(rusqlite::params![entity_id as i64], |row| {
                let frame: i64 = row.get(0)?;
                let timestamp: f64 = row.get(1)?;
                let entity_a: i64 = row.get(2)?;
                let entity_b: i64 = row.get(3)?;
                let data_json: String = row.get(4)?;

                Ok(CollisionEventData {
                    frame: frame as u64,
                    timestamp,
                    entity_a: entity_a as u64,
                    entity_b: entity_b as u64,
                    data_json,
                })
            })
            .map_err(|e| QueryError::QueryFailed {
                query: "entity_collisions".to_string(),
                reason: e.to_string(),
            })?;

        let mut events = Vec::new();
        for row_result in rows {
            events.push(row_result.map_err(|e| QueryError::QueryFailed {
                query: "entity_collisions".to_string(),
                reason: e.to_string(),
            })?);
        }

        Ok(events)
    }

    /// Query: Find frames where entity velocity exceeded threshold
    pub fn find_high_velocity(
        &self,
        entity_id: u64,
        threshold: f32,
    ) -> QueryResult<Vec<HighVelocityFrame>> {
        let threshold_sq = threshold * threshold;

        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, vel_x, vel_y, vel_z
             FROM entity_states
             WHERE entity_id = ?1
               AND (vel_x*vel_x + vel_y*vel_y + vel_z*vel_z) > ?2
             ORDER BY frame ASC",
            )
            .map_err(|e| QueryError::DatabaseError {
                reason: format!("Failed to prepare find_high_velocity query: {}", e),
            })?;

        let rows = stmt
            .query_map(rusqlite::params![entity_id as i64, threshold_sq], |row| {
                let frame: i64 = row.get(0)?;
                let vel_x: f32 = row.get(1)?;
                let vel_y: f32 = row.get(2)?;
                let vel_z: f32 = row.get(3)?;

                let velocity = Vec3::new(vel_x, vel_y, vel_z);

                Ok(HighVelocityFrame {
                    frame: frame as u64,
                    velocity,
                    magnitude: velocity.length(),
                })
            })
            .map_err(|e| QueryError::QueryFailed {
                query: "find_high_velocity".to_string(),
                reason: e.to_string(),
            })?;

        let mut frames = Vec::new();
        for row_result in rows {
            frames.push(row_result.map_err(|e| QueryError::QueryFailed {
                query: "find_high_velocity".to_string(),
                reason: e.to_string(),
            })?);
        }

        Ok(frames)
    }

    /// Query: Find all constraint break events
    pub fn constraint_breaks(&self) -> QueryResult<Vec<ConstraintBreakData>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, timestamp, entity_a, entity_b, data
             FROM events
             WHERE event_type = 'ConstraintBreak'
             ORDER BY frame ASC",
            )
            .map_err(|e| QueryError::DatabaseError {
                reason: format!("Failed to prepare constraint_breaks query: {}", e),
            })?;

        let rows = stmt
            .query_map([], |row| {
                let frame: i64 = row.get(0)?;
                let timestamp: f64 = row.get(1)?;
                let entity_a: Option<i64> = row.get(2)?;
                let entity_b: Option<i64> = row.get(3)?;
                let data_json: String = row.get(4)?;

                Ok(ConstraintBreakData {
                    frame: frame as u64,
                    timestamp,
                    entity_a: entity_a.map(|e| e as u64),
                    entity_b: entity_b.map(|e| e as u64),
                    data_json,
                })
            })
            .map_err(|e| QueryError::QueryFailed {
                query: "constraint_breaks".to_string(),
                reason: e.to_string(),
            })?;

        let mut breaks = Vec::new();
        for row_result in rows {
            breaks.push(row_result.map_err(|e| QueryError::QueryFailed {
                query: "constraint_breaks".to_string(),
                reason: e.to_string(),
            })?);
        }

        Ok(breaks)
    }

    /// Query: Find all solver convergence failures
    pub fn solver_failures(&self) -> QueryResult<Vec<SolverFailureData>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT frame, timestamp, data
             FROM events
             WHERE event_type = 'SolverFailure'
             ORDER BY frame ASC",
            )
            .map_err(|e| QueryError::DatabaseError {
                reason: format!("Failed to prepare solver_failures query: {}", e),
            })?;

        let rows = stmt
            .query_map([], |row| {
                let frame: i64 = row.get(0)?;
                let timestamp: f64 = row.get(1)?;
                let data_json: String = row.get(2)?;

                Ok(SolverFailureData { frame: frame as u64, timestamp, data_json })
            })
            .map_err(|e| QueryError::QueryFailed {
                query: "solver_failures".to_string(),
                reason: e.to_string(),
            })?;

        let mut failures = Vec::new();
        for row_result in rows {
            failures.push(row_result.map_err(|e| QueryError::QueryFailed {
                query: "solver_failures".to_string(),
                reason: e.to_string(),
            })?);
        }

        Ok(failures)
    }

    /// Query: Find determinism violations (hash mismatches)
    ///
    /// Compares recorded state hashes against reference hashes.
    /// Returns frames where hashes don't match.
    pub fn determinism_violations(
        &self,
        reference_hashes: &[(u64, u64)], // (frame, hash) pairs
    ) -> QueryResult<Vec<u64>> {
        let mut violations = Vec::new();

        for (frame, expected_hash) in reference_hashes {
            let actual_hash: Option<i64> = self
                .conn
                .query_row(
                    "SELECT state_hash FROM snapshots WHERE frame = ?1",
                    rusqlite::params![*frame as i64],
                    |row| row.get(0),
                )
                .ok();

            if let Some(actual) = actual_hash {
                if actual != *expected_hash as i64 {
                    violations.push(*frame);
                }
            }
        }

        Ok(violations)
    }

    /// Query: Get all events of a specific type in frame range
    pub fn events_by_type(
        &self,
        event_type: &str,
        start_frame: u64,
        end_frame: u64,
    ) -> QueryResult<Vec<PhysicsEvent>> {
        let query = if event_type == "*" {
            "SELECT data FROM events WHERE frame >= ?1 AND frame <= ?2 ORDER BY frame ASC"
                .to_string()
        } else {
            format!(
                "SELECT data FROM events WHERE event_type = '{}' AND frame >= ?1 AND frame <= ?2 ORDER BY frame ASC",
                event_type
            )
        };

        let mut stmt = self.conn.prepare(&query).map_err(|e| QueryError::DatabaseError {
            reason: format!("Failed to prepare events_by_type query: {}", e),
        })?;

        let rows = stmt
            .query_map(rusqlite::params![start_frame as i64, end_frame as i64], |row| {
                let data_json: String = row.get(0)?;
                Ok(data_json)
            })
            .map_err(|e| QueryError::QueryFailed {
                query: "events_by_type".to_string(),
                reason: e.to_string(),
            })?;

        let mut events = Vec::new();
        for row_result in rows {
            let json = row_result.map_err(|e| QueryError::QueryFailed {
                query: "events_by_type".to_string(),
                reason: e.to_string(),
            })?;

            let event: PhysicsEvent =
                serde_json::from_str(&json).map_err(|e| QueryError::QueryFailed {
                    query: "events_by_type".to_string(),
                    reason: format!("Failed to parse event JSON: {}", e),
                })?;

            events.push(event);
        }

        Ok(events)
    }

    /// Query: Custom SQL for advanced analysis
    ///
    /// **Use with caution** - allows arbitrary SQL queries.
    /// Returns rows as HashMap<String, rusqlite::types::Value>.
    pub fn raw_query(
        &self,
        sql: &str,
    ) -> QueryResult<Vec<HashMap<String, rusqlite::types::Value>>> {
        let mut stmt = self.conn.prepare(sql).map_err(|e| QueryError::DatabaseError {
            reason: format!("Failed to prepare raw query: {}", e),
        })?;

        let column_count = stmt.column_count();
        let column_names: Vec<String> =
            (0..column_count).map(|i| stmt.column_name(i).unwrap().to_string()).collect();

        let rows = stmt
            .query_map([], |row| {
                let mut map = HashMap::new();
                for (i, name) in column_names.iter().enumerate() {
                    let value: rusqlite::types::Value = row.get(i)?;
                    map.insert(name.clone(), value);
                }
                Ok(map)
            })
            .map_err(|e| QueryError::QueryFailed {
                query: "raw_query".to_string(),
                reason: e.to_string(),
            })?;

        let mut results = Vec::new();
        for row_result in rows {
            results.push(row_result.map_err(|e| QueryError::QueryFailed {
                query: "raw_query".to_string(),
                reason: e.to_string(),
            })?);
        }

        Ok(results)
    }

    /// Get database statistics
    pub fn statistics(&self) -> QueryResult<DatabaseStatistics> {
        let total_frames: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
            .unwrap_or(0);

        let total_entities: i64 = self
            .conn
            .query_row("SELECT COUNT(DISTINCT entity_id) FROM entity_states", [], |row| row.get(0))
            .unwrap_or(0);

        let total_events: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))
            .unwrap_or(0);

        Ok(DatabaseStatistics {
            total_frames: total_frames as usize,
            total_entities: total_entities as usize,
            total_events: total_events as usize,
        })
    }
}

// Helper function to convert SQL row to EntityState
fn row_to_entity_state(row: &Row) -> rusqlite::Result<EntityState> {
    Ok(EntityState {
        id: row.get::<_, i64>(1)? as u64,
        position: Vec3::new(row.get(2)?, row.get(3)?, row.get(4)?),
        rotation: Quat::from_xyzw(row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?),
        linear_velocity: Vec3::new(row.get(9)?, row.get(10)?, row.get(11)?),
        angular_velocity: Vec3::new(row.get(12)?, row.get(13)?, row.get(14)?),
        forces: Vec3::new(row.get(15)?, row.get(16)?, row.get(17)?),
        torques: Vec3::new(row.get(18)?, row.get(19)?, row.get(20)?),
        mass: row.get(21)?,
        sleeping: row.get::<_, i32>(22)? != 0,
        is_static: row.get::<_, i32>(23)? != 0,
        is_kinematic: false, // Not stored in DB currently
        can_sleep: true,     // Not stored in DB currently
        ccd_enabled: row.get::<_, i32>(24)? != 0,
        linear_damping: 0.0,  // Not stored in DB currently
        angular_damping: 0.0, // Not stored in DB currently
        gravity_scale: 1.0,   // Not stored in DB currently
    })
}

/// Collision event data from database
#[derive(Debug, Clone)]
pub struct CollisionEventData {
    pub frame: u64,
    pub timestamp: f64,
    pub entity_a: u64,
    pub entity_b: u64,
    pub data_json: String,
}

/// High-velocity frame data
#[derive(Debug, Clone)]
pub struct HighVelocityFrame {
    pub frame: u64,
    pub velocity: Vec3,
    pub magnitude: f32,
}

/// Constraint break data from database
#[derive(Debug, Clone)]
pub struct ConstraintBreakData {
    pub frame: u64,
    pub timestamp: f64,
    pub entity_a: Option<u64>,
    pub entity_b: Option<u64>,
    pub data_json: String,
}

/// Solver failure data from database
#[derive(Debug, Clone)]
pub struct SolverFailureData {
    pub frame: u64,
    pub timestamp: f64,
    pub data_json: String,
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStatistics {
    pub total_frames: usize,
    pub total_entities: usize,
    pub total_events: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_debug::{PhysicsDebugSnapshot, SqliteExporter};
    use tempfile::NamedTempFile;

    fn setup_test_database() -> (NamedTempFile, PhysicsQueryAPI) {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Create and populate database
        let mut exporter = SqliteExporter::create(path).unwrap();

        for frame in 0..10 {
            let mut snapshot = PhysicsDebugSnapshot::new(frame, frame as f64 * 0.016);
            snapshot.entities.push(EntityState {
                id: 1,
                position: Vec3::new(frame as f32, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                linear_velocity: Vec3::new((frame * 10) as f32, 0.0, 0.0),
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

            exporter.write_snapshot(&snapshot).unwrap();
        }

        drop(exporter);

        let query_api = PhysicsQueryAPI::open(path).unwrap();

        (temp_file, query_api)
    }

    #[test]
    fn test_entity_history() {
        let (_temp, api) = setup_test_database();

        let history = api.entity_history(1, 0, 9).unwrap();
        assert_eq!(history.len(), 10);

        // Verify positions are correct
        assert_eq!(history[0].position.x, 0.0);
        assert_eq!(history[5].position.x, 5.0);
        assert_eq!(history[9].position.x, 9.0);
    }

    #[test]
    fn test_find_high_velocity() {
        let (_temp, api) = setup_test_database();

        // Entities have velocity: 0, 10, 20, 30, ..., 90
        let high_vel = api.find_high_velocity(1, 50.0).unwrap();

        // Should find frames 6, 7, 8, 9 (velocity 60, 70, 80, 90)
        assert_eq!(high_vel.len(), 4);
        assert_eq!(high_vel[0].frame, 6);
        assert!((high_vel[0].magnitude - 60.0).abs() < 0.1);
    }

    #[test]
    fn test_database_statistics() {
        let (_temp, api) = setup_test_database();

        let stats = api.statistics().unwrap();
        assert_eq!(stats.total_frames, 10);
        assert_eq!(stats.total_entities, 1);
    }

    #[test]
    fn test_no_data_error() {
        let (_temp, api) = setup_test_database();

        // Query non-existent entity
        let result = api.entity_history(999, 0, 10);
        assert!(result.is_err());

        match result {
            Err(QueryError::NoData { .. }) => {} // Expected
            _ => panic!("Expected NoData error"),
        }
    }
}
