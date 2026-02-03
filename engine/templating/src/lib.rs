//! Template system for Silmaril game engine.

#![warn(missing_docs)]

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
