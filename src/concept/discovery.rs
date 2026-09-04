//! Bundle and concept discovery.
//!
//! Two walks. [`find_bundles`] answers "what bundles does this repository
//! have", which used to be a `dirs` line in `.arkoudarc.toml` (ADR
//! `discover-bundles`); [`find_concepts`] and [`find_reserved`] answer "what
//! is in this one", which they always did.

use crate::concept::is_reserved;
use std::path::{Path, PathBuf};

/// Directories a repository-wide walk must not enter.
///
/// Needed only since the walk started at a repository rather than at a bundle.
/// It is as much about correctness as speed: a vendored dependency carrying its
/// own OKF documents must not have them adopted as this project's concepts.
const IGNORED: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    "out",
    ".next",
    ".venv",
    "venv",
    "__pycache__",
    "coverage",
];

/// True when a directory is one the walk skips: ignored by name, or hidden.
fn is_skipped_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.') || IGNORED.contains(&name))
}

/// True when this file is a concept: a non-reserved Markdown file whose
/// frontmatter parses and declares a `type`.
///
/// Deliberately cheaper than a full parse — the walk asks this of every
/// Markdown file in the repository, and all it needs to know is whether the
/// directory holding it is a bundle.
fn declares_type(path: &Path) -> bool {
    if !is_markdown(path) || is_reserved(path) {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    // `split_content` is the same splitter `Manifest` uses, so the walk and the
    // parser agree about what a frontmatter block is. Whether the YAML inside
    // is *valid* is deliberately not asked: a document with broken frontmatter
    // is still a concept and still makes its directory a bundle, so that
    // `check` can report the breakage instead of the file silently vanishing.
    crate::concept::manifest::split_content(&text)
        .is_ok_and(|(frontmatter, _, _)| has_type_key(&frontmatter))
}

/// A `type:` key with a non-empty value, at the top level of the block.
fn has_type_key(frontmatter: &str) -> bool {
    frontmatter.lines().any(|line| {
        line.strip_prefix("type:")
            .is_some_and(|value| !value.trim().is_empty())
    })
}

/// Every bundle in the tree below `start`, in path order.
///
/// A bundle is the **topmost** directory that directly contains a concept:
/// descent stops there, so everything deeper belongs to that bundle and a
/// nested concept keeps its path in its id. Choosing the topmost such
/// directory rather than every such directory is what preserves `security/mtls`
/// as an id instead of splitting it into its own bundle — see ADR
/// `discover-bundles`.
///
/// When `start` is itself a Markdown file, its parent is the bundle.
pub fn find_bundles(start: &Path) -> std::io::Result<Vec<PathBuf>> {
    if start.is_file() {
        return Ok(vec![start.parent().unwrap_or(Path::new(".")).to_path_buf()]);
    }

    let mut bundles = Vec::new();
    collect_bundles(start, &mut bundles)?;
    bundles.sort();
    Ok(bundles)
}

fn collect_bundles(dir: &Path, found: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let mut subdirectories = Vec::new();
    let mut holds_concept = false;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            if !is_skipped_dir(&path) {
                subdirectories.push(path);
            }
        } else if path.is_file() && !holds_concept && declares_type(&path) {
            holds_concept = true;
        }
    }

    if holds_concept {
        found.push(dir.to_path_buf());
        return Ok(());
    }

    subdirectories.sort();
    for subdirectory in subdirectories {
        collect_bundles(&subdirectory, found)?;
    }
    Ok(())
}

