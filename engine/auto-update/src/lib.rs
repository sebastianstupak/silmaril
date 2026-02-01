//! Engine Auto-Update
//!
//! Provides automatic game updates:
//! - Update detection and download
//! - Delta patching
//! - Background updates
//! - Safe installation with rollback

#![warn(missing_docs)]

pub mod manager;
pub mod downloader;
pub mod patcher;
pub mod installer;

// Re-export commonly used types
pub use manager::{UpdateManager, UpdateStatus};
