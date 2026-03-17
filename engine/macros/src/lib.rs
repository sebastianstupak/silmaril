//! Engine Macros
//!
//! Provides procedural macros:
//! - #[derive(Component)] - Component trait implementation
//! - `define_error!` - Error type generation
//! - #[client_only], #[server_only], #[shared], #[server_authoritative] - Client/server separation
//! - System generation macros
//! - Serialization code generation
//! - Network message macros

#![warn(missing_docs)]

mod client_server;
mod error;

use proc_macro::TokenStream;

/// Derive macro for the Component trait
///
/// Automatically implements the Component marker trait for a struct.
/// The struct must be 'static, Send, and Sync (enforced by the trait bounds).
///
/// # Example
/// ```ignore
/// use engine_macros::Component;
///
/// #[derive(Component)]
/// struct Position { x: f32, y: f32, z: f32 }
/// ```
///
/// # Note
/// Works inside engine-core crate and external crates.
#[proc_macro_derive(Component)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Use engine_core::ecs::Component — works both inside engine-core
    // (via `extern crate self as engine_core`) and in external crates.
    let expanded = quote::quote! {
        impl #impl_generics engine_core::ecs::Component for #name #ty_generics #where_clause {}
    };

    TokenStream::from(expanded)
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

/// Marks a function as client-only.
///
/// Functions marked with this attribute are only compiled when building
/// the client binary. Server builds will not include this function.
///
/// # Use Cases
/// - Rendering code
/// - Audio playback
/// - Input handling
/// - UI logic
/// - Client-side predictions
///
/// # Example
/// ```ignore
/// use engine_macros::client_only;
///
/// #[client_only]
/// fn render_health_bar(health: &Health, renderer: &mut Renderer) {
///     // Only compiled in client builds
///     renderer.draw_bar(health.current / health.max);
/// }
/// ```
///
/// # Compile-Time Enforcement
/// If server code tries to call a client-only function, compilation will fail.
#[proc_macro_attribute]
pub fn client_only(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item2 = proc_macro2::TokenStream::from(item);
    match client_server::client_only_impl(item2) {
        Ok(output) => TokenStream::from(output),
        Err(e) => TokenStream::from(e.to_compile_error()),
    }
}

/// Marks a function as server-only.
///
/// Functions marked with this attribute are only compiled when building
/// the server binary. Client builds will not include this function.
///
/// # Use Cases
/// - Anti-cheat validation
/// - Authoritative game logic
/// - Economy systems
/// - Loot table generation
/// - Server-side AI
/// - Persistence/database operations
///
/// # Example
/// ```ignore
/// use engine_macros::server_only;
///
/// #[server_only]
/// fn validate_damage(attacker: Entity, target: Entity, amount: f32) -> bool {
///     // Only compiled in server builds
///     // Anti-cheat: Check if damage is legitimate
///     amount > 0.0 && amount < 1000.0
/// }
/// ```
///
/// # Security
/// Server-only code cannot be inspected by clients, preventing:
/// - Loot table reverse engineering
/// - Economy exploits
/// - Cheat detection bypass
#[proc_macro_attribute]
pub fn server_only(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item2 = proc_macro2::TokenStream::from(item);
    match client_server::server_only_impl(item2) {
        Ok(output) => TokenStream::from(output),
        Err(e) => TokenStream::from(e.to_compile_error()),
    }
}

/// Marks a function as shared between client and server.
///
/// Both client and server builds will include this function with
/// identical implementation.
///
/// # Use Cases
/// - Math utilities
/// - Physics calculations
/// - Shared game rules
/// - Utility functions
///
/// # Example
/// ```ignore
/// use engine_macros::shared;
///
/// #[shared]
/// fn calculate_distance(a: Vec3, b: Vec3) -> f32 {
///     // Both client and server execute the same code
///     (a - b).magnitude()
/// }
/// ```
///
/// # Note
/// For determinism in networked games, ensure shared functions
/// produce identical results on all platforms.
#[proc_macro_attribute]
pub fn shared(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item2 = proc_macro2::TokenStream::from(item);
    match client_server::shared_impl(item2) {
        Ok(output) => TokenStream::from(output),
        Err(e) => TokenStream::from(e.to_compile_error()),
    }
}

/// Marks a function as server-authoritative with client prediction.
///
/// Generates two implementations:
/// - Server: Uses provided implementation (authoritative)
/// - Client: Uses simplified prediction (default stub or custom)
///
/// # Use Cases
/// - Damage calculation
/// - Movement validation
/// - Ability cooldowns
/// - Resource consumption
/// - Any gameplay logic that needs prediction
///
/// # Example - Default (Client gets unimplemented!())
/// ```ignore
/// use engine_macros::server_authoritative;
///
/// #[server_authoritative]
/// fn apply_damage(target: Entity, amount: f32) -> f32 {
///     // Server implementation (authoritative)
///     let actual_damage = amount * 0.9; // Apply armor
///     actual_damage
/// }
/// // Client will get unimplemented!() and must be implemented separately
/// ```
///
/// # Example - Custom Client Prediction
/// ```ignore
/// #[server_authoritative(client = {
///     // Simplified client prediction (no armor calculation)
///     amount
/// })]
/// fn apply_damage(target: Entity, amount: f32) -> f32 {
///     // Server implementation
///     let actual_damage = amount * calculate_armor_reduction(target);
///     actual_damage
/// }
/// ```
///
/// # Server Reconciliation
/// Client prediction may be wrong. Server sends authoritative result,
/// client reconciles difference.
#[proc_macro_attribute]
pub fn server_authoritative(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr2 = proc_macro2::TokenStream::from(attr);
    let item2 = proc_macro2::TokenStream::from(item);
    match client_server::server_authoritative_impl(attr2, item2) {
        Ok(output) => TokenStream::from(output),
        Err(e) => TokenStream::from(e.to_compile_error()),
    }
}
