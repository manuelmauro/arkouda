//! ADR validation.
//!
//! Two layers stack here. OKF conformance (§9) is the floor: every concept
//! parses, and every concept declares a non-empty `type`. On top of that
//! arkouda enforces the ADR contract — a known `type`, a controlled `status`,
//! an ISO 8601 `timestamp`, and Michael Nygard's body sections — because a
//! bundle of ADRs is more useful when its concepts are uniform.
//!
//! OKF's permissive-consumption rule (§9) shapes what is a warning rather than
//! an error: an unknown declared OKF version and a stale `index.md` never
//! fail a bundle.

use crate::adr::manifest::{ManifestError, split_content};
use crate::adr::{ADR_TYPE, AdrStatus, Manifest, OKF_VERSION, is_valid_id};
use chrono::{DateTime, NaiveDate, NaiveDateTime};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Result of validating a concept or reserved file.
#[derive(Debug, Default, Clone)]
pub struct ValidationResult {
    /// Validation errors. Any error fails the bundle.
    pub errors: Vec<Diagnostic>,
    /// Validation warnings. Never fail the bundle.
    pub warnings: Vec<Diagnostic>,
}

impl ValidationResult {
    /// Merge another result into this one.
    pub fn merge(&mut self, other: Self) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// A validation diagnostic. Severity is determined by which [`ValidationResult`]
/// list it lands in, not by the diagnostic itself.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// One-based line number, when available.
    pub line: Option<usize>,
    /// Human-readable message.
    pub message: String,
    /// Diagnostic code.
    pub code: DiagnosticCode,
    /// Optional hint for fixing the issue.
    pub fix_hint: Option<String>,
}

impl Diagnostic {
    /// Create a diagnostic.
    pub fn new(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
            code,
            fix_hint: None,
        }
    }

    /// Set a line number.
    #[must_use]
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set a fix hint.
    #[must_use]
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }
}

/// Diagnostic codes for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// Concept could not be parsed.
    E000,
    /// Required frontmatter field is missing.
    E001,
    /// Required frontmatter field is empty.
    E002,
    /// Status is not in the controlled status list.
    E003,
    /// Concept id is not a lowercase slug.
    E004,
    /// Frontmatter `type` is not the ADR concept type.
    E005,
    /// Timestamp is not a valid ISO 8601 date or datetime.
    E006,
    /// Top-level Markdown heading is missing.
    E007,
    /// Top-level Markdown heading does not match title.
    E008,
    /// Required Markdown section is missing.
    E009,
    /// Concept id is duplicated.
    E010,
    /// `index.md` carries frontmatter where OKF does not permit it.
    E011,
    /// `log.md` heading is not an ISO 8601 `YYYY-MM-DD` date.
    E012,
    /// Bundle declares an OKF version arkouda does not implement. Warning.
    E013,
    /// `index.md` does not match the concepts in the bundle. Warning.
    E014,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Validate a collection of parsed ADR concepts.
pub fn validate_collection(manifests: &[Manifest]) -> Vec<ValidationResult> {
    let mut results: Vec<ValidationResult> = manifests.iter().map(validate).collect();
    check_duplicate_ids(manifests, &mut results);
    results
}

/// Validate one parsed ADR concept.
pub fn validate(manifest: &Manifest) -> ValidationResult {
    let mut result = ValidationResult::default();

    check_required_field(
        &mut result,
        "type",
        manifest.frontmatter.concept_type.as_deref(),
        &format!("Add `type: {ADR_TYPE}` to the YAML frontmatter."),
    );
    check_required_field(
        &mut result,
        "title",
        manifest.frontmatter.title.as_deref(),
        "Add a human-readable `title` to the YAML frontmatter.",
    );
    check_required_field(
        &mut result,
        "description",
        manifest.frontmatter.description.as_deref(),
        "Add a `description` summarizing the decision itself (what was decided, \
         not just the topic) in one sentence.",
    );
    check_required_field(
        &mut result,
        "status",
        manifest.frontmatter.status.as_deref(),
        "Add `status: proposed` or another valid status.",
    );
    check_required_field(
        &mut result,
        "timestamp",
        manifest.frontmatter.timestamp.as_deref(),
        "Add `timestamp: YYYY-MM-DD` to the YAML frontmatter.",
    );

    if let Some(concept_type) = non_empty(manifest.frontmatter.concept_type.as_deref())
        && concept_type != ADR_TYPE
    {
        result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E005,
                format!("frontmatter `type` is `{concept_type}`, not `{ADR_TYPE}`"),
            )
            .with_hint(format!(
                "arkouda manages a bundle of ADRs. Set `type: {ADR_TYPE}`, or move \
                 this concept out of the ADR bundle."
            )),
        );
    }

    check_concept_id(manifest, &mut result);

    if let Some(status) = non_empty(manifest.frontmatter.status.as_deref())
        && status.parse::<AdrStatus>().is_err()
    {
        let valid = AdrStatus::ALL
            .iter()
            .map(AdrStatus::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        result.errors.push(
            Diagnostic::new(DiagnosticCode::E003, format!("invalid status `{status}`"))
                .with_hint(format!("Use one of: {valid}.")),
        );
    }

    if let Some(timestamp) = non_empty(manifest.frontmatter.timestamp.as_deref())
        && !is_iso8601(timestamp)
    {
        result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E006,
                format!("timestamp `{timestamp}` is not a valid ISO 8601 date or datetime"),
            )
            .with_hint("Use `2026-05-06` or `2026-05-06T14:30:00Z`."),
        );
    }

    check_title_heading(manifest, &mut result);
    check_required_sections(manifest, &mut result);

    result
}

