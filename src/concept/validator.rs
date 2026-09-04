//! Concept validation.
//!
//! Three tiers stack here, and which of them a concept is judged by depends on
//! whether its `type` resolves to a configured [`ConceptType`]:
//!
//! | Tier | Codes | Applies to |
//! | --- | --- | --- |
//! | OKF conformance | `E000`, `E004`, `E007`, `E010`, `E011`, `E012` | every concept, always |
//! | arkouda profile | `E001`, `E002`, `E006`, `E008` | concepts whose `type` resolves |
//! | template contract | `E003`, `E009` | that type's vocabulary and sections |
//!
//! The tiers are what let arkouda be strict about the format it implements and
//! permissive about the contracts a project has chosen not to write down. A
//! concept declaring a type no `[[types]]` table configures is validated at the
//! OKF tier and warned about (`E005`) — not skipped, and not a failure. Since
//! types are user-definable, an unrecognized `type` is ordinarily one the
//! operator has not declared rather than a mistake, and failing on it would
//! make `arkouda check` reject conformant OKF bundles. See the ADR
//! `support-user-defined-concept-types`.
//!
//! `E013`, `E014`, and `E015` sit outside the tiers: they are bundle- and
//! reference-level warnings that apply whatever a concept's type. OKF's
//! permissive-consumption rule (§9) is why they warn rather than fail — an
//! unknown declared OKF version, a stale `index.md`, and a concept reference
//! that does not resolve never fail a bundle.

use crate::concept::manifest::{ManifestError, split_content};
use crate::concept::types::{self, ConceptType};
use crate::concept::{Manifest, OKF_VERSION, is_valid_id, markdown};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiagnosticCode {
    /// Concept could not be parsed.
    E000,
    /// Required frontmatter field is missing.
    E001,
    /// Required frontmatter field is empty.
    E002,
    /// Status is not in this concept type's vocabulary.
    E003,
    /// Concept id is not a lowercase slug.
    E004,
    /// Frontmatter `type` is not a configured concept type. Warning.
    E005,
    /// Timestamp is not a valid ISO 8601 date or datetime.
    E006,
    /// Top-level Markdown heading is missing.
    E007,
    /// Top-level Markdown heading does not match title.
    E008,
    /// A section this concept type requires is missing.
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
    /// A frontmatter concept reference does not resolve to a loaded concept.
    /// Warning.
    E015,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Validate a collection of parsed concepts. Checks that need to see the
/// whole collection — duplicate ids, and whether frontmatter references
/// resolve — happen here.
pub fn validate_collection(manifests: &[Manifest]) -> Vec<ValidationResult> {
    let mut results: Vec<ValidationResult> = manifests.iter().map(validate).collect();
    check_duplicate_ids(manifests, &mut results);
    check_references(manifests, &mut results);
    results
}

/// Validate one parsed concept, at whichever tiers its `type` unlocks.
pub fn validate(manifest: &Manifest) -> ValidationResult {
    let mut result = ValidationResult::default();

    // Tier 1 — OKF conformance. True of every concept in a bundle, whatever
    // it declares itself to be.
    check_concept_id(manifest, &mut result);
    check_title_heading_present(manifest, &mut result);

    let declared = non_empty(manifest.frontmatter.concept_type.as_deref());
    let Some(declared) = declared else {
        // `type` is the one key OKF itself requires, so its absence is an
        // error even though nothing else in the profile applies.
        check_required_field(
            &mut result,
            "type",
            manifest.frontmatter.concept_type.as_deref(),
            &format!(
                "Add `type:` to the YAML frontmatter. It selects the contract this \
                 concept is checked against; this project knows {}.",
                types::okf_type_list()
            ),
        );
        return result;
    };

    let Some(concept_type) = types::by_okf_type(declared) else {
        result.warnings.push(
            Diagnostic::new(
                DiagnosticCode::E005,
                format!("no configured concept type declares `type: {declared}`"),
            )
            .with_hint(format!(
                "Checked for OKF conformance only. Configured types are: {}. Add a `[[types]]` \
                 table to .arkoudarc.toml to have arkouda check this concept's status and \
                 sections too.",
                types::okf_type_list()
            )),
        );
        return result;
    };

    // Tier 2 — arkouda's profile. Layered on top of OKF, which requires none
    // of these, and applied only where a contract says what they mean.
    check_profile_fields(manifest, concept_type, &mut result);
    check_timestamp(manifest, &mut result);
    check_title_heading_matches(manifest, &mut result);

    // Tier 3 — the type's own contract.
    check_status_vocabulary(manifest, concept_type, &mut result);
    check_required_sections(manifest, concept_type, &mut result);

    result
}

/// The frontmatter keys arkouda's profile requires beyond OKF's `type`.
fn check_profile_fields(
    manifest: &Manifest,
    concept_type: &ConceptType,
    result: &mut ValidationResult,
) {
    check_required_field(
        result,
        "title",
        manifest.frontmatter.title.as_deref(),
        "Add a human-readable `title` to the YAML frontmatter.",
    );
    check_required_field(
        result,
        "description",
        manifest.frontmatter.description.as_deref(),
        "Add a `description` summarizing the concept itself (what was decided, or \
         what is being built — not just the topic) in one sentence.",
    );
    check_required_field(
        result,
        "status",
        manifest.frontmatter.status.as_deref(),
        &format!(
            "Add `status: {}` or another value from: {}.",
            concept_type.default_status().name,
            concept_type.status_list()
        ),
    );
    check_required_field(
        result,
        "timestamp",
        manifest.frontmatter.timestamp.as_deref(),
        "Add `timestamp: YYYY-MM-DD` to the YAML frontmatter.",
    );
}

fn check_timestamp(manifest: &Manifest, result: &mut ValidationResult) {
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
}

fn check_status_vocabulary(
    manifest: &Manifest,
    concept_type: &ConceptType,
    result: &mut ValidationResult,
) {
    if let Some(status) = non_empty(manifest.frontmatter.status.as_deref())
        && concept_type.status(status).is_none()
    {
        result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E003,
                format!(
                    "invalid status `{status}` for type `{}`",
                    concept_type.okf_type
                ),
            )
            .with_hint(format!("Use one of: {}.", concept_type.status_list())),
        );
    }
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

