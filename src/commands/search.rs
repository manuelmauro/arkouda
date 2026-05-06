//! Search ADRs.

use crate::adr::Manifest;
use crate::cli::{Cli, SearchArgs};
use crate::error::Result;
use std::process::ExitCode;

/// Run the search command.
pub fn run(args: &SearchArgs, cli: &Cli) -> Result<ExitCode> {
    let manifests = super::load_manifests(&cli.dir)?;
    let query = args.query.to_ascii_lowercase();
    let matches: Vec<Manifest> = manifests
        .into_iter()
        .filter(|manifest| searchable_text(manifest).contains(&query))
        .collect();

    if matches.is_empty() {
        if !cli.quiet {
            println!("No ADRs matched '{}'.", args.query);
        }
    } else {
        super::list::print_manifest_rows(&matches);
    }

    Ok(ExitCode::SUCCESS)
}

fn searchable_text(manifest: &Manifest) -> String {
    let frontmatter = &manifest.frontmatter;
    let mut text = String::new();
    for value in [
        frontmatter.id.as_deref(),
        frontmatter.title.as_deref(),
        frontmatter.abstract_text.as_deref(),
        frontmatter.status.as_deref(),
        frontmatter.date.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        text.push_str(value);
        text.push('\n');
    }
    for tag in &frontmatter.tags {
        text.push_str(tag);
        text.push('\n');
    }
    text.push_str(&manifest.body);
    text.make_ascii_lowercase();
    text
}
