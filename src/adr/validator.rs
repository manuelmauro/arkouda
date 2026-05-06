//! ADR validation.

use crate::adr::{Manifest, is_valid_id};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Valid ADR status values.
pub const VALID_STATUSES: &[&str] = &[
    "proposed",
    "accepted",
    "superseded",
    "deprecated",
    "rejected",
];

/// Result of validating an ADR.
#[derive(Debug, Default, Clone)]
pub struct ValidationResult {
    /// Validation errors.
    pub errors: Vec<Diagnostic>,
    /// Validation warnings.
    pub warnings: Vec<Diagnostic>,
}

impl ValidationResult {
    /// Returns true if there are no errors.
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Merge another result into this one.
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// A validation diagnostic.
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
    /// Create an error diagnostic.
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            line: None,
            message: message.into(),
            code,
            fix_hint: None,
        }
    }

    /// Set a line number.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set a fix hint.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }
}

/// Diagnostic codes for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// ADR could not be parsed.
    E000,
    /// Required frontmatter field is missing.
    E001,
    /// Required frontmatter field is empty.
    E002,
    /// Status is not in the controlled status list.
    E003,
    /// ADR id is not a valid slug.
    E004,
    /// ADR filename stem does not match frontmatter id.
    E005,
    /// Date is not a valid YYYY-MM-DD date.
    E006,
    /// Top-level Markdown heading is missing.
    E007,
    /// Top-level Markdown heading does not match title.
    E008,
    /// Required Markdown section is missing.
    E009,
    /// ADR id is duplicated.
    E010,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::E000 => write!(f, "E000"),
            Self::E001 => write!(f, "E001"),
            Self::E002 => write!(f, "E002"),
            Self::E003 => write!(f, "E003"),
            Self::E004 => write!(f, "E004"),
            Self::E005 => write!(f, "E005"),
            Self::E006 => write!(f, "E006"),
            Self::E007 => write!(f, "E007"),
            Self::E008 => write!(f, "E008"),
            Self::E009 => write!(f, "E009"),
            Self::E010 => write!(f, "E010"),
        }
    }
}

/// ADR validator.
pub struct Validator;

impl Validator {
    /// Validate a collection of parsed ADR manifests.
    pub fn validate_collection(manifests: &[Manifest]) -> Vec<ValidationResult> {
        let mut results: Vec<ValidationResult> = manifests.iter().map(Self::validate).collect();
        Self::check_duplicate_ids(manifests, &mut results);
        results
    }

    /// Validate one parsed ADR manifest.
    pub fn validate(manifest: &Manifest) -> ValidationResult {
        let mut result = ValidationResult::default();

        check_required_field(
            &mut result,
            "id",
            manifest.frontmatter.id.as_deref(),
            "Add `id: \"my-decision-slug\"` to the YAML frontmatter.",
        );
        check_required_field(
            &mut result,
            "title",
            manifest.frontmatter.title.as_deref(),
            "Add a human-readable `title` to the YAML frontmatter.",
        );
        check_required_field(
            &mut result,
            "abstract",
            manifest.frontmatter.abstract_text.as_deref(),
            "Add an `abstract` summarizing the decision in one or two sentences.",
        );
        check_required_field(
            &mut result,
            "status",
            manifest.frontmatter.status.as_deref(),
            "Add `status: \"proposed\"` or another valid status.",
        );
        check_required_field(
            &mut result,
            "date",
            manifest.frontmatter.date.as_deref(),
            "Add `date: \"YYYY-MM-DD\"` to the YAML frontmatter.",
        );

        if let Some(id) = non_empty(manifest.frontmatter.id.as_deref()) {
            if !is_valid_id(id) {
                result.errors.push(
                    Diagnostic::error(
                        DiagnosticCode::E004,
                        format!("ADR id `{id}` must be a lowercase slug"),
                    )
                    .with_hint("Use lowercase letters, numbers, and single hyphens only."),
                );
            }

            if let Some(stem) = manifest.path.file_stem().and_then(|stem| stem.to_str())
                && stem != id
            {
                result.errors.push(
                    Diagnostic::error(
                        DiagnosticCode::E005,
                        format!("filename stem `{stem}` does not match ADR id `{id}`"),
                    )
                    .with_hint(format!("Rename this file to `{id}.md`.")),
                );
            }
        }

        if let Some(status) = non_empty(manifest.frontmatter.status.as_deref())
            && !VALID_STATUSES.contains(&status)
        {
            result.errors.push(
                Diagnostic::error(DiagnosticCode::E003, format!("invalid status `{status}`"))
                    .with_hint(format!("Use one of: {}.", VALID_STATUSES.join(", "))),
            );
        }

        if let Some(date) = non_empty(manifest.frontmatter.date.as_deref())
            && !is_valid_iso_date(date)
        {
            result.errors.push(
                Diagnostic::error(
                    DiagnosticCode::E006,
                    format!("date `{date}` must be a valid YYYY-MM-DD date"),
                )
                .with_hint("Use an ISO date such as `2026-05-06`."),
            );
        }

        check_title_heading(manifest, &mut result);
        check_required_sections(manifest, &mut result);

        result
    }

