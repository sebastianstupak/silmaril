use anyhow::Result;
use std::path::Path;

pub fn remove_module(module_name: &str, project_root: &Path) -> Result<()> {
    engine_ops::module::remove::remove_module(module_name, project_root)
}
