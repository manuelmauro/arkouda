//! Validate an OKF bundle of ADRs.

use crate::adr::discovery::{self, ReservedKind};
use crate::adr::{Diagnostic, DiagnosticCode, Manifest, ValidationResult, index, validator};
use crate::cli::Cli;
use crate::commands::DiscoveredBundle;
use crate::error::Result;
use colored::Colorize;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// One entry per file examined: its path, and what validation found.
type Report = Vec<(String, ValidationResult)>;

/// Run the check command.
pub fn run(cli: &Cli) -> Result<i32> {
    let dirs = super::effective_dirs(cli)?;
    let bundles = super::discover_bundles(&dirs)?;

    let (report, concept_count) = validate_bundles(&bundles)?;
    print_report(&report, concept_count, cli.quiet);

    let total_errors: usize = report.iter().map(|(_, result)| result.errors.len()).sum();
    Ok(if total_errors > 0 { 1 } else { 0 })
}

/// Validate every discovered bundle, returning a per-file report and the
/// number of concepts examined.
fn validate_bundles(bundles: &[DiscoveredBundle]) -> Result<(Report, usize)> {
    let mut report: Report = Vec::new();
    let mut concept_count = 0;

    // Concepts from every bundle are validated as one collection: within a
    // single bundle a concept id is its path, so it cannot collide, and a
    // duplicate id is only ever a cross-bundle problem (E010).
    let mut manifests: Vec<Manifest> = Vec::new();
    let mut report_indices: Vec<usize> = Vec::new();
    let mut bundle_ranges: Vec<Range<usize>> = Vec::new();

    for bundle in bundles {
        concept_count += bundle.paths.len();
        let start = manifests.len();
        parse_concepts(
            &bundle.root,
            &bundle.paths,
            &mut report,
            &mut manifests,
            &mut report_indices,
        );
        bundle_ranges.push(start..manifests.len());
    }

    for (report_index, validation) in report_indices
        .iter()
        .zip(validator::validate_collection(&manifests))
    {
        report[*report_index].1.merge(validation);
    }

    for (bundle, range) in bundles.iter().zip(bundle_ranges) {
        // A single-file `--dir` says nothing about the surrounding bundle, so
        // its reserved files are none of this invocation's business.
        if bundle.complete {
            check_reserved_files(&bundle.root, &manifests[range], &mut report)?;
        }
    }

    Ok((report, concept_count))
}

/// Parse every concept in one bundle. Unparseable files get an `E000` in
/// `report`; the rest are appended to `manifests`, with the report slot each
/// one came from recorded in `report_indices`.
fn parse_concepts(
    root: &Path,
    paths: &[PathBuf],
    report: &mut Report,
    manifests: &mut Vec<Manifest>,
    report_indices: &mut Vec<usize>,
) {
    let base = report.len();
    report.extend(
        paths
            .iter()
            .map(|path| (path.display().to_string(), ValidationResult::default())),
    );

    for (offset, path) in paths.iter().enumerate() {
        match Manifest::parse(path, root) {
            Ok(manifest) => {
                report_indices.push(base + offset);
                manifests.push(manifest);
            }
            Err(error) => report[base + offset].1.errors.push(
                Diagnostic::new(DiagnosticCode::E000, error.to_string()).with_hint(
                    "Ensure the file starts with valid YAML frontmatter delimited by `---`.",
                ),
            ),
        }
    }
}

/// Validate the reserved files OKF §3.1 defines, plus whether a bundle-root
/// `index.md` still reflects the bundle's concepts.
fn check_reserved_files(root: &Path, manifests: &[Manifest], report: &mut Report) -> Result<()> {
    for (path, kind) in discovery::find_reserved(root)? {
        let content = std::fs::read_to_string(&path)?;
        let is_bundle_root = path.parent() == Some(root);

        let mut result = match kind {
            ReservedKind::Index => validator::validate_index(&content, is_bundle_root),
            ReservedKind::Log => validator::validate_log(&content),
        };

        if kind == ReservedKind::Index
            && is_bundle_root
            && let Some(stale) =
                validator::check_index_freshness(&content, &index::render(manifests))
        {
            result.warnings.push(stale);
        }

        report.push((path.display().to_string(), result));
    }

    Ok(())
}

#[derive(Clone, Copy)]
enum DiagnosticKind {
    Error,
    Warning,
}

fn print_report(report: &Report, concept_count: usize, quiet: bool) {
    let total_errors: usize = report.iter().map(|(_, r)| r.errors.len()).sum();
    let total_warnings: usize = report.iter().map(|(_, r)| r.warnings.len()).sum();

    for (path, result) in report {
        if result.errors.is_empty() && result.warnings.is_empty() {
            continue;
        }

        println!("\n{}", path.bold());
        for diagnostic in &result.errors {
            print_diagnostic(DiagnosticKind::Error, diagnostic);
        }
        for diagnostic in &result.warnings {
            print_diagnostic(DiagnosticKind::Warning, diagnostic);
        }
    }

    if quiet && total_errors == 0 && total_warnings == 0 {
        return;
    }

    println!();
    if total_errors == 0 && total_warnings == 0 {
        println!(
            "{} {} ADR(s) checked, no issues found",
            "✓".green().bold(),
            concept_count
        );
    } else {
        let marker = if total_errors > 0 {
            "✗".red()
        } else {
            "!".yellow()
        };
        println!(
            "{} {} ADR(s) checked: {} error(s), {} warning(s)",
            marker, concept_count, total_errors, total_warnings
        );
    }
}

