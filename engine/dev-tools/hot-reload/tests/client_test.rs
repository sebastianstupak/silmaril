use engine_dev_tools_hot_reload::client::ReloadClient;

#[tokio::test]
async fn test_reload_client_send_fails_gracefully_when_no_server() {
    // No server on port 19998 — should log and return Ok (best-effort)
    let client = ReloadClient::new(19998);
    let result = client.send_reload_asset("assets/textures/test.png").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_reload_client_serialize_state_fails_gracefully_on_no_server() {
    let client = ReloadClient::new(19997);
    let result = client.send_serialize_state().await;
    // serialize_state failure is best-effort too
    assert!(result.is_ok());
}
