// This test should FAIL to compile when building client
// Client should not be able to call server-only functions

use engine_macros::server_only;

#[server_only]
fn validate_damage() -> bool {
    true
}

#[cfg(feature = "client")]
fn main() {
    // This should fail: client calling server-only function
    validate_damage();
}

#[cfg(not(feature = "client"))]
fn main() {}
