use silm::commands::dev::reload_client::DevReloadClient;

#[tokio::test]
async fn test_dev_reload_client_creation() {
    let client = DevReloadClient::new(9999, 9998);
    // Just verify it constructs without panicking
    // Actual sending is tested in the engine-dev-tools tests
    drop(client);
}

#[tokio::test]
async fn test_dev_reload_client_routes_server_config() {
    let client = DevReloadClient::new(9999, 9998);
    // Config routing: server.ron → server port, client.ron → client port
    // This just tests the routing logic, not actual sending
    assert!(client.is_server_config("config/server.ron"));
    assert!(!client.is_server_config("config/client.ron"));
}
