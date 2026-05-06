//! Create a new ADR from the standard template.

use crate::adr::{AdrStatus, is_valid_id, slugify};
use crate::cli::{Cli, NewArgs};
use crate::error::{ArkoudaError, Result};
use chrono::Local;
use colored::Colorize;
use serde::Serialize;
use std::process::ExitCode;

/// Run the new command.
pub fn run(args: &NewArgs, cli: &Cli) -> Result<ExitCode> {
    let id = match args.id.as_deref() {
        Some(explicit) => explicit.to_owned(),
        None => slugify(&args.title),
    };
    if !is_valid_id(&id) {
        return Err(ArkoudaError::InvalidId(id));
    }

    std::fs::create_dir_all(&cli.dir)?;
    let path = cli.dir.join(format!("{id}.md"));
    if path.exists() {
        return Err(ArkoudaError::AdrExists {
            id,
            path: path.display().to_string(),
        });
    }

    let date = Local::now().date_naive().format("%Y-%m-%d").to_string();
    let abstract_text = args
        .abstract_text
        .as_deref()
        .unwrap_or("TODO: summarize the decision in one or two sentences.");
    let content = render_template(&id, &args.title, abstract_text, args.status, &date);

    std::fs::write(&path, content)?;

    if !cli.quiet {
        println!(
            "{} Created ADR '{}' at {}",
            "✓".green().bold(),
            id,
            path.display()
        );
    }

    Ok(ExitCode::SUCCESS)
}

#[derive(Serialize)]
struct TemplateFrontmatter<'a> {
    id: &'a str,
    title: &'a str,
    #[serde(rename = "abstract")]
    abstract_text: &'a str,
    status: AdrStatus,
    date: &'a str,
    deciders: &'a [String],
    tags: &'a [String],
}

fn render_template(
    id: &str,
    title: &str,
    abstract_text: &str,
    status: AdrStatus,
    date: &str,
) -> String {
    let frontmatter = TemplateFrontmatter {
        id,
        title,
        abstract_text,
        status,
        date,
        deciders: &[],
        tags: &[],
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
