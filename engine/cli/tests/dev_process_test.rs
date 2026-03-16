// Tests for ProcessManager state machine and ProcessKiller factory.

#[tokio::test]
async fn test_process_manager_state_starts_stopped() {
    use silm::commands::dev::process::{ProcessManager, ProcessState};
    let manager = ProcessManager::new("test-package".to_string(), 9999);
    assert!(matches!(manager.state(), ProcessState::Stopped));
}

#[tokio::test]
async fn test_process_manager_package_name_preserved() {
    use silm::commands::dev::process::{ProcessManager, ProcessState};
    let manager = ProcessManager::new("my-game-server".to_string(), 7777);
    assert!(matches!(manager.state(), ProcessState::Stopped));
    assert_eq!(manager.package_name(), "my-game-server");
}

#[tokio::test]
async fn test_create_killer_returns_impl() {
    use silm::commands::dev::process::create_killer;
    // Just verifying create_killer() doesn't panic and returns a valid killer.
    let _killer = create_killer();
}