    fn check_duplicate_ids(manifests: &[Manifest], results: &mut [ValidationResult]) {
        let mut id_to_indexes: HashMap<&str, Vec<usize>> = HashMap::new();

        for (index, manifest) in manifests.iter().enumerate() {
            if let Some(id) = non_empty(manifest.frontmatter.id.as_deref()) {
                id_to_indexes.entry(id).or_default().push(index);
            }
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
                    Diagnostic::error(DiagnosticCode::E010, format!("ADR id `{id}` is duplicated"))
                        .with_hint(format!("Use unique ids. Also found in: {paths}")),
                );
            }
        }
    }
}

fn check_required_field(
    result: &mut ValidationResult,
    field: &str,
    value: Option<&str>,
    hint: &'static str,
) {
    match value {
        None => result.errors.push(
            Diagnostic::error(
                DiagnosticCode::E001,
                format!("missing required frontmatter field `{field}`"),
            )
            .with_hint(hint),
        ),
        Some(value) if value.trim().is_empty() => result.errors.push(
            Diagnostic::error(
                DiagnosticCode::E002,
                format!("frontmatter field `{field}` must not be empty"),
            )
            .with_hint(hint),
        ),
        Some(_) => {}
    }
}

fn check_title_heading(manifest: &Manifest, result: &mut ValidationResult) {
    let title = match non_empty(manifest.frontmatter.title.as_deref()) {
        Some(title) => title,
        None => return,
    };

    let h1 = manifest
        .body
        .lines()
        .enumerate()
        .find_map(|(line_index, line)| {
            line.strip_prefix("# ")
                .map(str::trim)
                .map(|heading| (line_index, heading))
        });

    match h1 {
        None => result.errors.push(
            Diagnostic::error(DiagnosticCode::E007, "missing top-level Markdown heading")
                .with_hint(format!("Add `# {title}` after the frontmatter.")),
        ),
        Some((line_index, heading)) if heading != title => result.errors.push(
            Diagnostic::error(
                DiagnosticCode::E008,
                format!("top-level heading `{heading}` does not match title `{title}`"),
            )
            .with_line(manifest.body_start_line + line_index)
            .with_hint(format!("Change the heading to `# {title}`.")),
        ),
        Some(_) => {}
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
            result.errors.push(
                Diagnostic::error(
                    DiagnosticCode::E009,
                    format!(
                        "missing required Markdown section `## {}`",
                        title_case(required)
                    ),
                )
                .with_hint(format!(
                    "Add a `## {}` section to the ADR body.",
                    title_case(required)
                )),
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

fn is_valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }

    if !bytes
        .iter()
        .enumerate()
        .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return false;
    }

    let Ok(year) = value[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };

    if month == 0 || month > 12 || day == 0 {
        return false;
    }

    day <= days_in_month(year, month)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::Manifest;
    use std::path::PathBuf;

    #[test]
    fn validates_a_good_adr() {
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

## Context

Context.

## Decision

Decision.

## Consequences

Consequences.
"#;
        let manifest = Manifest::parse_content(PathBuf::from("docs/adr/basic-adr-cli.md"), content)
            .expect("valid manifest");
        let result = Validator::validate(&manifest);

        assert!(result.errors.is_empty(), "{:#?}", result.errors);
    }

    #[test]
    fn rejects_invalid_dates() {
        assert!(is_valid_iso_date("2026-05-06"));
        assert!(is_valid_iso_date("2024-02-29"));
        assert!(!is_valid_iso_date("2026-02-29"));
        assert!(!is_valid_iso_date("2026-13-01"));
        assert!(!is_valid_iso_date("06-05-2026"));
    }
}