/// A reserved OKF file found in a bundle (OKF §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedKind {
    /// `index.md` — directory listing (OKF §8).
    Index,
    /// `log.md` — update history (OKF §9).
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
        if path.is_dir() {
            if !is_skipped_dir(&path) {
                subdirectories.push(path);
            }
        } else if path.is_file() && !is_hidden(&path) {
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

    /// A minimal concept: enough frontmatter to declare a type.
    const CONCEPT: &str = "---\ntype: Architecture Decision Record\ntitle: T\n---\n\n# T\n";

    fn relative(root: &Path, bundles: Vec<PathBuf>) -> Vec<String> {
        bundles
            .iter()
            .map(|b| {
                b.strip_prefix(root)
                    .unwrap_or(b)
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn a_bundle_is_the_topmost_directory_holding_a_concept() {
        let root = temp_bundle("topmost");
        write(&root.join("docs/adr/use-postgres.md"), CONCEPT);
        write(&root.join("docs/adr/security/mtls.md"), CONCEPT);

        // `docs/adr/security` must NOT be its own bundle, or `security/mtls`
        // stops being an id and nested references break.
        assert_eq!(
            relative(&root, find_bundles(&root).expect("walk")),
            vec!["docs/adr"]
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn sibling_bundles_are_all_found() {
        let root = temp_bundle("siblings");
        write(&root.join("docs/adr/a.md"), CONCEPT);
        write(&root.join("docs/prd/b.md"), CONCEPT);
        write(&root.join("knowledge/c.md"), CONCEPT);

        assert_eq!(
            relative(&root, find_bundles(&root).expect("walk")),
            vec!["docs/adr", "docs/prd", "knowledge"]
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn concepts_only_in_a_subdirectory_make_that_subdirectory_the_bundle() {
        // The one layout whose ids the rule changes, recorded in ADR
        // `discover-bundles` rather than left to be discovered by surprise.
        let root = temp_bundle("subdir-only");
        write(&root.join("docs/adr/security/mtls.md"), CONCEPT);

        assert_eq!(
            relative(&root, find_bundles(&root).expect("walk")),
            vec!["docs/adr/security"]
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn ignored_and_hidden_directories_are_never_bundles() {
        let root = temp_bundle("ignored");
        write(&root.join("docs/adr/a.md"), CONCEPT);
        write(&root.join("node_modules/pkg/docs/adr/vendored.md"), CONCEPT);
        write(&root.join("target/doc/generated.md"), CONCEPT);
        write(&root.join(".git/hooks/notes.md"), CONCEPT);
        write(&root.join(".hidden/secret.md"), CONCEPT);

        // A vendored dependency's own OKF documents are not this project's
        // concepts.
        assert_eq!(
            relative(&root, find_bundles(&root).expect("walk")),
            vec!["docs/adr"]
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn markdown_without_a_type_does_not_make_a_bundle() {
        let root = temp_bundle("plain-markdown");
        write(&root.join("README.md"), "# Just prose\n");
        write(&root.join("notes/thoughts.md"), "---\ntitle: no type\n---\n");
        write(&root.join("docs/adr/a.md"), CONCEPT);

        assert_eq!(
            relative(&root, find_bundles(&root).expect("walk")),
            vec!["docs/adr"]
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_listing_is_not_a_bundle() {
        let root = temp_bundle("reserved-only");
        write(&root.join("docs/adr/index.md"), CONCEPT);
        write(&root.join("docs/adr/log.md"), CONCEPT);

        // Reserved names never count, however OKF-looking their content.
        assert!(find_bundles(&root).expect("walk").is_empty());
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_concept_with_broken_yaml_still_makes_a_bundle() {
        // Otherwise the file vanishes from the walk and `check` never gets to
        // report the breakage.
        let root = temp_bundle("broken-yaml");
        write(
            &root.join("docs/adr/broken.md"),
            "---\ntype: Architecture Decision Record\ntitle: [unclosed\n---\n\n# T\n",
        );

        assert_eq!(
            relative(&root, find_bundles(&root).expect("walk")),
            vec!["docs/adr"]
        );
        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_markdown_file_as_the_start_is_its_own_bundles_parent() {
        let root = temp_bundle("single-file");
        let file = root.join("docs/adr/a.md");
        write(&file, CONCEPT);

        assert_eq!(find_bundles(&file).expect("walk"), vec![root.join("docs/adr")]);
        std::fs::remove_dir_all(&root).expect("cleanup");
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
