// TODO: This module is superseded by engine_ops::project (BasicTemplate, TemplateFile, etc.).
// Kept during incremental CLI -> ops migration. Remove once all commands use engine_ops.

pub mod basic;

pub use basic::BasicTemplate;

pub trait Template {
    fn name(&self) -> &str;
    fn files(&self) -> Vec<TemplateFile>;
}

pub struct TemplateFile {
    pub path: String,
    pub content: String,
}

impl TemplateFile {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self { path: path.into(), content: content.into() }
    }
}
