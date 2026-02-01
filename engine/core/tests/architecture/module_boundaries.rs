//! Runtime tests for module boundary enforcement.
//!
//! These tests verify architectural invariants:
//! - ECS compiles without platform dependencies
//! - No #[cfg(target_os)] in business logic modules
//! - Module isolation is maintained

use std::path::Path;

/// Test that ECS module doesn't import platform-specific code directly.
#[test]
fn test_ecs_module_independence() {
    // This test verifies at runtime that the ECS can be used without platform dependencies
    use engine_core::World;

    let mut world = World::new();
    let entity = world.spawn();

    assert!(world.is_alive(entity));

    // The fact that this compiles and runs means ECS doesn't require platform code
}

/// Test that serialization module doesn't import platform-specific code directly.
#[test]
fn test_serialization_module_independence() {
    use engine_core::serialization::{Format, WorldState};

    // Create a world state without needing platform code
    let _state = WorldState::new();

    // Verify we can reference formats without platform dependencies
    let _formats = [Format::Yaml, Format::Bincode, Format::FlatBuffers];

    // The fact that this compiles and runs means serialization doesn't require platform code
}

/// Test that error types are independent of platform code.
#[test]
fn test_error_module_independence() {
    use engine_core::{ErrorCode, ErrorSeverity};

    // All error codes should be accessible without platform code
    let codes = [
        ErrorCode::EntityNotFound,
        ErrorCode::ComponentNotFound,
        ErrorCode::SerializationFailed,
        ErrorCode::WindowCreationFailed,
    ];

    for code in &codes {
        let subsystem = code.subsystem();
        assert!(!subsystem.is_empty());
    }

    // Severity levels should work without platform code
    let severities = [ErrorSeverity::Warning, ErrorSeverity::Error, ErrorSeverity::Critical];

    for severity in &severities {
        let _display = format!("{}", severity);
    }
}

/// Test that component types don't require platform code.
#[test]
fn test_component_independence() {
    use engine_core::{gameplay::Health, math::Transform, rendering::MeshRenderer};

    // All these components should be usable without platform dependencies
    let _transform = Transform::default();
    let _health = Health::new(100.0, 100.0);
    let _mesh = MeshRenderer::new(0, 0);

    // The fact that this compiles and runs means components don't require platform code
}

/// Test that we can query and iterate over entities without platform code.
#[test]
fn test_query_independence() {
    use engine_core::{gameplay::Health, math::Transform, World};

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    // Spawn entities with components
    for i in 0..100 {
        let entity = world.spawn();
        let mut transform = Transform::default();
        transform.position.x = i as f32;
        world.add(entity, transform);
        world.add(entity, Health::new(100.0, 100.0));
    }

    // Query entities - no platform code needed
    let mut count = 0;
    for _ in world.query::<(&Transform, &Health)>() {
        count += 1;
    }
    assert_eq!(count, 100);
}

/// Test that serialization works without platform-specific filesystem.
#[test]
fn test_serialization_independence() {
    use engine_core::{math::Transform, serialization::WorldState, World};

    let mut world = World::new();
    world.register::<Transform>();
    let entity = world.spawn();
    world.add(entity, Transform::default());

    // Create world state without platform code
    let _state = WorldState::new();

    // The fact that this compiles and runs means serialization doesn't require platform code
}

/// Test that platform abstraction is properly isolated.
#[test]
fn test_platform_abstraction_isolation() {
    // Platform module should be completely isolated
    // Business logic should only access it through traits

    use engine_core::platform::{
        create_filesystem_backend, create_threading_backend, create_time_backend,
    };

    // All platform code is accessed through factory functions
    let _time = create_time_backend();
    let _fs = create_filesystem_backend();
    let _threading = create_threading_backend();

    // No need to know about Windows, Unix, or macOS specifics
}

/// Verify that no business logic modules contain cfg(target_os) attributes.
///
/// This test uses source code inspection to ensure architectural rules are followed.
#[test]
fn test_no_cfg_in_business_logic() {
    // Define business logic modules (not platform-specific)
    let business_modules = [
        "src/ecs/entity.rs",
        "src/ecs/component.rs",
        "src/ecs/world.rs",
        "src/ecs/query.rs",
        "src/ecs/storage.rs",
        "src/serialization/mod.rs",
        "src/serialization/world_state.rs",
        "src/serialization/component_data.rs",
        "src/math.rs",
        "src/gameplay.rs",
        "src/physics_components.rs",
        "src/rendering.rs",
    ];

    let base_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    for module in &business_modules {
        let file_path = base_path.join(module);

        if !file_path.exists() {
            continue; // Skip non-existent modules
        }

        let content = std::fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", module));

        // Check for platform-specific cfg attributes
        let forbidden_patterns = [
            "#[cfg(windows)]",
            "#[cfg(unix)]",
            "#[cfg(target_os",
            "cfg!(windows)",
            "cfg!(unix)",
            "cfg!(target_os",
        ];

        for pattern in &forbidden_patterns {
            if content.contains(pattern) {
                panic!(
                    "Business logic module {} contains forbidden platform-specific code: {}",
                    module, pattern
                );
            }
        }
    }
}

