//! ADR manifest parsing.

use crate::adr::frontmatter::Frontmatter;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// A parsed ADR Markdown file.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Path to the ADR Markdown file.
    pub path: PathBuf,

    /// Parsed YAML frontmatter.
    pub frontmatter: Frontmatter,

    /// Markdown body content.
    pub body: String,

    /// One-based line number where the Markdown body starts.
    pub body_start_line: usize,
}

/// Errors that can occur when parsing an ADR Markdown file.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// The file does not start with a YAML frontmatter delimiter.
    #[error("ADR must start with YAML frontmatter (---)")]
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
    /// Parse an ADR Markdown file.
    pub fn parse(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_content(path, &content)
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

    /// Parse ADR content from a string.
    pub fn parse_content(path: &Path, content: &str) -> Result<Self, ManifestError> {
        let (frontmatter_raw, body, body_start_line) = split_content(content)?;
        let frontmatter = if frontmatter_raw.trim().is_empty() {
            Frontmatter::default()
        } else {
            serde_yaml::from_str(&frontmatter_raw)?
        };

        Ok(Self {
            path: path.to_path_buf(),
            frontmatter,
            body,
            body_start_line,
        })
    }
}

fn split_content(content: &str) -> Result<(String, String, usize), ManifestError> {
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

    #[test]
    fn parses_valid_manifest() {
        let content = "---
id: basic-adr-cli
title: Basic ADR CLI
abstract: Navigate ADRs
status: proposed
date: 2026-05-06
---

# Basic ADR CLI

## Status

Proposed
";
        let manifest = Manifest::parse_content(Path::new("docs/adr/basic-adr-cli.md"), content)
            .expect("valid manifest");

        assert_eq!(manifest.frontmatter.id.as_deref(), Some("basic-adr-cli"));
        assert!(manifest.body.contains("# Basic ADR CLI"));
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let content = "# No frontmatter here";
        let result = Manifest::parse_content(Path::new("docs/adr/nope.md"), content);
        assert!(matches!(result, Err(ManifestError::MissingFrontmatter)));
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let content = "---\nid: nope\n# no closing marker";
        let result = Manifest::parse_content(Path::new("docs/adr/nope.md"), content);
        assert!(matches!(result, Err(ManifestError::UnclosedFrontmatter)));
    }

    #[test]
    fn extracts_named_section() {
        let content = "---
id: x
title: X
abstract: x
status: proposed
date: 2026-05-06
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
        let manifest = Manifest::parse_content(Path::new("x.md"), content).expect("valid manifest");
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
