//! ADR discovery utilities.

use std::path::{Path, PathBuf};

/// Find ADR Markdown files.
///
/// If `root` is a Markdown file, it is returned directly. Otherwise, direct
/// Markdown children of the directory are returned in path order.
pub fn find_adrs(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(is_markdown(root)
            .then(|| root.to_path_buf())
            .into_iter()
            .collect());
    }

    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_markdown(&path) {
            paths.push(path);
        }
    }

    paths.sort();
    Ok(paths)
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}
