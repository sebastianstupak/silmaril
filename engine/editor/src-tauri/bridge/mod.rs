//! Editor bridge — commands, subscriptions, events.
//!
//! Svelte -> Rust: invoke() commands
//! Rust -> Svelte: emit() events via subscriptions

pub mod builtin_schemas;
pub mod commands;
pub mod events;
pub mod schema_registry;
pub mod subscriptions;