fn print_diagnostic(kind: DiagnosticKind, diagnostic: &Diagnostic) {
    let label = match kind {
        DiagnosticKind::Error => "error".red().bold(),
        DiagnosticKind::Warning => "warning".yellow().bold(),
    };
    let location = match diagnostic.line {
        Some(line) => format!(" {line}"),
        None => String::new(),
    };

    println!(
        "  {} {}{}: {}",
        label,
        format!("[{}]", diagnostic.code).dimmed(),
        location.dimmed(),
        diagnostic.message
    );

    if let Some(hint) = &diagnostic.fix_hint {
        println!("    {} {}", "hint:".cyan(), hint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::discovery;

    const CONCEPT: &str = "---
type: Architecture Decision Record
title: Use Postgres
description: Store relational data in Postgres.
tags: []
timestamp: 2026-05-06
status: accepted
deciders: []
---

# Use Postgres

## Status

Accepted

## Context

c

## Decision

d

## Consequences

x
";

    /// Build a bundle rooted at `root` containing one concept per name.
    fn bundle(root: &Path, concepts: &[&str]) -> DiscoveredBundle {
        std::fs::create_dir_all(root).expect("create bundle");
        for name in concepts {
            std::fs::write(root.join(format!("{name}.md")), CONCEPT).expect("write concept");
        }
        DiscoveredBundle {
            root: root.to_path_buf(),
            paths: discovery::find_concepts(root).expect("discover"),
            complete: true,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("arkouda-check-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    fn codes(report: &Report) -> Vec<DiagnosticCode> {
        report
            .iter()
            .flat_map(|(_, result)| result.errors.iter().chain(&result.warnings))
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    #[test]
    fn a_duplicate_concept_id_across_bundles_is_flagged_in_both() {
        let root = temp_dir("duplicate");
        let bundles = [
            bundle(&root.join("a"), &["use-postgres"]),
            bundle(&root.join("b"), &["use-postgres"]),
        ];

        let (report, concepts) = validate_bundles(&bundles).expect("validate");

        assert_eq!(concepts, 2);
        assert_eq!(
            codes(&report),
            vec![DiagnosticCode::E010, DiagnosticCode::E010],
            "duplicate ids only ever collide across bundles, so validation must \
             span all of them at once"
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn distinct_concept_ids_across_bundles_are_clean() {
        let root = temp_dir("distinct");
        let bundles = [
            bundle(&root.join("a"), &["use-postgres"]),
            bundle(&root.join("b"), &["use-kafka"]),
        ];

        let (report, concepts) = validate_bundles(&bundles).expect("validate");

        assert_eq!(concepts, 2);
        assert!(codes(&report).is_empty(), "{report:#?}");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_stale_index_warns_without_erroring() {
        let root = temp_dir("stale-index");
        let bundles = [bundle(&root.join("a"), &["use-postgres"])];
        std::fs::write(
            root.join("a/index.md"),
            "---\nokf_version: \"0.1\"\n---\n\n# Accepted\n\n* [Gone](gone.md)\n",
        )
        .expect("write index");

        let (report, _) = validate_bundles(&bundles).expect("validate");

        assert_eq!(codes(&report), vec![DiagnosticCode::E014]);
        assert!(report.iter().all(|(_, result)| result.errors.is_empty()));

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_generated_index_is_never_stale() {
        let root = temp_dir("fresh-index");
        let bundles = [bundle(&root.join("a"), &["use-postgres", "use-kafka"])];
        let manifests: Vec<Manifest> = bundles[0]
            .paths
            .iter()
            .map(|path| Manifest::parse(path, &bundles[0].root).expect("parse"))
            .collect();
        std::fs::write(root.join("a/index.md"), index::render(&manifests)).expect("write index");

        let (report, _) = validate_bundles(&bundles).expect("validate");
        assert!(codes(&report).is_empty(), "{report:#?}");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_incomplete_bundle_skips_reserved_file_checks() {
        let root = temp_dir("incomplete");
        let mut single = bundle(&root.join("a"), &["use-postgres", "use-kafka"]);
        std::fs::write(
            root.join("a/index.md"),
            "---\nokf_version: \"0.1\"\n---\n\n# Accepted\n",
        )
        .expect("write index");

        // Simulate `--dir <one-file>`: a partial view of the bundle.
        single.paths.truncate(1);
        single.complete = false;

        let (report, _) = validate_bundles(&[single]).expect("validate");
        assert!(
            codes(&report).is_empty(),
            "a partial view must not judge the bundle's index: {report:#?}"
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
