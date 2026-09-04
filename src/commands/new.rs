//! Create a new concept from its type's template.

use crate::cli::{Cli, NewArgs};
use crate::commands::Outcome;
use crate::concept::types::{ConceptType, Status};
use crate::concept::{index, is_valid_id, slugify};
use crate::error::{ArkoudaError, Result};
use chrono::Local;
use colored::Colorize;
use serde_yaml::{Mapping, Value};
use std::path::{Path, PathBuf};

/// Run the new command.
pub fn run(args: &NewArgs, cli: &Cli) -> Result<Outcome> {
    let concept_type = super::resolve_type(&args.concept_type)?;
    let status = resolve_status(concept_type, args.status.as_deref())?;

    let id = match args.id.as_deref() {
        Some(explicit) => explicit.to_owned(),
        None => slugify(&args.title, &concept_type.slug),
    };
    if !is_valid_id(&id) {
        return Err(ArkoudaError::InvalidId(id));
    }

    let dirs = super::effective_dirs(cli)?;
    let target_dir = super::write_dir(&dirs, concept_type)?.to_path_buf();
    std::fs::create_dir_all(&target_dir)?;
    let path = target_dir.join(format!("{id}.md"));
    if path.exists() {
        return Err(ArkoudaError::ConceptExists {
            id,
            path: path.display().to_string(),
        });
    }

    let timestamp = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let description = args
        .description
        .as_deref()
        .unwrap_or("TODO: summarize this concept in one sentence.");
    let content = render_template(concept_type, &args.title, description, status, &timestamp);

    std::fs::write(&path, content)?;

    if !cli.quiet {
        println!(
            "{} Created {} '{}' at {}",
            "✓".green().bold(),
            concept_type.okf_type,
            id,
            path.display()
        );
    }

    // The concept is on disk; the command has succeeded. Refreshing the index
    // re-parses the whole bundle, so an unrelated malformed concept must not
    // turn a successful creation into a failing exit code. A stale index is
    // only ever a warning (E014), and `arkouda check` will say so.
    if let Err(error) = refresh_index(&target_dir, cli)
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

    Ok(Outcome {
        exit: 0,
        codes: Vec::new(),
        type_kind: Some(concept_type.origin),
    })
}

/// Validate `--status` against the resolved type's vocabulary. Statuses are no
/// longer a clap `ValueEnum` because which values are valid depends on
/// `--type`, so the check happens here.
fn resolve_status(
    concept_type: &'static ConceptType,
    requested: Option<&str>,
) -> Result<&'static Status> {
    let Some(requested) = requested else {
        return Ok(concept_type.default_status());
    };

    concept_type
        .status(requested)
        .ok_or_else(|| ArkoudaError::InvalidStatus {
            status: requested.to_owned(),
            concept_type: concept_type.okf_type.to_owned(),
            valid: concept_type.status_list(),
        })
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
/// then the producer extensions this type scaffolds.
fn render_frontmatter(
    concept_type: &ConceptType,
    title: &str,
    description: &str,
    status: &Status,
    timestamp: &str,
) -> String {
    let mut frontmatter = Mapping::new();
    let mut set = |key: &str, value: Value| {
        frontmatter.insert(Value::from(key), value);
    };

    set("type", Value::from(concept_type.okf_type.as_str()));
    set("title", Value::from(title));
    set("description", Value::from(description));
    set("tags", Value::Sequence(Vec::new()));
    set("timestamp", Value::from(timestamp));
    set("status", Value::from(status.name.as_str()));
    for extension in &concept_type.template_extensions {
        set(extension.as_str(), Value::Sequence(Vec::new()));
    }

    serde_yaml::to_string(&Value::Mapping(frontmatter))
        .expect("frontmatter serialization is infallible for static fields")
}

