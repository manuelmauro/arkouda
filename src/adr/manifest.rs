//! ADR manifest parsing.

use crate::adr::frontmatter::Frontmatter;
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// A parsed ADR Markdown file.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Path to the ADR Markdown file.
    pub path: PathBuf,

    /// Parsed YAML frontmatter.
    pub frontmatter: Frontmatter,

    /// Raw frontmatter YAML string.
    pub frontmatter_raw: String,

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
    #[error("IO error reading {path}: {source}")]
    Io {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

impl Manifest {
    /// Parse an ADR Markdown file.
    pub fn parse(path: PathBuf) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(&path).map_err(|source| ManifestError::Io {
            path: path.clone(),
            source,
        })?;
        Self::parse_content(path, &content)
    }

    /// Parse ADR content from a string.
    pub fn parse_content(path: PathBuf, content: &str) -> Result<Self, ManifestError> {
        let (frontmatter_raw, body, body_start_line) = Self::split_content(content)?;
        let frontmatter = if frontmatter_raw.trim().is_empty() {
            Frontmatter::default()
        } else {
            serde_yaml::from_str(&frontmatter_raw)?
        };

        Ok(Self {
            path,
            frontmatter,
            frontmatter_raw,
            body,
            body_start_line,
        })
    }

    fn split_content(content: &str) -> Result<(String, String, usize), ManifestError> {
        let content = content.strip_prefix('\u{feff}').unwrap_or(content);
        let lines: Vec<&str> = content.lines().collect();

        if lines.first().copied() != Some("---") {
            return Err(ManifestError::MissingFrontmatter);
        }

        let closing_index = lines
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, line)| (*line == "---").then_some(index))
            .ok_or(ManifestError::UnclosedFrontmatter)?;

        let frontmatter = lines[1..closing_index].join("\n");
        let body = if closing_index + 1 < lines.len() {
            lines[closing_index + 1..].join("\n")
        } else {
            String::new()
        };
        let body_start_line = closing_index + 2;

        Ok((frontmatter, body, body_start_line))
    }
}

impl fmt::Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "---\n{}\n---\n\n{}",
            self.frontmatter_raw.trim(),
            self.body
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_manifest() {
        let content = r#"---
id: basic-adr-cli
title: Basic ADR CLI
abstract: Navigate ADRs
status: proposed
date: 2026-05-06
---

# Basic ADR CLI

## Status

Proposed
"#;
        let manifest = Manifest::parse_content(PathBuf::from("docs/adr/basic-adr-cli.md"), content)
            .expect("valid manifest");

        assert_eq!(manifest.frontmatter.id.as_deref(), Some("basic-adr-cli"));
        assert!(manifest.body.contains("# Basic ADR CLI"));
    }

    #[test]
    fn rejects_missing_frontmatter() {
        let content = "# No frontmatter here";
        let result = Manifest::parse_content(PathBuf::from("docs/adr/nope.md"), content);
        assert!(matches!(result, Err(ManifestError::MissingFrontmatter)));
    }

    #[test]
    fn rejects_unclosed_frontmatter() {
        let content = "---\nid: nope\n# no closing marker";
        let result = Manifest::parse_content(PathBuf::from("docs/adr/nope.md"), content);
        assert!(matches!(result, Err(ManifestError::UnclosedFrontmatter)));
    }
}