/// Validate an `index.md` (OKF §6, §11). Frontmatter is permitted only in a
/// bundle-root index, and only to declare `okf_version`.
pub fn validate_index(content: &str, is_bundle_root: bool) -> ValidationResult {
    let mut result = ValidationResult::default();

    let frontmatter = match split_content(content) {
        // No frontmatter at all is always valid for an index.
        Err(ManifestError::MissingFrontmatter) => return result,
        Err(error) => {
            result.errors.push(
                Diagnostic::new(DiagnosticCode::E011, error.to_string())
                    .with_hint("Close the frontmatter block with `---`, or remove it entirely."),
            );
            return result;
        }
        Ok((frontmatter, _, _)) => frontmatter,
    };

    if !is_bundle_root {
        result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E011,
                "only a bundle-root `index.md` may carry frontmatter",
            )
            .with_hint("Remove the frontmatter block from this index."),
        );
        return result;
    }

    let parsed: HashMap<String, serde_yaml::Value> = match serde_yaml::from_str(&frontmatter) {
        Ok(parsed) => parsed,
        Err(error) => {
            result.errors.push(Diagnostic::new(
                DiagnosticCode::E011,
                format!("invalid YAML in index frontmatter: {error}"),
            ));
            return result;
        }
    };

    for key in parsed.keys() {
        if key != "okf_version" {
            result.errors.push(
                Diagnostic::new(
                    DiagnosticCode::E011,
                    format!("`{key}` is not permitted in an `index.md` frontmatter block"),
                )
                .with_hint("A bundle-root index may declare `okf_version` and nothing else."),
            );
        }
    }

    if let Some(declared) = parsed.get("okf_version") {
        let declared = declared.as_str().map(str::to_owned).unwrap_or_else(|| {
            serde_yaml::to_string(declared)
                .unwrap_or_default()
                .trim()
                .to_owned()
        });
        if declared != OKF_VERSION {
            result.warnings.push(
                Diagnostic::new(
                    DiagnosticCode::E013,
                    format!(
                        "bundle declares OKF version `{declared}`; arkouda implements {OKF_VERSION}"
                    ),
                )
                .with_hint("Consumption continues on a best-effort basis (OKF §11)."),
            );
        }
    }

    result
}

/// Validate a `log.md` (OKF §7). Every `##` heading must be an ISO 8601 date.
pub fn validate_log(content: &str) -> ValidationResult {
    let mut result = ValidationResult::default();

    for (line_index, line) in content.lines().enumerate() {
        let Some(heading) = line.strip_prefix("## ") else {
            continue;
        };
        let heading = heading.trim().trim_end_matches('#').trim();
        if NaiveDate::parse_from_str(heading, "%Y-%m-%d").is_err() {
            result.errors.push(
                Diagnostic::new(
                    DiagnosticCode::E012,
                    format!("log heading `{heading}` is not an ISO 8601 `YYYY-MM-DD` date"),
                )
                .with_line(line_index + 1)
                .with_hint("Group log entries under `## 2026-05-06` date headings."),
            );
        }
    }

    result
}

/// Compare an existing bundle-root `index.md` against what arkouda would
/// generate. A stale index is a warning: OKF §9 forbids rejecting a bundle
/// over its index, and a missing one is always fine.
pub fn check_index_freshness(existing: &str, rendered: &str) -> Option<Diagnostic> {
    (existing.trim_end() != rendered.trim_end()).then(|| {
        Diagnostic::new(
            DiagnosticCode::E014,
            "`index.md` does not match the concepts in this bundle",
        )
        .with_hint("Run `arkouda index` to regenerate it.")
    })
}

