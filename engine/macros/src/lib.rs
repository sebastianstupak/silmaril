//! Engine Macros
//!
//! Provides procedural macros:
//! - #[derive(Component)] - Component trait implementation
//! - `define_error!` - Error type generation
//! - System generation macros
//! - Serialization code generation
//! - Network message macros

#![warn(missing_docs)]

mod error;

use proc_macro::TokenStream;

/// Derive macro for the Component trait
///
/// # Example
/// ```ignore
/// #[derive(Component)]
/// struct Position { x: f32, y: f32, z: f32 }
/// ```
#[proc_macro_derive(Component)]
pub fn derive_component(_input: TokenStream) -> TokenStream {
    // TODO: Implement Component derive macro
    TokenStream::new()
}

/// Define a structured error type with error codes and severity levels.
///
/// This macro generates an enum with the `EngineError` trait implemented,
/// providing automatic error codes, severity levels, and structured logging.
///
/// # Syntax
///
/// ```ignore
/// use engine_macros::define_error;
/// use engine_core::{ErrorCode, ErrorSeverity};
///
/// define_error! {
///     pub enum MyError {
///         NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
///         InvalidData { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
///         SystemFailure { details: String } = ErrorCode::VulkanInitFailed, ErrorSeverity::Critical,
///     }
/// }
/// ```
///
/// # Generated Code
///
/// The macro generates:
/// - The error enum with Debug derive
/// - Display implementation with field formatting
/// - Error trait implementation
/// - `EngineError` trait implementation with `code()` and `severity()` methods
///
/// # Example
///
/// ```ignore
/// use engine_macros::define_error;
///
/// define_error! {
///     pub enum AssetError {
///         NotFound { path: String } = ErrorCode::TextureLoadFailed, ErrorSeverity::Error,
///         LoadFailed { path: String, reason: String } = ErrorCode::TextureLoadFailed, ErrorSeverity::Error,
///     }
/// }
///
/// // Usage:
/// let error = AssetError::NotFound { path: "texture.png".to_string() };
/// println!("{}", error); // Prints: NotFound { path: texture.png }
/// assert_eq!(error.code(), ErrorCode::TextureLoadFailed);
/// assert_eq!(error.severity(), ErrorSeverity::Error);
/// ```
#[proc_macro]
pub fn define_error(input: TokenStream) -> TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);
    let output = error::define_error_impl(input2);
    TokenStream::from(output)
}
