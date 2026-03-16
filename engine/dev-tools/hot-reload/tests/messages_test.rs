use engine_dev_tools_hot_reload::messages::ReloadMessage;

#[test]
fn test_reload_asset_round_trip() {
    let msg = ReloadMessage::ReloadAsset {
        path: "assets/textures/grass.png".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::ReloadAsset { path } if path == "assets/textures/grass.png"));
}

#[test]
fn test_reload_config_round_trip() {
    let msg = ReloadMessage::ReloadConfig {
        path: "config/server.ron".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::ReloadConfig { path } if path == "config/server.ron"));
}

#[test]
fn test_serialize_state_round_trip() {
    let msg = ReloadMessage::SerializeState;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::SerializeState));
}

#[test]
fn test_ack_round_trip() {
    let msg = ReloadMessage::Ack;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ReloadMessage = serde_json::from_str(&json).unwrap();
    assert!(matches!(decoded, ReloadMessage::Ack));
}
