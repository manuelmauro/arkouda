//! Create a new ADR concept from the standard template.

use crate::adr::{ADR_TYPE, AdrStatus, index, is_valid_id, slugify};
use crate::cli::{Cli, NewArgs};
use crate::error::{ArkoudaError, Result};
use chrono::Local;
use colored::Colorize;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Run the new command.
pub fn run(args: &NewArgs, cli: &Cli) -> Result<i32> {
    let id = match args.id.as_deref() {
        Some(explicit) => explicit.to_owned(),
        None => slugify(&args.title),
    };
    if !is_valid_id(&id) {
        return Err(ArkoudaError::InvalidId(id));
    }

    let dirs = super::effective_dirs(cli)?;
    let target_dir = super::primary_dir(&dirs);
    std::fs::create_dir_all(target_dir)?;
    let path = target_dir.join(format!("{id}.md"));
    if path.exists() {
        return Err(ArkoudaError::AdrExists {
            id,
            path: path.display().to_string(),
        });
    }

    let timestamp = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let description = args
        .description
        .as_deref()
        .unwrap_or("TODO: summarize the decision in one sentence.");
    let content = render_template(&args.title, description, args.status, &timestamp);

    std::fs::write(&path, content)?;

    if !cli.quiet {
        println!(
            "{} Created ADR '{}' at {}",
            "✓".green().bold(),
            id,
            path.display()
        );
    }

    // The ADR is on disk; the command has succeeded. Refreshing the index
    // re-parses the whole bundle, so an unrelated malformed concept must not
    // turn a successful creation into a failing exit code. A stale index is
    // only ever a warning (E014), and `arkouda check` will say so.
    if let Err(error) = refresh_index(target_dir, cli)
        && !cli.quiet
    {
        eprintln!(
            "{} index not refreshed: {error}",
            "warning:".yellow().bold()
        );
        eprintln!(
            "    {} Run `arkouda index` once the bundle validates.",
            "hint:".cyan()
        );
    }

    Ok(0)
}

/// Keep a bundle's `index.md` in step with the concept just added. A bundle
/// without an index stays without one — OKF §9 makes the index optional, so
/// creating one unasked would be a surprise.
fn refresh_index(target_dir: &Path, cli: &Cli) -> Result<()> {
    let root = super::bundle_root(target_dir);
    if !root.join("index.md").exists() {
        return Ok(());
    }

    let dirs = [PathBuf::from(target_dir)];
    for bundle in super::load_bundles(&dirs)?.iter().filter(|b| b.complete) {
        let path = super::index::write(&bundle.root, &index::render(&bundle.manifests))?;
        if !cli.quiet {
            println!("{} Refreshed {}", "✓".green().bold(), path.display());
        }
    }

    Ok(())
}

/// OKF frontmatter: the spec's required `type` and recommended fields first,
/// then the ADR-specific extensions.
#[derive(Serialize)]
struct TemplateFrontmatter<'a> {
    #[serde(rename = "type")]
    concept_type: &'a str,
    title: &'a str,
    description: &'a str,
    tags: &'a [String],
    timestamp: &'a str,
    status: AdrStatus,
    deciders: &'a [String],
}

fn render_template(title: &str, description: &str, status: AdrStatus, timestamp: &str) -> String {
    let frontmatter = TemplateFrontmatter {
        concept_type: ADR_TYPE,
        title,
        description,
        tags: &[],
        timestamp,
        status,
        deciders: &[],
    };
    let yaml = serde_yaml::to_string(&frontmatter)
        .expect("frontmatter serialization is infallible for static fields");

    format!(
        "---
{yaml}---

# {title}

## Status

{label}

## Context

TODO: describe the forces, constraints, and background for this decision.

## Decision

TODO: describe the decision.

## Consequences

TODO: describe the positive, negative, and neutral consequences.
",
        label = status.label(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adr::{Manifest, validator};
    use crate::cli::Command;
    use clap::Parser;

    fn temp_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("arkouda-new-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create dir");
        root
    }

    /// Parse a `new` invocation and run it against `dir`.
    fn run_new(dir: &Path, title: &str) -> Result<i32> {
        let cli = Cli::parse_from([
            "arkouda",
            "--quiet",
            "--dir",
            dir.to_str().expect("utf-8 path"),
            "new",
            title,
        ]);
        let Command::New(args) = &cli.command else {
            unreachable!("parsed a `new` invocation")
        };
        run(args, &cli)
    }

    #[test]
    fn a_failed_index_refresh_does_not_fail_the_creation() {
        let root = temp_dir("broken-sibling");
        // An unrelated malformed concept makes `refresh_index` fail, because
        // refreshing re-parses every concept in the bundle.
        std::fs::write(root.join("broken.md"), "no frontmatter here\n").expect("write");
        std::fs::write(root.join("index.md"), "---\nokf_version: \"0.1\"\n---\n").expect("write");

        let exit = run_new(&root, "Use Postgres").expect("creation must not error");

        assert_eq!(exit, 0, "the ADR was created; the exit code must say so");
        assert!(root.join("use-postgres.md").is_file());

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn creation_refreshes_an_existing_index() {
        let root = temp_dir("refresh");
        std::fs::write(root.join("index.md"), "---\nokf_version: \"0.1\"\n---\n").expect("write");

        assert_eq!(run_new(&root, "Use Postgres").expect("create"), 0);

        let index = std::fs::read_to_string(root.join("index.md")).expect("read index");
        assert!(
            index.contains("* [Use Postgres](use-postgres.md)"),
            "{index}"
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn creation_never_conjures_an_index() {
        let root = temp_dir("no-index");

        assert_eq!(run_new(&root, "Use Postgres").expect("create"), 0);

        assert!(
            !root.join("index.md").exists(),
            "OKF makes the index optional; `new` must not create one unasked"
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn the_template_validates_and_declares_the_okf_type() {
        let rendered = render_template(
            "Use Postgres",
            "Store relational data in Postgres.",
            AdrStatus::Proposed,
            "2026-05-06",
        );

        let manifest = Manifest::parse_content(
            Path::new("docs/adr/use-postgres.md"),
            Path::new("docs/adr"),
            &rendered,
        )
        .expect("template parses");

        assert_eq!(manifest.frontmatter.concept_type.as_deref(), Some(ADR_TYPE));
        assert_eq!(manifest.concept_id, "use-postgres");

        let result = validator::validate(&manifest);
        assert!(result.errors.is_empty(), "{:#?}", result.errors);
    }
}
