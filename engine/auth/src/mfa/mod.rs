//! Multi-Factor Authentication (MFA) module.
//!
//! Provides TOTP (Time-based One-Time Password) and backup codes.

pub mod backup_codes;
pub mod totp;

pub use backup_codes::{BackupCode, BackupCodeManager};
pub use totp::{TotpManager, TotpSetup};
