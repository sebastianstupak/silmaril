use anyhow::Result;
use std::path::Path;

pub fn list_modules(project_root: &Path) -> Result<()> {
    engine_ops::module::list::list_modules(project_root)
}
