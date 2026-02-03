//! Template system for Silmaril game engine.

#![warn(missing_docs)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unused_self)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::only_used_in_recursion)]

pub mod cache;
pub mod compiler;
pub mod error;
pub mod loader;
pub mod loader_optimized;
pub mod template;
pub mod validator;

// Re-export main types for convenience
pub use cache::TemplateCache;
pub use compiler::{CompiledTemplate, TemplateCompiler};
pub use error::{TemplateError, TemplateResult};
pub use loader::{TemplateInstance, TemplateLoader};
pub use loader_optimized::TemplateLoaderOptimized;
pub use template::{EntityDefinition, EntitySource, Template, TemplateMetadata};
pub use validator::{TemplateValidator, ValidationReport};
