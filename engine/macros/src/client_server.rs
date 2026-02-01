//! Client/Server Separation Macros
//!
//! These macros enforce compile-time separation between client and server code,
//! enabling single-codebase multiplayer with strong guarantees about code placement.
//!
//! # Patterns
//!
//! 1. **#[client_only]** - Code ONLY runs on client (rendering, audio, input)
//! 2. **#[server_only]** - Code ONLY runs on server (anti-cheat, economy, loot)
//! 3. **#[shared]** - Code runs on both (math, physics, utility functions)
//! 4. **#[server_authoritative]** - Different implementations (server: authoritative, client: prediction)
//!
//! # Examples
//!
//! ```ignore
//! #[client_only]
//! fn render_health_bar(health: &Health, renderer: &mut Renderer) {
//!     // Only compiled in client builds
//!     renderer.draw_bar(health.current / health.max);
//! }
//!
//! #[server_only]
//! fn validate_damage(attacker: Entity, target: Entity, amount: f32) -> bool {
//!     // Only compiled in server builds
//!     // Anti-cheat logic here
//!     amount > 0.0 && amount < 1000.0
//! }
//!
//! #[shared]
//! fn calculate_distance(a: Vec3, b: Vec3) -> f32 {
//!     // Both client and server execute
//!     (a - b).magnitude()
//! }
//!
//! #[server_authoritative]
//! fn apply_damage(target: Entity, amount: f32) -> f32 {
//!     // Server implementation (authoritative)
//!     // Client implementation (prediction) generated separately
//! }
//! ```

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, ItemFn, Result};

/// Implements #[client_only] macro
///
/// Expands to `#[cfg(feature = "client")]`
pub fn client_only_impl(item: TokenStream) -> Result<TokenStream> {
    let input = parse2::<ItemFn>(item)?;

    Ok(quote! {
        #[cfg(feature = "client")]
        #input
    })
}

/// Implements #[server_only] macro
///
/// Expands to `#[cfg(feature = "server")]`
pub fn server_only_impl(item: TokenStream) -> Result<TokenStream> {
    let input = parse2::<ItemFn>(item)?;

    Ok(quote! {
        #[cfg(feature = "server")]
        #input
    })
}

/// Implements #[shared] macro
///
/// Expands to `#[cfg(any(feature = "client", feature = "server"))]`
pub fn shared_impl(item: TokenStream) -> Result<TokenStream> {
    let input = parse2::<ItemFn>(item)?;

    Ok(quote! {
        #[cfg(any(feature = "client", feature = "server"))]
        #input
    })
}

/// Implements #[server_authoritative] macro
///
/// Generates two implementations:
/// - Server: Uses provided implementation (authoritative)
/// - Client: Uses simplified prediction (default or custom via attribute)
///
/// # Syntax
///
/// ```ignore
/// // Default: Client gets unimplemented!() stub
/// #[server_authoritative]
/// fn calculate_damage(base: f32, armor: f32) -> f32 {
///     // Server implementation
/// }
///
/// // Custom client implementation
/// #[server_authoritative(client = {
///     // Simplified prediction
///     base * 0.5
/// })]
/// fn calculate_damage(base: f32, armor: f32) -> f32 {
///     // Server implementation
/// }
/// ```
pub fn server_authoritative_impl(
    attr: TokenStream,
    item: TokenStream,
) -> Result<TokenStream> {
    let input = parse2::<ItemFn>(item)?;
    let fn_sig = &input.sig;
    let server_block = &input.block;

    // Parse attribute for custom client implementation
    let client_block = if attr.is_empty() {
        // Default: unimplemented stub for client
        quote! {{
            // TODO: Implement client prediction
            // This is authoritative on server, estimated on client
            unimplemented!("Client prediction not implemented for {}", stringify!(#fn_sig))
        }}
    } else {
        // User provided custom client implementation
        attr
    };

    Ok(quote! {
        // Server gets the authoritative implementation
        #[cfg(feature = "server")]
        #fn_sig #server_block

        // Client gets simplified/predicted implementation
        #[cfg(feature = "client")]
        #fn_sig #client_block
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_client_only_expansion() {
        let input = quote! {
            fn render() {
                println!("Rendering");
            }
        };

        let output = client_only_impl(input).unwrap();
        let expected = quote! {
            #[cfg(feature = "client")]
            fn render() {
                println!("Rendering");
            }
        };

        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_server_only_expansion() {
        let input = quote! {
            fn validate() -> bool {
                true
            }
        };

        let output = server_only_impl(input).unwrap();
        let expected = quote! {
            #[cfg(feature = "server")]
            fn validate() -> bool {
                true
            }
        };

        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_shared_expansion() {
        let input = quote! {
            fn calculate(a: f32, b: f32) -> f32 {
                a + b
            }
        };

        let output = shared_impl(input).unwrap();
        let expected = quote! {
            #[cfg(any(feature = "client", feature = "server"))]
            fn calculate(a: f32, b: f32) -> f32 {
                a + b
            }
        };

        assert_eq!(output.to_string(), expected.to_string());
    }

    #[test]
    fn test_server_authoritative_default() {
        let input = quote! {
            fn apply_damage(amount: f32) -> f32 {
                amount * 0.9
            }
        };

        let output = server_authoritative_impl(TokenStream::new(), input).unwrap();

        // Should contain both server and client implementations
        let output_str = output.to_string();
        eprintln!("Output: {}", output_str);
        assert!(output_str.contains("cfg (feature = \"server\")"));
        assert!(output_str.contains("cfg (feature = \"client\")"));
        assert!(output_str.contains("unimplemented")); // Check without !
        assert!(output_str.contains("0.9")); // Server implementation
    }

    #[test]
    fn test_server_authoritative_custom_client() {
        let input = quote! {
            fn apply_damage(amount: f32) -> f32 {
                amount * 0.9
            }
        };

        let client_impl = quote! {{
            amount * 0.8  // Client prediction
        }};

        let output = server_authoritative_impl(client_impl, input).unwrap();

        // Should contain both implementations
        let output_str = output.to_string();
        assert!(output_str.contains("cfg (feature = \"server\")"));
        assert!(output_str.contains("cfg (feature = \"client\")"));
        assert!(output_str.contains("0.9")); // Server implementation
        assert!(output_str.contains("0.8")); // Client implementation
    }
}
