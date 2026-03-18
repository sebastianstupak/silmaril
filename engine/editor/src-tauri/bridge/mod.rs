//! Editor bridge — commands, subscriptions, events.
//!
//! Svelte -> Rust: invoke() commands
//! Rust -> Svelte: emit() events via subscriptions

pub mod commands;
pub mod events;
pub mod subscriptions;
