use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    File,
    Dir,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub kind: NodeKind,
    /// None = not yet expanded (dirs only). Some([]) = empty dir.
    pub children: Option<Vec<TreeNode>>,
    pub git_status: Option<String>,
    pub ignored: bool,
}

/// Read one level of a directory. Dirs get children = None (lazy).
/// Entries are sorted: dirs first, then files, both alphabetical.
pub fn read_dir_one_level(dir: &Path) -> Result<Vec<TreeNode>, String> {
    let mut entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory: {e}"))?
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        b_is_dir.cmp(&a_is_dir).then(a.file_name().cmp(&b.file_name()))
    });

    Ok(entries
        .into_iter()
        .map(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            TreeNode {
                name: e.file_name().to_string_lossy().into_owned(),
                path: e.path().to_string_lossy().into_owned(),
                kind: if is_dir { NodeKind::Dir } else { NodeKind::File },
                children: None, // always None on initial load; expand_dir populates dirs lazily
                git_status: None,
                ignored: false,
            }
        })
        .collect())
}

#[tauri::command]
pub fn get_file_tree(root: String) -> Result<Vec<TreeNode>, String> {
    read_dir_one_level(Path::new(&root))
}

#[tauri::command]
pub fn expand_dir(path: String) -> Result<Vec<TreeNode>, String> {
    read_dir_one_level(Path::new(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_get_file_tree_one_level_deep() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::write(root.join("file.txt"), "").unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        fs::write(root.join("subdir").join("nested.txt"), "").unwrap();

        let nodes = read_dir_one_level(root).unwrap();

        // Should have 2 entries: file.txt and subdir
        assert_eq!(nodes.len(), 2);
        // subdir should have children = None (not expanded yet)
        let dir_node = nodes.iter().find(|n| n.kind == NodeKind::Dir).unwrap();
        assert!(dir_node.children.is_none());
        // file should have kind File
        let file_node = nodes.iter().find(|n| n.kind == NodeKind::File).unwrap();
        assert_eq!(file_node.name, "file.txt");
    }

    #[test]
    fn test_file_nodes_have_null_children() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::write(root.join("file.txt"), "").unwrap();

        let nodes = read_dir_one_level(root).unwrap();
        let file_node = nodes.iter().find(|n| n.kind == NodeKind::File).unwrap();
        // Files must have children = None (not Some([]))
        assert!(file_node.children.is_none());
    }

    #[test]
    fn test_expand_dir_returns_children() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        fs::create_dir(root.join("sub")).unwrap();
        fs::write(root.join("sub").join("a.rs"), "").unwrap();
        fs::write(root.join("sub").join("b.rs"), "").unwrap();

        let children = read_dir_one_level(&root.join("sub")).unwrap();
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_empty_dir_returns_empty_vec() {
        let dir = TempDir::new().unwrap();
        let nodes = read_dir_one_level(dir.path()).unwrap();
        assert!(nodes.is_empty());
    }
}
