//! ADR concept document parsing.

use crate::adr::concept_id;
use crate::adr::frontmatter::Frontmatter;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A parsed ADR concept document.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Path to the concept's Markdown file.
    pub path: PathBuf,

    /// OKF concept id: the bundle-relative path without the `.md` suffix.
    pub concept_id: String,

    /// Parsed YAML frontmatter.
    pub frontmatter: Frontmatter,

    /// Markdown body content.
    pub body: String,

    /// One-based line number where the Markdown body starts.
    pub body_start_line: usize,
}

/// Errors that can occur when parsing an ADR concept document.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// The file does not start with a YAML frontmatter delimiter.
    #[error("concept must start with YAML frontmatter (---)")]
    MissingFrontmatter,

    /// The YAML frontmatter is not properly closed.
    #[error("frontmatter is not closed (missing closing ---)")]
    UnclosedFrontmatter,

    /// The YAML frontmatter contains invalid YAML.
    #[error("invalid YAML in frontmatter: {0}")]
    InvalidYaml(#[from] serde_yaml::Error),

    /// An I/O error occurred while reading the file.
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

impl Manifest {
    /// Parse an ADR concept document sitting inside `bundle_root`.
    pub fn parse(path: &Path, bundle_root: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_content(path, bundle_root, &content)
    }

    /// Return the body of a `## <name>` Markdown section, with surrounding
    /// blank lines trimmed. Matching is case-insensitive and ignores trailing
    /// `#` characters in the heading. Returns `None` if no such section exists.
    pub fn section(&self, name: &str) -> Option<String> {
        let target = name.trim().to_ascii_lowercase();
        let mut lines = self.body.lines();

        let found = lines.by_ref().any(|line| {
            line.strip_prefix("## ")
                .map(|heading| heading.trim().trim_end_matches('#').trim())
                .is_some_and(|heading| heading.eq_ignore_ascii_case(&target))
        });

        if !found {
            return None;
        }

        let body: Vec<&str> = lines.take_while(|line| !line.starts_with("## ")).collect();

        Some(body.join("\n").trim().to_owned())
    }

    /// Parse concept content from a string.
    pub fn parse_content(
        path: &Path,
        bundle_root: &Path,
        content: &str,
    ) -> Result<Self, ManifestError> {
        let (frontmatter_raw, body, body_start_line) = split_content(content)?;
        let frontmatter = if frontmatter_raw.trim().is_empty() {
            Frontmatter::default()
        } else {
            serde_yaml::from_str(&frontmatter_raw)?
        };

        Ok(Self {
            path: path.to_path_buf(),
            concept_id: concept_id(path, bundle_root),
            frontmatter,
            body,
            body_start_line,
        })
    }
}

/// Split a Markdown file into its frontmatter block, body, and the one-based
/// line number on which the body starts.
pub(crate) fn split_content(content: &str) -> Result<(String, String, usize), ManifestError> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let lines: Vec<&str> = content.lines().collect();

    if lines.first() != Some(&"---") {
        return Err(ManifestError::MissingFrontmatter);
    }

    let closing_index = lines
        .iter()
        .skip(1)
        .position(|line| *line == "---")
        .map(|offset| offset + 1)
        .ok_or(ManifestError::UnclosedFrontmatter)?;

    let frontmatter = lines[1..closing_index].join("\n");
    let body = lines[closing_index + 1..].join("\n");
    let body_start_line = closing_index + 2;

    Ok((frontmatter, body, body_start_line))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDLE: &str = "docs/adr";

    #[test]
    fn parses_valid_manifest() {
        let content = "---
type: Architecture Decision Record
title: Basic ADR CLI
description: Navigate ADRs
status: accepted
timestamp: 2026-05-06
---

# Basic ADR CLI

## Status

Accepted
";
        let manifest = Manifest::parse_content(
            Path::new("docs/adr/basic-adr-cli.md"),
            Path::new(BUNDLE),
            content,
        )
        .expect("valid manifest");

        assert_eq!(
            manifest.frontmatter.concept_type.as_deref(),
            Some("Architecture Decision Record")
        );
        assert_eq!(manifest.concept_id, "basic-adr-cli");
        assert!(manifest.body.contains("# Basic ADR CLI"));
    }

    #[test]
    fn nested_concepts_get_a_path_shaped_id() {
        let content = "---\ntype: Architecture Decision Record\n---\n\n# X\n";
        let manifest = Manifest::parse_content(
            Path::new("docs/adr/security/mtls.md"),
            Path::new(BUNDLE),
            content,
        )
        .expect("valid manifest");
        assert_eq!(manifest.concept_id, "security/mtls");
    }

    #[test]
    fn unknown_frontmatter_keys_are_tolerated() {
        let content = "---
type: Architecture Decision Record
title: X
some_producer_extension: 42
---

# X
";
        let manifest =
            Manifest::parse_content(Path::new("docs/adr/x.md"), Path::new(BUNDLE), content)
                .expect("unknown keys must not be rejected");
        assert_eq!(manifest.frontmatter.title.as_deref(), Some("X"));
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let content = "# No frontmatter here";
        let result =
            Manifest::parse_content(Path::new("docs/adr/nope.md"), Path::new(BUNDLE), content);
        assert!(matches!(result, Err(ManifestError::MissingFrontmatter)));
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let content = "---\ntype: Architecture Decision Record\n# no closing marker";
        let result =
            Manifest::parse_content(Path::new("docs/adr/nope.md"), Path::new(BUNDLE), content);
        assert!(matches!(result, Err(ManifestError::UnclosedFrontmatter)));
    }

    #[test]
    fn extracts_named_section() {
        let content = "---
type: Architecture Decision Record
title: X
description: x
status: proposed
timestamp: 2026-05-06
---

# X

## Status

Proposed

## Decision

We will adopt X.

It scales.

## Consequences

Faster.
";
        let manifest =
            Manifest::parse_content(Path::new("docs/adr/x.md"), Path::new(BUNDLE), content)
                .expect("valid manifest");
        assert_eq!(
            manifest.section("decision").as_deref(),
            Some("We will adopt X.\n\nIt scales.")
        );
        assert_eq!(
            manifest.section("DECISION").as_deref(),
            Some("We will adopt X.\n\nIt scales.")
        );
        assert_eq!(manifest.section("missing"), None);
    }
}
