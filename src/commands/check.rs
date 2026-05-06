//! Validate ADRs.

use crate::adr::{Diagnostic, DiagnosticCode, Manifest, ValidationResult, Validator};
use crate::cli::{CheckArgs, Cli};
use crate::error::Result;
use colored::Colorize;

/// Run the check command.
pub fn run(_args: CheckArgs, cli: &Cli) -> Result<i32> {
    let paths = super::discover_paths(&cli.dir)?;
    let mut manifests = Vec::new();
    let mut manifest_indexes = Vec::new();
    let mut results = paths
        .iter()
        .map(|path| (path.display().to_string(), ValidationResult::default()))
        .collect::<Vec<_>>();

    for (index, path) in paths.iter().enumerate() {
        match Manifest::parse(path.clone()) {
            Ok(manifest) => {
                manifest_indexes.push(index);
                manifests.push(manifest);
            }
            Err(error) => {
                results[index].1.errors.push(
                    Diagnostic::error(DiagnosticCode::E000, error.to_string()).with_hint(
                        "Ensure the file starts with valid YAML frontmatter delimited by `---`.",
                    ),
                );
            }
        }
    }

    let validation_results = Validator::validate_collection(&manifests);
    for (manifest_index, validation_result) in manifest_indexes.into_iter().zip(validation_results)
    {
        results[manifest_index].1.merge(validation_result);
    }

    print_validation(&results, cli.quiet);

    let total_errors: usize = results.iter().map(|(_, result)| result.errors.len()).sum();
    Ok(if total_errors == 0 { 0 } else { 1 })
}

fn print_validation(results: &[(String, ValidationResult)], quiet: bool) {
    let total_errors: usize = results.iter().map(|(_, result)| result.errors.len()).sum();
    let total_warnings: usize = results
        .iter()
        .map(|(_, result)| result.warnings.len())
        .sum();

    for (path, result) in results {
        if result.errors.is_empty() && result.warnings.is_empty() {
            continue;
        }

        println!("\n{}", path.bold());

        for diagnostic in &result.errors {
            print_diagnostic("error", diagnostic);
        }

        for diagnostic in &result.warnings {
            print_diagnostic("warning", diagnostic);
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
            results.len()
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
            results.len(),
            total_errors,
            total_warnings
        );
    }
}

fn print_diagnostic(kind: &str, diagnostic: &Diagnostic) {
    let kind = match kind {
        "error" => "error".red().bold(),
        "warning" => "warning".yellow().bold(),
        _ => kind.normal(),
    };
    let location = diagnostic
        .line
        .map(|line| format!("{line}:"))
        .unwrap_or_default();

    println!(
        "  {} {} {}: {}",
        kind,
        format!("[{}]", diagnostic.code).dimmed(),
        location.dimmed(),
        diagnostic.message
    );

    if let Some(hint) = &diagnostic.fix_hint {
        println!("    {} {}", "hint:".cyan(), hint);
    }
}
