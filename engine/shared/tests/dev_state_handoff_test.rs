//! Cross-crate test: StateHandoff round-trips a World through YAML.
//!
//! Lives in engine/shared/tests/ because it imports both engine-core and
//! engine-dev-tools-hot-reload.

use engine_core::ecs::World;
use engine_dev_tools_hot_reload::handoff::{RestoreResult, StateHandoff};
use tempfile::TempDir;

fn make_test_world() -> World {
    let mut world = World::new();
    let _e1 = world.spawn();
    let _e2 = world.spawn();
    let _e3 = world.spawn();
    world
}

#[test]
fn test_state_handoff_round_trip() {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());

    let world = make_test_world();
    let original_count = world.entity_count();

    handoff.save(&world).expect("save should succeed");
    assert!(handoff.exists(), "state file should exist after save");

    let mut restored_world = World::new();
    let result = handoff.restore(&mut restored_world).expect("restore should succeed");

    assert!(matches!(result, RestoreResult::Restored));
    assert_eq!(restored_world.entity_count(), original_count);
    assert!(!handoff.exists(), "state file should be deleted after restore");
}

#[test]
fn test_state_handoff_corrupt_file_gives_clean_start() {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());

    let state_path = dir.path().join(".silmaril").join("dev-state.yaml");
    std::fs::create_dir_all(state_path.parent().unwrap()).unwrap();
    std::fs::write(&state_path, b"not: valid: yaml: [[[[").unwrap();

    let mut world = World::new();
    let result =
        handoff.restore(&mut world).expect("restore should not error on corrupt file");

    assert!(matches!(result, RestoreResult::CleanStart));
    assert!(!state_path.exists(), "corrupt state file should be deleted");
}

#[test]
fn test_state_handoff_missing_file_gives_clean_start() {
    let dir = TempDir::new().unwrap();
    let handoff = StateHandoff::new(dir.path());

    let mut world = World::new();
    let result =
        handoff.restore(&mut world).expect("restore should succeed even with no file");

    assert!(matches!(result, RestoreResult::CleanStart));
}
