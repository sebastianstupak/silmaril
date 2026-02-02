//! Error definition macro implementation.
//!
//! Provides the `define_error!` macro for generating structured error types
//! with error codes, severity levels, and automatic logging.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, Result, Token, Visibility,
};

/// Parsed error definition.
///
/// Syntax:
/// ```ignore
/// define_error! {
///     pub enum MyError {
///         VariantName { field1: Type1, field2: Type2 } = ErrorCode::Code, ErrorSeverity::Level,
///         OtherVariant { field: Type } = ErrorCode::OtherCode, ErrorSeverity::OtherLevel,
///     }
/// }
/// ```
pub struct ErrorDefinition {
    vis: Visibility,
    name: Ident,
    variants: Vec<ErrorVariant>,
}

struct ErrorVariant {
    name: Ident,
    fields: Vec<ErrorField>,
    error_code: syn::Expr,
    severity: syn::Expr,
}

struct ErrorField {
    name: Ident,
    ty: syn::Type,
}

impl Parse for ErrorDefinition {
    fn parse(input: ParseStream) -> Result<Self> {
        let vis: Visibility = input.parse()?;
        input.parse::<Token![enum]>()?;
        let name: Ident = input.parse()?;

        let content;
        syn::braced!(content in input);

        let mut variants = Vec::new();
        while !content.is_empty() {
            variants.push(content.parse()?);
            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(ErrorDefinition { vis, name, variants })
    }
}

impl Parse for ErrorVariant {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;

        // Parse struct-like fields { field1: Type1, field2: Type2 }
        let fields_content;
        syn::braced!(fields_content in input);

        let mut fields = Vec::new();
        while !fields_content.is_empty() {
            let field_name: Ident = fields_content.parse()?;
            fields_content.parse::<Token![:]>()?;
            let field_ty: syn::Type = fields_content.parse()?;
            fields.push(ErrorField { name: field_name, ty: field_ty });

            if !fields_content.is_empty() {
                fields_content.parse::<Token![,]>()?;
            }
        }

        input.parse::<Token![=]>()?;

        // Parse ErrorCode::Variant
        let error_code: syn::Expr = input.parse()?;

        input.parse::<Token![,]>()?;

        // Parse ErrorSeverity::Variant
        let severity: syn::Expr = input.parse()?;

        Ok(ErrorVariant { name, fields, error_code, severity })
    }
}

