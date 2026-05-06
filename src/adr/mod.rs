//! ADR parsing, discovery, and validation.

use regex::Regex;
use std::sync::OnceLock;

pub mod discovery;
pub mod frontmatter;
pub mod manifest;
pub mod validator;

pub use discovery::Discovery;
pub use frontmatter::Frontmatter;
pub use manifest::Manifest;
pub use validator::{Diagnostic, DiagnosticCode, ValidationResult, Validator};

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

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "adr".to_string()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