/// A concept document is a Markdown document with a title (OKF §3.2), so a
/// missing `#` heading is an OKF-tier error regardless of type.
fn check_title_heading_present(manifest: &Manifest, result: &mut ValidationResult) {
    if markdown::headings(&manifest.body)
        .iter()
        .any(|heading| heading.level == markdown::TITLE_LEVEL)
    {
        return;
    }

    let hint = match non_empty(manifest.frontmatter.title.as_deref()) {
        Some(title) => format!("Add `# {title}` after the frontmatter."),
        None => "Add a `# <title>` heading after the frontmatter.".to_owned(),
    };
    result.errors.push(
        Diagnostic::new(DiagnosticCode::E007, "missing top-level Markdown heading").with_hint(hint),
    );
}

/// That the heading agrees with the frontmatter `title` is arkouda's profile,
/// not OKF's: OKF requires neither key.
fn check_title_heading_matches(manifest: &Manifest, result: &mut ValidationResult) {
    let Some(title) = non_empty(manifest.frontmatter.title.as_deref()) else {
        return;
    };

    let headings = markdown::headings(&manifest.body);
    let Some(h1) = headings
        .iter()
        .find(|heading| heading.level == markdown::TITLE_LEVEL)
    else {
        // Already reported at the OKF tier.
        return;
    };

    if h1.text != title {
        let line_index = markdown::line_index(&manifest.body, h1.span.start);
        result.errors.push(
            Diagnostic::new(
                DiagnosticCode::E008,
                format!(
                    "top-level heading `{}` does not match title `{title}`",
                    h1.text
                ),
            )
            .with_line(manifest.body_start_line + line_index)
            .with_hint(format!("Change the heading to `# {title}`.")),
        );
    }
}

fn check_required_sections(
    manifest: &Manifest,
    concept_type: &ConceptType,
    result: &mut ValidationResult,
) {
    if concept_type.required_sections.is_empty() {
        return;
    }

    let sections: HashSet<String> = markdown::headings(&manifest.body)
        .into_iter()
        .filter(|heading| heading.level == markdown::SECTION_LEVEL)
        .map(|heading| heading.text.to_ascii_lowercase())
        .collect();

    for required in &concept_type.required_sections {
        if !sections.contains(&required.to_ascii_lowercase()) {
            result.errors.push(
                Diagnostic::new(
                    DiagnosticCode::E009,
                    format!("missing required Markdown section `## {required}`"),
                )
                .with_hint(format!(
                    "Add a `## {required}` section; a {} requires it.",
                    concept_type.okf_type
                )),
            );
        }
    }
}

