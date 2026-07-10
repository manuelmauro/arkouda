//! ADR parsing, discovery, and validation, over an [OKF][okf] knowledge bundle.
//!
//! An ADR directory is an OKF bundle: a tree of markdown concept documents,
//! each carrying YAML frontmatter. A concept's id is its path within the
//! bundle with the `.md` suffix removed (OKF §2), so `security/mtls.md` has
//! concept id `security/mtls`.
//!
//! [okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md

use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub mod discovery;
pub mod frontmatter;
pub mod index;
pub mod manifest;
pub mod status;
pub mod validator;

pub use frontmatter::{ADR_TYPE, Frontmatter};
pub use manifest::Manifest;
pub use status::AdrStatus;
pub use validator::{Diagnostic, DiagnosticCode, ValidationResult};

/// The OKF version this crate targets.
pub const OKF_VERSION: &str = "0.1";

/// Filenames OKF §3.1 reserves at every level of a bundle. They are never
/// concept documents.
pub const RESERVED_FILENAMES: [&str; 2] = ["index.md", "log.md"];

/// Return true when `path`'s filename is reserved by OKF §3.1.
pub fn is_reserved(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            RESERVED_FILENAMES
                .iter()
                .any(|reserved| name.eq_ignore_ascii_case(reserved))
        })
}

/// The OKF concept id for `path` within `bundle_root`: the bundle-relative
/// path with the `.md` suffix removed, always `/`-separated.
///
/// Falls back to the bare file stem when `path` is not inside `bundle_root`,
/// so a concept id never leaks an absolute path.
pub fn concept_id(path: &Path, bundle_root: &Path) -> String {
    let Ok(relative) = path.strip_prefix(bundle_root) else {
        return file_stem(path);
    };

    let stemmed = relative.with_extension("");
    let segments: Vec<String> = stemmed
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();

    if segments.is_empty() {
        file_stem(path)
    } else {
        segments.join("/")
    }
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Return true when an ADR id is a lowercase slug.
pub fn is_valid_id(id: &str) -> bool {
    static ID_REGEX: OnceLock<Regex> = OnceLock::new();
    let regex = ID_REGEX.get_or_init(|| Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap());
    regex.is_match(id)
}

/// Generate a lowercase slug id from free-form text.
pub fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for character in input.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_was_dash = false;
        } else if !slug.is_empty() && !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.truncate(slug.trim_end_matches('-').len());

    if slug.is_empty() {
        String::from("adr")
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn validates_slug_ids() {
        assert!(is_valid_id("basic-adr-cli"));
        assert!(is_valid_id("adr2"));
        assert!(!is_valid_id("Basic"));
        assert!(!is_valid_id("double--dash"));
        assert!(!is_valid_id("trailing-"));
    }

    #[test]
    fn slugifies_titles() {
        assert_eq!(slugify("Basic ADR CLI"), "basic-adr-cli");
        assert_eq!(slugify("  Hello, world!  "), "hello-world");
        assert_eq!(slugify("---"), "adr");
    }

    #[test]
    fn reserved_filenames_are_not_concepts() {
        assert!(is_reserved(Path::new("docs/adr/index.md")));
        assert!(is_reserved(Path::new("docs/adr/log.md")));
        assert!(is_reserved(Path::new("docs/adr/INDEX.md")));
        assert!(!is_reserved(Path::new("docs/adr/indexing.md")));
        assert!(!is_reserved(Path::new("docs/adr/use-postgres.md")));
    }

    #[test]
    fn concept_id_is_the_bundle_relative_path_without_suffix() {
        let root = PathBuf::from("docs/adr");
        assert_eq!(
            concept_id(Path::new("docs/adr/use-postgres.md"), &root),
            "use-postgres"
        );
        assert_eq!(
            concept_id(Path::new("docs/adr/security/mtls.md"), &root),
            "security/mtls"
        );
    }

    #[test]
    fn concept_id_falls_back_to_the_stem_outside_the_bundle() {
        assert_eq!(
            concept_id(Path::new("/elsewhere/x.md"), Path::new("docs/adr")),
            "x"
        );
    }
}
