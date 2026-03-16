//! Cross-crate test: ForceReloader bridge between async TCP and HotReloader.
//! In engine/shared/tests/ because it imports engine-assets + engine-dev-tools.

#![cfg(feature = "hot-reload")]

use engine_assets::{
    hot_reload::{HotReloadConfig, HotReloader},
    AssetId, AssetManager,
};
use engine_dev_tools_hot_reload::force_reload::ForceReloader;
use std::sync::{Arc, Mutex};

#[test]
fn test_force_reloader_queues_registered_asset() {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let hot_reloader = Arc::new(Mutex::new(
        HotReloader::new(manager, config).expect("create reloader"),
    ));

    let path = std::path::PathBuf::from("assets/textures/test.png");
    let asset_id = AssetId::from_content(b"test_asset");
    {
        let mut r = hot_reloader.lock().unwrap();
        r.register_asset(path.clone(), asset_id);
    }

    let force = ForceReloader::new(hot_reloader);
    let result = force.reload(path.to_str().unwrap());
    assert!(result.is_ok(), "reload of registered asset should succeed: {result:?}");
}

#[test]
fn test_force_reloader_errors_on_unregistered() {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let hot_reloader = Arc::new(Mutex::new(
        HotReloader::new(manager, config).expect("create reloader"),
    ));

    let force = ForceReloader::new(hot_reloader);
    let result = force.reload("assets/unknown/nope.png");
    assert!(result.is_err(), "reload of unregistered asset should fail");
}
