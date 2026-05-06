//! Create a new ADR from the standard template.

use crate::adr::{is_valid_id, slugify};
use crate::cli::{Cli, NewArgs};
use crate::error::{ArkoudaError, Result};
use chrono::Local;
use colored::Colorize;

/// Run the new command.
pub fn run(args: NewArgs, cli: &Cli) -> Result<i32> {
    let id = args.id.unwrap_or_else(|| slugify(&args.title));
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
        .unwrap_or_else(|| "TODO: summarize the decision in one or two sentences.".to_string());
    let content = render_template(
        &id,
        &args.title,
        &abstract_text,
        args.status.as_str(),
        &date,
    );

    std::fs::write(&path, content)?;

    if !cli.quiet {
        println!(
            "{} Created ADR '{}' at {}",
            "✓".green().bold(),
            id,
            path.display()
        );
    }

    Ok(0)
}

fn render_template(id: &str, title: &str, abstract_text: &str, status: &str, date: &str) -> String {
    format!(
        "---\nid: \"{}\"\ntitle: \"{}\"\nabstract: \"{}\"\nstatus: \"{}\"\ndate: \"{}\"\ndeciders: []\ntags: []\n---\n\n# {}\n\n## Status\n\n{}\n\n## Context\n\nTODO: describe the forces, constraints, and background for this decision.\n\n## Decision\n\nTODO: describe the decision.\n\n## Consequences\n\nTODO: describe the positive, negative, and neutral consequences.\n",
        yaml_double_quote(id),
        yaml_double_quote(title),
        yaml_double_quote(abstract_text),
        yaml_double_quote(status),
        yaml_double_quote(date),
        title,
        status_label(status),
    )
}

fn yaml_double_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn status_label(status: &str) -> &'static str {
    match status {
        "proposed" => "Proposed",
        "accepted" => "Accepted",
        "superseded" => "Superseded",
        "deprecated" => "Deprecated",
        "rejected" => "Rejected",
        _ => "Proposed",
    }
}
