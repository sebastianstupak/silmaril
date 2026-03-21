//! Editor bridge — commands, subscriptions, events.
//!
//! Svelte -> Rust: invoke() commands
//! Rust -> Svelte: emit() events via subscriptions

pub mod builtin_schemas;
pub mod commands;
pub mod events;
pub mod gizmo_commands;
pub mod schema_registry;
pub mod subscriptions;
pub mod registry;
pub mod runner;
pub mod template_commands;
