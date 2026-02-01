//! Compile-fail tests for client/server macro separation
//!
//! These tests verify that client cannot call server-only functions
//! and vice versa. Uses trybuild to ensure compilation fails.

#[test]
#[cfg(feature = "server")]
fn test_server_cannot_call_client_code() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/server_calls_client.rs");
}

#[test]
#[cfg(feature = "client")]
fn test_client_cannot_call_server_code() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/client_calls_server.rs");
}

#[test]
fn test_both_features_disabled() {
    // When neither feature is enabled, both should compile fine
    // (the functions just won't exist)
    let t = trybuild::TestCases::new();
    t.pass("tests/compile_fail/server_calls_client.rs");
    t.pass("tests/compile_fail/client_calls_server.rs");
}