fn render_template(
    concept_type: &ConceptType,
    title: &str,
    description: &str,
    status: &Status,
    timestamp: &str,
) -> String {
    let yaml = render_frontmatter(concept_type, title, description, status, timestamp);

    // `## Status` is common to every type and its body is the status label, so
    // it is rendered here rather than listed as a template section.
    let mut out = format!(
        "---\n{yaml}---\n\n# {title}\n\n## Status\n\n{}\n",
        status.label
    );
    for section in &concept_type.template_sections {
        out.push_str(&format!("\n## {}\n\n{}\n", section.heading, section.body));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Command;
    use crate::concept::types;
    use crate::concept::{Manifest, validator};
    use clap::Parser;

    fn temp_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("arkouda-new-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create dir");
        root
    }

    /// Parse a `new` invocation and run it against `dir`.
    fn run_new(dir: &Path, extra: &[&str]) -> Result<i32> {
        let mut argv = vec![
            "arkouda",
            "--quiet",
            "--dir",
            dir.to_str().expect("utf-8 path"),
            "new",
        ];
        argv.extend_from_slice(extra);
        let cli = Cli::parse_from(argv);
        let Command::New(args) = &cli.command else {
            unreachable!("parsed a `new` invocation")
        };
        run(args, &cli).map(|outcome| outcome.exit)
    }

    #[test]
    fn a_failed_index_refresh_does_not_fail_the_creation() {
        let root = temp_dir("broken-sibling");
        // An unrelated malformed concept makes `refresh_index` fail, because
        // refreshing re-parses every concept in the bundle.
        std::fs::write(root.join("broken.md"), "no frontmatter here\n").expect("write");
        std::fs::write(root.join("index.md"), "---\nokf_version: \"0.1\"\n---\n").expect("write");

        let exit = run_new(&root, &["Use Postgres"]).expect("creation must not error");

        assert_eq!(
            exit, 0,
            "the concept was created; the exit code must say so"
        );
        assert!(root.join("use-postgres.md").is_file());

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn creation_refreshes_an_existing_index() {
        let root = temp_dir("refresh");
        std::fs::write(root.join("index.md"), "---\nokf_version: \"0.1\"\n---\n").expect("write");

        assert_eq!(run_new(&root, &["Use Postgres"]).expect("create"), 0);

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

        assert_eq!(run_new(&root, &["Use Postgres"]).expect("create"), 0);

        assert!(
            !root.join("index.md").exists(),
            "OKF makes the index optional; `new` must not create one unasked"
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_status_from_another_types_vocabulary_is_rejected() {
        let root = temp_dir("wrong-status");

        let error = run_new(&root, &["Bulk Import", "--status", "shipped"])
            .expect_err("`shipped` is a PRD status");
        assert!(error.to_string().contains("proposed"), "{error}");
        assert!(
            !root.join("bulk-import.md").exists(),
            "nothing is written when the status is invalid"
        );

        assert_eq!(
            run_new(
                &root,
                &["Bulk Import", "--type", "prd", "--status", "shipped"]
            )
            .expect("valid for a PRD"),
            0
        );

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// Every built-in type's template must parse and validate, or `new`
    /// scaffolds a document that `check` immediately rejects.
    #[test]
    fn every_template_validates_and_declares_its_okf_type() {
        for concept_type in types::all() {
            let rendered = render_template(
                concept_type,
                "Use Postgres",
                "Store relational data in Postgres.",
                concept_type.default_status(),
                "2026-05-06",
            );

            let manifest = Manifest::parse_content(
                Path::new("docs/adr/use-postgres.md"),
                Path::new("docs/adr"),
                &rendered,
            )
            .expect("template parses");

            assert_eq!(
                manifest.frontmatter.concept_type.as_deref(),
                Some(concept_type.okf_type.as_str())
            );
            assert_eq!(manifest.concept_id, "use-postgres");

            let result = validator::validate(&manifest);
            assert!(
                result.errors.is_empty(),
                "{}: {:#?}",
                concept_type.slug,
                result.errors
            );
        }
    }

    #[test]
    fn the_prd_template_scaffolds_its_optional_sections_and_extensions() {
        let prd = types::by_slug("prd").expect("built in");
        let rendered = render_template(
            prd,
            "Bulk ADR Import",
            "Import a directory of loose Markdown files.",
            prd.default_status(),
            "2026-08-07",
        );

        assert!(rendered.contains("type: Product Requirements Document"));
        assert!(rendered.contains("status: draft"));
        assert!(rendered.contains("owner: []"));
        assert!(rendered.contains("decisions: []"));
        assert!(rendered.contains("## Status\n\nDraft\n"));
        // Prompted for but not validated, so writing them is the default
        // without being the price of entry.
        assert!(rendered.contains("## Approach"));
        assert!(rendered.contains("## Open Questions"));
        assert!(
            !rendered.contains("## Context"),
            "`Context` is a decision record's word; a PRD has a Problem"
        );
    }
}
