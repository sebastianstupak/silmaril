use engine_assets::{AssetId, AssetLoader, AssetManager, MeshData};

/// Register built-in primitive meshes in the asset manager at editor startup.
///
/// Seeds 1–5 correspond to:
/// - `builtin://cube`     → seed 1
/// - `builtin://sphere`   → seed 2
/// - `builtin://plane`    → seed 3
/// - `builtin://cylinder` → seed 4
/// - `builtin://capsule`  → seed 5
///
/// All are stored under `AssetId::from_seed_and_params(seed, b"mesh")`.
pub fn register_primitives(manager: &AssetManager) {
    let primitives: [(u64, MeshData); 5] = [
        (1, MeshData::cube()),
        (2, MeshData::sphere(1.0, 32, 16)),
        (3, MeshData::plane(1.0)),
        (4, MeshData::cylinder(0.5, 1.0, 32)),
        (5, MeshData::capsule(0.5, 1.0, 32, 8)),
    ];
    for (seed, mesh) in primitives {
        let id = AssetId::from_seed_and_params(seed, b"mesh");
        let _ = <MeshData as AssetLoader>::insert(manager, id, mesh);
    }
    tracing::info!("Built-in primitives registered (cube, sphere, plane, cylinder, capsule)");
}
