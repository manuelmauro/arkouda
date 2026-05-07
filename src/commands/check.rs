//! Validate ADRs.

use crate::adr::{Diagnostic, DiagnosticCode, Manifest, ValidationResult, validator};
use crate::cli::Cli;
use crate::error::Result;
use colored::Colorize;
use std::process::ExitCode;

/// Run the check command.
pub fn run(cli: &Cli) -> Result<ExitCode> {
    let dirs = super::effective_dirs(cli)?;
    let paths = super::discover_paths(&dirs)?;

    let mut report: Vec<(String, ValidationResult)> = paths
        .iter()
        .map(|path| (path.display().to_string(), ValidationResult::default()))
        .collect();

    let mut parsed_indices = Vec::new();
    let mut manifests = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        match Manifest::parse(path) {
            Ok(manifest) => {
                parsed_indices.push(index);
                manifests.push(manifest);
            }
            Err(error) => report[index].1.errors.push(
                Diagnostic::error(DiagnosticCode::E000, error.to_string()).with_hint(
                    "Ensure the file starts with valid YAML frontmatter delimited by `---`.",
                ),
            ),
        }
    }

    for (index, validation) in parsed_indices
        .into_iter()
        .zip(validator::validate_collection(&manifests))
    {
        report[index].1.merge(validation);
    }

    print_report(&report, cli.quiet);

    let total_errors: usize = report.iter().map(|(_, r)| r.errors.len()).sum();
    Ok(if total_errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

#[derive(Clone, Copy)]
enum DiagnosticKind {
    Error,
    Warning,
}

fn print_report(report: &[(String, ValidationResult)], quiet: bool) {
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
            report.len()
        );
    } else {
        let marker = if total_errors > 0 {
            "✗".red()
        } else {
            "!".yellow()
        };
        println!(
            "{} {} ADR(s) checked: {} error(s), {} warning(s)",
            marker,
            report.len(),
            total_errors,
            total_warnings
        );
    }
}

fn print_diagnostic(kind: DiagnosticKind, diagnostic: &Diagnostic) {
    let label = match kind {
        DiagnosticKind::Error => "error".red().bold(),
        DiagnosticKind::Warning => "warning".yellow().bold(),
    };
    let location = diagnostic
        .line
        .map(|line| format!("{line}:"))
        .unwrap_or_default();

    println!(
        "  {} {} {}: {}",
        label,
        format!("[{}]", diagnostic.code).dimmed(),
        location.dimmed(),
        diagnostic.message
    );

    if let Some(hint) = &diagnostic.fix_hint {
        println!("    {} {}", "hint:".cyan(), hint);
    }
}