/// Accept an ISO 8601 calendar date, an offset datetime, or a local datetime.
fn is_iso8601(value: &str) -> bool {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
        || DateTime::parse_from_rfc3339(value).is_ok()
        || NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").is_ok()
}

/// A concept id is a `/`-separated path; every segment must be a slug so that
/// ids stay stable, greppable, and URL-safe.
fn check_concept_id(manifest: &Manifest, result: &mut ValidationResult) {
    let id = &manifest.concept_id;
    if id.split('/').all(is_valid_id) {
        return;
    }

    result.errors.push(
        Diagnostic::new(
            DiagnosticCode::E004,
            format!("concept id `{id}` must be a lowercase slug"),
        )
        .with_hint(
            "A concept id is its path in the bundle without `.md`. Rename the file (and any \
             parent directories) to use lowercase letters, numbers, and single hyphens.",
        ),
    );
}

fn check_duplicate_ids(manifests: &[Manifest], results: &mut [ValidationResult]) {
    let mut id_to_indexes: HashMap<&str, Vec<usize>> = HashMap::new();

    for (index, manifest) in manifests.iter().enumerate() {
        id_to_indexes
            .entry(manifest.concept_id.as_str())
            .or_default()
            .push(index);
    }

    for (id, indexes) in id_to_indexes {
        if indexes.len() <= 1 {
            continue;
        }

        let paths = indexes
            .iter()
            .map(|index| manifests[*index].path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");

        for index in indexes {
            results[index].errors.push(
                Diagnostic::new(
                    DiagnosticCode::E010,
                    format!("concept id `{id}` is duplicated"),
                )
                .with_hint(format!("Use unique ids. Also found in: {paths}")),
            );
        }
    }
}

fn check_required_field(
    result: &mut ValidationResult,
    field: &str,
    value: Option<&str>,
    hint: &str,
) {
    match value {
        None => result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E001,
                format!("missing required frontmatter field `{field}`"),
            )
            .with_hint(hint),
        ),
        Some(value) if value.trim().is_empty() => result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E002,
                format!("frontmatter field `{field}` must not be empty"),
            )
            .with_hint(hint),
        ),
        Some(_) => {}
    }
}

fn check_title_heading(manifest: &Manifest, result: &mut ValidationResult) {
    let Some(title) = non_empty(manifest.frontmatter.title.as_deref()) else {
        return;
    };

    let h1 = manifest
        .body
        .lines()
        .enumerate()
        .find_map(|(line_index, line)| {
            line.strip_prefix("# ")
                .map(|heading| (line_index, heading.trim()))
        });

    let Some((line_index, heading)) = h1 else {
        result.errors.push(
            Diagnostic::new(DiagnosticCode::E007, "missing top-level Markdown heading")
                .with_hint(format!("Add `# {title}` after the frontmatter.")),
        );
        return;
    };

    if heading != title {
        result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E008,
                format!("top-level heading `{heading}` does not match title `{title}`"),
            )
            .with_line(manifest.body_start_line + line_index)
            .with_hint(format!("Change the heading to `# {title}`.")),
        );
    }
}