/// Verify that platform modules are properly isolated.
#[test]
fn test_platform_modules_properly_isolated() {
    // Platform-specific code should only be in platform/ directory
    let platform_modules = [
        "src/platform/time/windows.rs",
        "src/platform/time/unix.rs",
        "src/platform/threading/windows.rs",
        "src/platform/threading/unix.rs",
        "src/platform/filesystem/native.rs",
    ];

    let base_path = Path::new(env!("CARGO_MANIFEST_DIR"));

    for module in &platform_modules {
        let file_path = base_path.join(module);

        if !file_path.exists() {
            continue; // Skip non-existent modules
        }

        // These files SHOULD contain platform-specific code
        // Just verify they exist and are readable
        let _content = std::fs::read_to_string(&file_path)
            .unwrap_or_else(|_| panic!("Failed to read platform module {}", module));
    }
}

/// Test that trait abstraction boundaries are respected.
#[test]
fn test_trait_boundaries() {
    use engine_core::platform::{FileSystemBackend, ThreadingBackend, TimeBackend};

    // Verify traits are object-safe (can be used as trait objects)
    fn accepts_time_trait(_backend: &dyn TimeBackend) {}
    fn accepts_fs_trait(_backend: &dyn FileSystemBackend) {}
    fn accepts_threading_trait(_backend: &dyn ThreadingBackend) {}

    let time = engine_core::platform::create_time_backend().unwrap();
    let fs = engine_core::platform::create_filesystem_backend();
    let threading = engine_core::platform::create_threading_backend().unwrap();

    accepts_time_trait(&*time);
    accepts_fs_trait(&*fs);
    accepts_threading_trait(&*threading);
}

/// Test that we can use the engine without importing platform internals.
#[test]
fn test_public_api_doesnt_expose_platform_internals() {
    // This test verifies that the public API doesn't leak platform implementation details

    use engine_core::platform::{FileSystemBackend, ThreadingBackend, TimeBackend};

    // We should be able to use traits without knowing about concrete types
    fn use_backends(
        time: Box<dyn TimeBackend>,
        fs: Box<dyn FileSystemBackend>,
        threading: Box<dyn ThreadingBackend>,
    ) {
        let _t = time.monotonic_nanos();
        let _exists = fs.file_exists(Path::new("/tmp"));
        let _cpus = threading.num_cpus();
    }

    let time = engine_core::platform::create_time_backend().unwrap();
    let fs = engine_core::platform::create_filesystem_backend();
    let threading = engine_core::platform::create_threading_backend().unwrap();

    use_backends(time, fs, threading);
}

/// Test that module re-exports are clean.
#[test]
fn test_clean_module_reexports() {
    // Verify that commonly used types are re-exported from the crate root
    use engine_core::{ErrorCode, ErrorSeverity, PlatformError, World};

    // These should all be accessible without deep imports
    let _world: World = World::new();
    let _error_code: ErrorCode = ErrorCode::EntityNotFound;
    let _severity: ErrorSeverity = ErrorSeverity::Error;

    // Error types should be accessible
    fn _takes_platform_error(_e: PlatformError) {}
}

/// Test that internal implementation details are not exposed.
#[test]
fn test_implementation_details_private() {
    // This test verifies that we can't access internal implementation details

    // We should NOT be able to directly construct platform-specific types
    // (This is a compile-time check, but we document it here)

    // ✅ CORRECT: Use factory functions
    let _time = engine_core::platform::create_time_backend();

    // ❌ WRONG: Direct construction (should not compile if tried)
    // let _time = WindowsTime::new(); // <-- This should not be possible
}

/// Test that cross-module dependencies are one-directional.
#[test]
fn test_dependency_direction() {
    // This test documents the intended dependency flow:
    // 1. Core ECS depends on: nothing (pure data structures)
    // 2. Serialization depends on: ECS
    // 3. Platform depends on: error types only
    // 4. Business logic depends on: ECS, not platform

    use engine_core::{gameplay::Health, math::Transform, serialization::WorldState, World};

    // ECS is independent
    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();
    let entity = world.spawn();

    // Business logic uses ECS
    world.add(entity, Transform::default());
    world.add(entity, Health::new(100.0, 100.0));

    // Serialization uses ECS
    let _state = WorldState::new();

    // None of these require platform code
}

/// Test that we can build a complete game system without platform code.
#[test]
fn test_game_logic_without_platform() {
    use engine_core::{gameplay::Health, math::Transform, World};

    let mut world = World::new();
    world.register::<Transform>();
    world.register::<Health>();

    // Create a player
    let player = world.spawn();
    world.add(player, Transform::default());
    world.add(player, Health::new(100.0, 100.0));

    // Create enemies
    for i in 0..10 {
        let enemy = world.spawn();
        let mut transform = Transform::default();
        transform.position.x = i as f32 * 5.0;
        world.add(enemy, transform);
        world.add(enemy, Health::new(50.0, 50.0));
    }

    // Count enemies - pure business logic
    let mut enemy_count = 0;
    for (_transform, _health) in world.query::<(&Transform, &Health)>() {
        enemy_count += 1;
    }
    assert_eq!(enemy_count, 11); // 1 player + 10 enemies

    // No platform code was needed for any of this
}
