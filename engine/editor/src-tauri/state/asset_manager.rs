use engine_assets::AssetManager;
use std::sync::Arc;

/// Tauri-managed shared asset manager.
///
/// Wrapped in `Arc` so it can be cloned into background threads (e.g. the render thread).
pub struct AssetManagerState(pub Arc<AssetManager>);
