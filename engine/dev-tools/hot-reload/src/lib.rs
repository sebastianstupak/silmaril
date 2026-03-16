//! Development hot-reload infrastructure for Silmaril game projects.
//!
//! This crate provides the protocol messages and supporting types for the
//! `silm dev` hot-reload workflow, enabling live asset and config reloading
//! without restarting the game process.

pub mod error;
pub mod force_reload;
pub mod handoff;
pub mod messages;