fn check_required_sections(manifest: &Manifest, result: &mut ValidationResult) {
    let sections: HashSet<String> = manifest
        .body
        .lines()
        .filter_map(|line| {
            line.strip_prefix("## ")
                .map(str::trim)
                .map(|section| section.trim_end_matches('#').trim().to_ascii_lowercase())
        })
        .collect();

    for required in ["status", "context", "decision", "consequences"] {
        if !sections.contains(required) {
            let cased = title_case(required);
            result.errors.push(
                Diagnostic::new(
                    DiagnosticCode::E009,
                    format!("missing required Markdown section `## {cased}`"),
                )
                .with_hint(format!("Add a `## {cased}` section to the ADR body.")),
            );
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::Manifest;
    use std::path::Path;

    const BUNDLE: &str = "docs/adr";

    fn parse(path: &str, content: &str) -> Manifest {
        Manifest::parse_content(Path::new(path), Path::new(BUNDLE), content)
            .expect("valid manifest")
    }

    fn good_adr() -> String {
        "---
type: Architecture Decision Record
title: Basic ADR CLI
description: Navigate ADRs.
status: proposed
timestamp: 2026-05-06
---

# Basic ADR CLI

## Status

Proposed

## Context

Context.

## Decision

Decision.

## Consequences

Consequences.
"
        .to_owned()
    }

    fn codes(result: &ValidationResult) -> Vec<DiagnosticCode> {
        result.errors.iter().map(|d| d.code).collect()
    }

    #[test]
    fn validates_a_good_adr() {
        let manifest = parse("docs/adr/basic-adr-cli.md", &good_adr());
        let result = validate(&manifest);
        assert!(result.errors.is_empty(), "{:#?}", result.errors);
    }

    #[test]
    fn requires_the_okf_type_field() {
        let content = good_adr().replace("type: Architecture Decision Record\n", "");
        let manifest = parse("docs/adr/basic-adr-cli.md", &content);
        assert!(codes(&validate(&manifest)).contains(&DiagnosticCode::E001));
    }

    #[test]
    fn rejects_a_foreign_concept_type() {
        let content = good_adr().replace("Architecture Decision Record", "BigQuery Table");
        let manifest = parse("docs/adr/basic-adr-cli.md", &content);
        assert!(codes(&validate(&manifest)).contains(&DiagnosticCode::E005));
    }

    #[test]
    fn accepts_iso_dates_and_datetimes() {
        assert!(is_iso8601("2026-05-06"));
        assert!(is_iso8601("2026-05-06T14:30:00Z"));
        assert!(is_iso8601("2026-05-06T14:30:00+02:00"));
        assert!(is_iso8601("2026-05-06T14:30:00"));
        assert!(!is_iso8601("06/05/2026"));
        assert!(!is_iso8601("2026-13-01"));
    }

    #[test]
    fn rejects_a_non_iso_timestamp() {
        let content = good_adr().replace("timestamp: 2026-05-06", "timestamp: 06/05/2026");
        let manifest = parse("docs/adr/basic-adr-cli.md", &content);
        assert!(codes(&validate(&manifest)).contains(&DiagnosticCode::E006));
    }

    #[test]
    fn rejects_a_non_slug_concept_id() {
        let manifest = parse("docs/adr/Basic_ADR.md", &good_adr());
        assert!(codes(&validate(&manifest)).contains(&DiagnosticCode::E004));
    }

    #[test]
    fn accepts_a_nested_slug_concept_id() {
        let manifest = parse("docs/adr/security/basic-adr-cli.md", &good_adr());
        assert!(!codes(&validate(&manifest)).contains(&DiagnosticCode::E004));
    }

    #[test]
    fn flags_duplicate_concept_ids_across_bundles() {
        let left = parse("docs/adr/dup.md", &good_adr());
        let mut right = parse("docs/adr/dup.md", &good_adr());
        right.path = Path::new("services/billing/adr/dup.md").to_path_buf();

        let results = validate_collection(&[left, right]);
        assert!(codes(&results[0]).contains(&DiagnosticCode::E010));
        assert!(codes(&results[1]).contains(&DiagnosticCode::E010));
    }

    #[test]
    fn an_index_without_frontmatter_is_valid_anywhere() {
        let content = "# Accepted\n\n* [X](x.md) - Y\n";
        assert!(validate_index(content, true).errors.is_empty());
        assert!(validate_index(content, false).errors.is_empty());
    }

    #[test]
    fn only_a_root_index_may_declare_the_okf_version() {
        let content = "---\nokf_version: \"0.1\"\n---\n\n# Accepted\n";
        assert!(validate_index(content, true).errors.is_empty());

        let nested = validate_index(content, false);
        assert_eq!(codes(&nested), vec![DiagnosticCode::E011]);
    }

    #[test]
    fn a_root_index_may_not_carry_other_keys() {
        let content = "---\ntitle: My decisions\n---\n\n# Accepted\n";
        assert_eq!(
            codes(&validate_index(content, true)),
            vec![DiagnosticCode::E011]
        );
    }

    #[test]
    fn an_unknown_okf_version_is_only_a_warning() {
        let content = "---\nokf_version: \"9.9\"\n---\n";
        let result = validate_index(content, true);
        assert!(result.errors.is_empty());
        assert_eq!(result.warnings[0].code, DiagnosticCode::E013);
    }

    #[test]
    fn log_headings_must_be_iso_dates() {
        let good = "# Log\n\n## 2026-05-22\n\n* **Update**: Something.\n";
        assert!(validate_log(good).errors.is_empty());

        let bad = "# Log\n\n## May 22, 2026\n\n* **Update**: Something.\n";
        assert_eq!(codes(&validate_log(bad)), vec![DiagnosticCode::E012]);
    }

    #[test]
    fn a_stale_index_is_a_warning_not_an_error() {
        assert!(check_index_freshness("# Accepted\n", "# Accepted\n").is_none());
        assert!(check_index_freshness("# Accepted\n", "# Accepted\n\n# Proposed\n").is_some());
        // Trailing-newline drift alone must not trip the check.
        assert!(check_index_freshness("# Accepted", "# Accepted\n\n").is_none());
    }
}
