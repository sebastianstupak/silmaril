use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::file_explorer::tree::GitStatus;

/// Parse `git status --porcelain -z` output.
/// Returns map of relative_path → status string (internal helper, testable without Tauri).
pub fn parse_porcelain(output: &str, _root: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for entry in output.split('\0') {
        if entry.len() < 4 {
            continue;
        }
        let xy = &entry[..2];
        let path = entry[3..].trim().to_string();
        if path.is_empty() {
            continue;
        }
        let status = match xy {
            s if s == "??" => "untracked",
            s if s.starts_with(' ') && s.ends_with('D') => "deleted",
            s if s.starts_with(' ') && s.ends_with('M') => "modified",
            s if s.starts_with('M') || s.starts_with('A') => "staged",
            s if s.ends_with('D') => "deleted",
            _ => continue,
        };
        map.insert(path, status.to_string());
    }
    map
}

fn string_to_git_status(s: &str) -> Option<GitStatus> {
    match s {
        "modified" => Some(GitStatus::Modified),
        "untracked" => Some(GitStatus::Untracked),
        "deleted" => Some(GitStatus::Deleted),
        "staged" => Some(GitStatus::Staged),
        _ => None,
    }
}

#[tauri::command]
pub fn get_git_status(root: String) -> HashMap<String, GitStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain", "-z"])
        .current_dir(Path::new(&root))
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout).into_owned();
            parse_porcelain(&text, &root)
                .into_iter()
                .filter_map(|(k, v)| string_to_git_status(&v).map(|s| (k, s)))
                .collect()
        }
        _ => HashMap::new(), // silent failure — not a git repo, git not installed, etc.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_modified() {
        let output = " M src/main.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("src/main.rs").map(|s| s.as_str()), Some("modified"));
    }

    #[test]
    fn test_parse_porcelain_untracked() {
        let output = "?? new_file.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("new_file.rs").map(|s| s.as_str()), Some("untracked"));
    }

    #[test]
    fn test_parse_porcelain_staged() {
        let output = "M  src/lib.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("src/lib.rs").map(|s| s.as_str()), Some("staged"));
    }

    #[test]
    fn test_parse_porcelain_deleted() {
        let output = " D old.rs\0";
        let result = parse_porcelain(output, "/project");
        assert_eq!(result.get("old.rs").map(|s| s.as_str()), Some("deleted"));
    }

    #[test]
    fn test_parse_porcelain_empty() {
        let result = parse_porcelain("", "/project");
        assert!(result.is_empty());
    }
}
