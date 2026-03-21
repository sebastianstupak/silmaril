//! Editor bridge — commands, subscriptions, events.
//!
//! Svelte -> Rust: invoke() commands
//! Rust -> Svelte: emit() events via subscriptions

pub mod builtin_schemas;
pub mod commands;
pub mod events;
pub mod modules;
pub mod schema_registry;
pub mod subscriptions;
pub mod registry;
pub mod registry_bridge;
pub mod runner;
pub mod template_commands;

#[cfg(test)]
pub mod tests;
