//! Simple E2E test to verify test infrastructure works
//!
//! MANDATORY: Cross-crate test using engine-core + engine-networking

use engine_core::ecs::World;
use engine_networking::ServerLoop;

#[tokio::test]
async fn test_e2e_infrastructure_basic() {
    // Just verify we can import and create the types
    let world = World::new();
    let server_loop = ServerLoop::new(world);

    assert_eq!(server_loop.tick(), 0);
    assert_eq!(server_loop.client_count().await, 0);
}
