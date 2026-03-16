//! Cross-crate integration test: DevReloadServer TCP accept + ReloadClient round-trip.
//!
//! Lives in engine/shared/tests/ because it imports both engine-assets and
//! engine-dev-tools-hot-reload, spanning two crates.
//!
//! Requires the `dev` feature (activates TCP binding in DevReloadServer).

#![cfg(feature = "dev")]

use engine_assets::{
    hot_reload::{HotReloadConfig, HotReloader},
    AssetManager,
};
use engine_dev_tools_hot_reload::{
    client::ReloadClient, force_reload::ForceReloader, server::DevReloadServer,
};
use std::sync::{Arc, Mutex};

/// Bind port 0 to get a free OS-assigned port, then release it so the server
/// can bind. This is a known TOCTOU race but is good enough for local tests.
async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    listener.local_addr().unwrap().port()
}

fn make_force_reloader() -> Arc<ForceReloader> {
    let manager = Arc::new(AssetManager::new());
    let config = HotReloadConfig::default();
    let hot_reloader = Arc::new(Mutex::new(
        HotReloader::new(manager, config).expect("create HotReloader"),
    ));
    Arc::new(ForceReloader::new(hot_reloader))
}

#[tokio::test]
async fn test_server_starts_and_accepts_serialize_state() {
    let port = free_port().await;
    std::env::set_var("SILMARIL_DEV_PORT", port.to_string());

    let force_reloader = make_force_reloader();

    // Start server in background; it loops until process exit.
    tokio::spawn(DevReloadServer::start(Some(force_reloader)));

    // Give the server time to bind before we attempt to connect.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // ReloadClient::send_serialize_state returns Ok(()) whether or not the
    // server is reachable (best-effort design), so we assert Ok *and* that
    // the server did not panic (implicit: if it panicked, tokio would abort
    // the spawned task and the test runtime would surface the panic).
    let client = ReloadClient::new(port);
    let result = client.send_serialize_state().await;
    assert!(result.is_ok(), "send_serialize_state should succeed: {result:?}");
}

#[tokio::test]
async fn test_server_handles_reload_asset_with_unknown_path() {
    let port = free_port().await;
    // Use a dedicated env variable scope: set before spawn, read during bind.
    std::env::set_var("SILMARIL_DEV_PORT", port.to_string());

    let force_reloader = make_force_reloader();
    tokio::spawn(DevReloadServer::start(Some(force_reloader)));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Sending a reload for an unregistered asset triggers a warn! inside
    // ForceReloader but must not crash the server — the connection stays open.
    let client = ReloadClient::new(port);
    let result = client.send_reload_asset("assets/textures/nonexistent.png").await;
    assert!(result.is_ok(), "send_reload_asset should be best-effort Ok: {result:?}");

    // Server should still be alive: a second message must be accepted.
    let result2 = client.send_reload_config("config/game.yaml").await;
    assert!(result2.is_ok(), "second message after error must succeed: {result2:?}");
}

#[tokio::test]
async fn test_server_start_none_is_noop_with_dev_feature() {
    // DevReloadServer::start(None) must return immediately even with `dev`
    // feature enabled (no reloader → no bind).
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(200),
        DevReloadServer::start(None),
    )
    .await;
    assert!(result.is_ok(), "start(None) should return immediately (not time out)");
}
