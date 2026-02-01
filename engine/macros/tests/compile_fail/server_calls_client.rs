// This test should FAIL to compile when building server
// Server should not be able to call client-only functions

use engine_macros::client_only;

#[client_only]
fn render_health_bar() {
    println!("Rendering...");
}

#[cfg(feature = "server")]
fn main() {
    // This should fail: server calling client-only function
    render_health_bar();
}

#[cfg(not(feature = "server"))]
fn main() {}