/// Resolve every frontmatter concept reference against the loaded collection.
///
/// A dangling reference is a **warning**, not an error, because arkouda cannot
/// tell a broken reference from an out-of-scope one: `arkouda check --dir
/// docs/prd` loads one bundle, and a `decisions` entry pointing into
/// `docs/adr` is then legitimately unresolvable. Failing there would make
/// `check` depend on which directories the invocation happened to cover.
fn check_references(manifests: &[Manifest], results: &mut [ValidationResult]) {
    let known: HashSet<&str> = manifests
        .iter()
        .map(|manifest| manifest.concept_id.as_str())
        .collect();

    for (manifest, result) in manifests.iter().zip(results.iter_mut()) {
        for (field, reference) in manifest.frontmatter.references() {
            if known.contains(reference) {
                continue;
            }
            result.warnings.push(
                Diagnostic::new(
                    DiagnosticCode::E015,
                    format!("`{field}` references `{reference}`, which is not a loaded concept"),
                )
                .with_hint(
                    "Use a bundle-relative concept id (`security/mtls`, not a path). If the \
                     concept lives in a bundle this invocation did not load, widen `--dir` or \
                     `dirs` to cover it.",
                ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept::Manifest;
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

    fn good_prd() -> String {
        "---
type: Product Requirements Document
title: Bulk ADR Import
description: Import a directory of loose Markdown files as ADRs.
status: draft
timestamp: 2026-08-07
---

# Bulk ADR Import

## Status

Draft

## Problem

Problem.

## Requirements

Requirements.

## Non-Goals

Non-goals.

## Success Metrics

Metrics.
"
        .to_owned()
    }

    fn codes(result: &ValidationResult) -> Vec<DiagnosticCode> {
        result.errors.iter().map(|d| d.code).collect()
    }

    fn warning_codes(result: &ValidationResult) -> Vec<DiagnosticCode> {
        result.warnings.iter().map(|d| d.code).collect()
    }

    #[test]
    fn validates_a_good_adr() {
        let manifest = parse("docs/adr/basic-adr-cli.md", &good_adr());
        let result = validate(&manifest);
        assert!(result.errors.is_empty(), "{:#?}", result.errors);
    }

    #[test]
    fn validates_a_good_prd() {
        let manifest = parse("docs/prd/bulk-adr-import.md", &good_prd());
        let result = validate(&manifest);
        assert!(result.errors.is_empty(), "{:#?}", result.errors);
    }

    #[test]
    fn a_prd_is_not_checked_against_the_adr_section_set() {
        // The whole point of the descriptor: a requirements document must not
        // be asked for a `## Decision`, and an ADR must not be asked for
        // `## Requirements`.
        let prd = parse("docs/prd/bulk-adr-import.md", &good_prd());
        assert!(validate(&prd).errors.is_empty());

        let adr_sections_only = good_prd().replace("## Problem", "## Context");
        let manifest = parse("docs/prd/bulk-adr-import.md", &adr_sections_only);
        let result = validate(&manifest);
        let messages: Vec<&str> = result
            .errors
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert_eq!(
            messages,
            ["missing required Markdown section `## Problem`"],
            "borrowing the ADR's word for the problem is not enough"
        );
    }

    #[test]
    fn a_section_heading_inside_a_fence_does_not_satisfy_the_requirement() {
        // Before Markdown was parsed rather than scanned, this validated
        // cleanly: the scanner saw `## Decision` and `## Consequences` inside
        // the fence and reported four required sections present on a document
        // that has two.
        let content = "---
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

An ADR template looks like this:

```markdown
## Decision

## Consequences
```
";
        let manifest = parse("docs/adr/basic-adr-cli.md", content);
        let codes = codes(&validate(&manifest));
        assert_eq!(
            codes,
            [DiagnosticCode::E009, DiagnosticCode::E009],
            "the fenced headings are code, so Decision and Consequences are missing"
        );
    }

    #[test]
    fn a_concept_may_document_its_own_format() {
        // The complement of the test above: a real section whose body carries
        // a fenced template must still validate, and must not be truncated.
        let content = good_adr().replace(
            "Decision.\n",
            "Use this shape:\n\n```markdown\n# Title\n\n## Status\n\n## Context\n```\n\nThat is all.\n",
        );
        let manifest = parse("docs/adr/basic-adr-cli.md", &content);

        assert!(validate(&manifest).errors.is_empty());
        let decision = manifest.section("decision").expect("decision present");
        assert!(
            decision.contains("## Context") && decision.ends_with("That is all."),
            "the fence must not end the section: {decision}"
        );
    }

    #[test]
    fn requires_the_okf_type_field() {
        let content = good_adr().replace("type: Architecture Decision Record\n", "");
        let manifest = parse("docs/adr/basic-adr-cli.md", &content);
        assert!(codes(&validate(&manifest)).contains(&DiagnosticCode::E001));
    }

    #[test]
    fn an_unconfigured_type_warns_and_stops_at_the_okf_tier() {
        let content = good_adr().replace("Architecture Decision Record", "BigQuery Table");
        let manifest = parse("docs/adr/basic-adr-cli.md", &content);
        let result = validate(&manifest);

        assert_eq!(
            warning_codes(&result),
            [DiagnosticCode::E005],
            "an unconfigured type is reported, not silently skipped: {result:#?}"
        );
        assert!(
            result.errors.is_empty(),
            "a conformant OKF concept must not fail a bundle just because no \
             `[[types]]` table describes it: {:#?}",
            result.errors
        );
    }

    #[test]
    fn an_unconfigured_type_is_still_checked_for_okf_conformance() {
        // The concept is not skipped: its id and its `#` heading are OKF's
        // business whatever the document claims to be.
        let content = good_adr()
            .replace("Architecture Decision Record", "BigQuery Table")
            .replace("# Basic ADR CLI\n", "");
        let manifest = parse("docs/adr/Basic_CLI.md", &content);
        let result = validate(&manifest);

        assert!(
            codes(&result).contains(&DiagnosticCode::E007),
            "a missing title heading is OKF-tier: {result:#?}"
        );
        assert!(
            codes(&result).contains(&DiagnosticCode::E004),
            "a non-slug concept id is OKF-tier: {result:#?}"
        );
    }

    #[test]
    fn a_missing_type_is_still_an_error() {
        // `type` is the one key OKF itself requires, so its absence is not the
        // same as declaring a type nothing configures.
        let content = good_adr().replace("type: Architecture Decision Record\n", "");
        let manifest = parse("docs/adr/basic-adr-cli.md", &content);
        assert!(codes(&validate(&manifest)).contains(&DiagnosticCode::E001));
    }

    #[test]
    fn the_profile_and_template_tiers_are_skipped_for_an_unconfigured_type() {
        // Everything arkouda layers on top of OKF — its required frontmatter,
        // a status vocabulary, a section set — presupposes a contract. A
        // concept with none must not be judged against another type's.
        let content = "---\ntype: Runbook\n---\n\n# Restart the queue\n\n## Steps\n\nDo it.\n";
        let manifest = parse("docs/ops/restart-the-queue.md", content);
        let result = validate(&manifest);

        assert!(result.errors.is_empty(), "{:#?}", result.errors);
        assert_eq!(warning_codes(&result), [DiagnosticCode::E005]);
    }

    #[test]
    fn statuses_are_checked_against_the_declared_type() {
        // `shipped` is valid for a PRD and invalid for an ADR; `accepted` is
        // the other way round.
        let adr = good_adr().replace("status: proposed", "status: shipped");
        assert!(codes(&validate(&parse("docs/adr/x.md", &adr))).contains(&DiagnosticCode::E003));

        let prd = good_prd().replace("status: draft", "status: accepted");
        assert!(codes(&validate(&parse("docs/prd/x.md", &prd))).contains(&DiagnosticCode::E003));

        let prd = good_prd().replace("status: draft", "status: in-review");
        assert!(validate(&parse("docs/prd/x.md", &prd)).errors.is_empty());
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
    fn a_resolvable_reference_is_clean() {
        let prd = good_prd().replace(
            "status: draft",
            "status: draft\ndecisions:\n  - basic-adr-cli",
        );
        let results = validate_collection(&[
            parse("docs/prd/bulk-adr-import.md", &prd),
            parse("docs/adr/basic-adr-cli.md", &good_adr()),
        ]);
        assert!(
            results.iter().all(|r| r.warnings.is_empty()),
            "{results:#?}"
        );
    }

    #[test]
    fn a_dangling_reference_warns_without_failing_the_bundle() {
        let prd = good_prd().replace("status: draft", "status: draft\ndecisions:\n  - gone");
        let results = validate_collection(&[parse("docs/prd/bulk-adr-import.md", &prd)]);
        assert_eq!(warning_codes(&results[0]), [DiagnosticCode::E015]);
        assert!(
            results[0].errors.is_empty(),
            "a reference into a bundle this run did not load is not a defect"
        );
    }

    #[test]
    fn superseded_by_is_resolved_too() {
        // Parsed since it was introduced, never checked until now.
        let content = good_adr().replace(
            "status: proposed",
            "status: superseded\nsuperseded_by: gone",
        );
        let results = validate_collection(&[parse("docs/adr/basic-adr-cli.md", &content)]);
        assert_eq!(warning_codes(&results[0]), [DiagnosticCode::E015]);
        assert!(results[0].warnings[0].message.contains("superseded_by"));
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
