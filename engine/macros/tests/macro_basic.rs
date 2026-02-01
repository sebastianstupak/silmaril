//! Basic integration tests for client/server macros

use engine_macros::{client_only, server_authoritative, server_only, shared};

// Test that macros compile and work correctly

#[shared]
fn shared_function() -> i32 {
    42
}

#[test]
#[cfg(any(feature = "client", feature = "server"))]
fn test_shared_macro_works() {
    assert_eq!(shared_function(), 42);
}

#[client_only]
fn client_function() -> &'static str {
    "client"
}

#[test]
#[cfg(feature = "client")]
fn test_client_only_macro_works() {
    assert_eq!(client_function(), "client");
}

#[server_only]
fn server_function() -> &'static str {
    "server"
}

#[test]
#[cfg(feature = "server")]
fn test_server_only_macro_works() {
    assert_eq!(server_function(), "server");
}

#[server_authoritative]
fn authoritative_function(x: i32) -> i32 {
    // Server implementation
    x * 2
}

// Client version needs to be provided separately since the macro
// generates unimplemented!() by default
#[cfg(feature = "client")]
#[allow(dead_code)]
fn authoritative_function_client_impl(x: i32) -> i32 {
    // Client prediction
    x * 2
}

#[test]
#[cfg(feature = "server")]
fn test_server_authoritative_server_side() {
    assert_eq!(authoritative_function(5), 10);
}

#[test]
fn test_macros_compile_without_features() {
    // This test verifies that the macros compile even when
    // features are not enabled (functions just won't exist)
    assert!(true);
}
