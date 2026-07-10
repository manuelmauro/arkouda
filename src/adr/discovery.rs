//! Concept discovery over an OKF bundle.

use crate::adr::is_reserved;
use std::path::{Path, PathBuf};

/// A reserved OKF file found in a bundle (OKF §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedKind {
    /// `index.md` — directory listing (OKF §6).
    Index,
    /// `log.md` — update history (OKF §7).
    Log,
}

/// Find concept documents in a bundle.
///
/// If `root` is a Markdown file, it is returned directly. Otherwise the
/// bundle is walked recursively — OKF bundles nest concepts in
/// subdirectories — skipping reserved filenames and hidden entries. Results
/// are returned in path order.
pub fn find_concepts(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    if root.is_file() {
        let is_concept = is_markdown(root) && !is_reserved(root);
        return Ok(is_concept.then(|| root.to_path_buf()).into_iter().collect());
    }

    let mut paths = Vec::new();
    walk(root, &mut |path| {
        if is_markdown(path) && !is_reserved(path) {
            paths.push(path.to_path_buf());
        }
    })?;
    paths.sort();
    Ok(paths)
}

/// Find the reserved files (`index.md`, `log.md`) anywhere in a bundle.
pub fn find_reserved(root: &Path) -> std::io::Result<Vec<(PathBuf, ReservedKind)>> {
    if root.is_file() {
        return Ok(reserved_kind(root)
            .map(|kind| (root.to_path_buf(), kind))
            .into_iter()
            .collect());
    }

    let mut found = Vec::new();
    walk(root, &mut |path| {
        if let Some(kind) = reserved_kind(path) {
            found.push((path.to_path_buf(), kind));
        }
    })?;
    found.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(found)
}

fn reserved_kind(path: &Path) -> Option<ReservedKind> {
    let name = path.file_name()?.to_str()?;
    if name.eq_ignore_ascii_case("index.md") {
        Some(ReservedKind::Index)
    } else if name.eq_ignore_ascii_case("log.md") {
        Some(ReservedKind::Log)
    } else {
        None
    }
}

/// Walk `dir` recursively, invoking `visit` for every file. Hidden entries
/// (those whose name starts with `.`) are skipped so a bundle can sit
/// alongside `.git`, `.github`, and friends.
fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }

    let mut subdirectories = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if is_hidden(&path) {
            continue;
        }
        if path.is_dir() {
            subdirectories.push(path);
        } else if path.is_file() {
            visit(&path);
        }
    }

    subdirectories.sort();
    for subdirectory in subdirectories {
        walk(&subdirectory, visit)?;
    }
    Ok(())
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("create dirs");
        std::fs::write(path, content).expect("write file");
    }

    fn temp_bundle(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("arkouda-discovery-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create root");
        root
    }

    #[test]
    fn walks_nested_concepts_and_skips_reserved_and_hidden() {
        let root = temp_bundle("nested");
        write(&root.join("top.md"), "top");
        write(&root.join("index.md"), "index");
        write(&root.join("log.md"), "log");
        write(&root.join("notes.txt"), "not markdown");
        write(&root.join("security/mtls.md"), "nested");
        write(&root.join("security/index.md"), "nested index");
        write(&root.join(".hidden/secret.md"), "hidden");

        let concepts = find_concepts(&root).expect("walk");
        assert_eq!(
            concepts,
            vec![root.join("security/mtls.md"), root.join("top.md")]
        );

        let reserved = find_reserved(&root).expect("walk");
        assert_eq!(
            reserved,
            vec![
                (root.join("index.md"), ReservedKind::Index),
                (root.join("log.md"), ReservedKind::Log),
                (root.join("security/index.md"), ReservedKind::Index),
            ]
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn missing_root_yields_nothing() {
        let root = std::env::temp_dir().join("arkouda-discovery-absent");
        let _ = std::fs::remove_dir_all(&root);
        assert!(find_concepts(&root).expect("walk").is_empty());
        assert!(find_reserved(&root).expect("walk").is_empty());
    }

    #[test]
    fn a_reserved_file_named_directly_is_not_a_concept() {
        let root = temp_bundle("direct");
        let index = root.join("index.md");
        write(&index, "index");
        assert!(find_concepts(&index).expect("walk").is_empty());
        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
