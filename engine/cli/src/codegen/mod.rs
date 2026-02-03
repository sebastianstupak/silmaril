//! Code generation for components and systems.
//!
//! This module provides functionality to generate Rust code for ECS components
//! and systems via CLI commands like `silm add component` and `silm add system`.
//!
//! # Module Organization
//!
//! - [`parser`]: Parses field definitions and query components from CLI input
//! - [`validator`]: Validates naming conventions and type syntax
//!
//! # Examples
//!
//! ```ignore
//! use silm::codegen::{parser, validator};
//!
//! // Parse component fields
//! let fields = parser::parse_fields("current:f32,max:f32")?;
//!
//! // Validate component name
//! validator::validate_pascal_case("Health")?;
//!
//! // Generate default value for type
//! let default = parser::default_value_for_type("f32");
//! assert_eq!(default, "0.0");
//! ```

pub mod component;
pub mod module_exports;
pub mod parser;
pub mod registry;
pub mod system;
pub mod validator;

// Re-export commonly used items
pub use component::{
    default_value_for_type, extract_array_type, generate_component_code, parse_fields,
};
pub use module_exports::{to_snake_case, update_module_exports};
pub use parser::{parse_query_components, QueryAccess, QueryComponent};
pub use system::{generate_system_code, SystemPhase};
pub use validator::{
    validate_field_name, validate_pascal_case, validate_snake_case, validate_type_syntax,
};