/// Generate error type definition and implementations.
#[allow(clippy::too_many_lines)]
pub fn define_error_impl(input: TokenStream) -> TokenStream {
    let error_def: ErrorDefinition = match syn::parse2(input) {
        Ok(def) => def,
        Err(e) => return e.to_compile_error(),
    };

    let vis = &error_def.vis;
    let name = &error_def.name;

    // Generate enum definition with optional backtrace field
    let variants: Vec<_> = error_def
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let field_names: Vec<_> = v.fields.iter().map(|f| &f.name).collect();
            let field_types: Vec<_> = v.fields.iter().map(|f| &f.ty).collect();

            quote! {
                #variant_name {
                    #(#field_names: #field_types,)*
                    #[cfg(feature = "backtrace")]
                    backtrace: ::std::backtrace::Backtrace,
                }
            }
        })
        .collect();

    // Generate Display implementation
    let display_arms: Vec<_> = error_def
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let field_names: Vec<_> = v.fields.iter().map(|f| &f.name).collect();

            // Build format string and arguments
            let variant_str = variant_name.to_string();
            if field_names.is_empty() {
                quote! {
                    #name::#variant_name {
                        #[cfg(feature = "backtrace")]
                        backtrace: _,
                    } => write!(f, #variant_str)
                }
            } else {
                // Create format string with field names
                // Use {:?} for Debug formatting to handle all types
                // Need to escape braces: {{ and }} in the format string
                let field_strs: Vec<String> =
                    field_names.iter().map(|fn_| format!("{fn_}: {{:?}}")).collect();
                let format_str = format!("{} {{{{ {} }}}}", variant_str, field_strs.join(", "));

                quote! {
                    #name::#variant_name {
                        #(#field_names,)*
                        #[cfg(feature = "backtrace")]
                        backtrace: _,
                    } =>
                        write!(f, #format_str, #(#field_names),*)
                }
            }
        })
        .collect();

    // Generate EngineError implementation
    let code_arms: Vec<_> = error_def
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let error_code = &v.error_code;
            let field_names: Vec<_> = v.fields.iter().map(|f| &f.name).collect();

            quote! {
                #name::#variant_name {
                    #(#field_names: _,)*
                    #[cfg(feature = "backtrace")]
                    backtrace: _,
                } => #error_code
            }
        })
        .collect();

    let severity_arms: Vec<_> = error_def
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let severity = &v.severity;
            let field_names: Vec<_> = v.fields.iter().map(|f| &f.name).collect();

            quote! {
                #name::#variant_name {
                    #(#field_names: _,)*
                    #[cfg(feature = "backtrace")]
                    backtrace: _,
                } => #severity
            }
        })
        .collect();

    // Generate backtrace implementation
    let backtrace_arms: Vec<_> = error_def
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let field_names: Vec<_> = v.fields.iter().map(|f| &f.name).collect();

            quote! {
                #name::#variant_name {
                    #(#field_names: _,)*
                    #[cfg(feature = "backtrace")]
                    backtrace,
                } => {
                    #[cfg(feature = "backtrace")]
                    return Some(backtrace);
                    #[cfg(not(feature = "backtrace"))]
                    return None;
                }
            }
        })
        .collect();

    // Generate constructor methods for each variant
    let constructor_methods: Vec<_> = error_def
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let method_name =
                Ident::new(&variant_name.to_string().to_lowercase(), variant_name.span());
            let field_names: Vec<_> = v.fields.iter().map(|f| &f.name).collect();
            let field_types: Vec<_> = v.fields.iter().map(|f| &f.ty).collect();

            quote! {
                #[allow(missing_docs)]
                pub fn #method_name(#(#field_names: #field_types),*) -> Self {
                    Self::#variant_name {
                        #(#field_names,)*
                        #[cfg(feature = "backtrace")]
                        backtrace: ::std::backtrace::Backtrace::capture(),
                    }
                }
            }
        })
        .collect();

    // Generate Clone implementation (backtrace field doesn't implement Clone)
    let clone_arms: Vec<_> = error_def
        .variants
        .iter()
        .map(|v| {
            let variant_name = &v.name;
            let field_names: Vec<_> = v.fields.iter().map(|f| &f.name).collect();

            quote! {
                #name::#variant_name {
                    #(#field_names,)*
                    #[cfg(feature = "backtrace")]
                    backtrace: _,
                } => #name::#variant_name {
                    #(#field_names: #field_names.clone(),)*
                    #[cfg(feature = "backtrace")]
                    backtrace: ::std::backtrace::Backtrace::disabled(),
                }
            }
        })
        .collect();

    // Generate the full implementation
    // Types (EngineError, ErrorCode, ErrorSeverity) must be in scope
    quote! {
        #[derive(Debug)]
        #[allow(missing_docs)]
        #vis enum #name {
            #(#variants),*
        }

        impl Clone for #name {
            fn clone(&self) -> Self {
                match self {
                    #(#clone_arms),*
                }
            }
        }

        impl #name {
            #(#constructor_methods)*
        }

        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#display_arms),*
                }
            }
        }

        impl ::std::error::Error for #name {}

        impl EngineError for #name {
            fn code(&self) -> ErrorCode {
                match self {
                    #(#code_arms),*
                }
            }

            fn severity(&self) -> ErrorSeverity {
                match self {
                    #(#severity_arms),*
                }
            }

            fn backtrace(&self) -> Option<&::std::backtrace::Backtrace> {
                match self {
                    #(#backtrace_arms),*
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_parse_error_definition() {
        let input = quote! {
            pub enum TestError {
                NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
                InvalidData { reason: String } = ErrorCode::InvalidFormat, ErrorSeverity::Error,
            }
        };

        let result: Result<ErrorDefinition> = syn::parse2(input);
        assert!(result.is_ok());

        let def = result.unwrap();
        assert_eq!(def.name.to_string(), "TestError");
        assert_eq!(def.variants.len(), 2);
        assert_eq!(def.variants[0].name.to_string(), "NotFound");
        assert_eq!(def.variants[0].fields.len(), 1);
        assert_eq!(def.variants[0].fields[0].name.to_string(), "id");
    }

    #[test]
    fn test_generate_error_code() {
        let input = quote! {
            pub enum TestError {
                NotFound { id: u32 } = ErrorCode::EntityNotFound, ErrorSeverity::Error,
            }
        };

        let output = define_error_impl(input);
        let output_str = output.to_string();

        // Check that enum is generated
        assert!(output_str.contains("enum TestError"));
        // Check that variant is generated
        assert!(output_str.contains("NotFound"));
        // Check that implementations are generated
        assert!(output_str.contains("impl :: std :: fmt :: Display"));
        assert!(output_str.contains("impl :: std :: error :: Error"));
        assert!(output_str.contains("impl EngineError"));
    }
}
