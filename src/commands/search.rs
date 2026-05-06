//! Search ADRs.

use crate::adr::Manifest;
use crate::cli::{Cli, SearchArgs};
use crate::error::Result;

/// Run the search command.
pub fn run(args: SearchArgs, cli: &Cli) -> Result<i32> {
    let manifests = super::load_manifests(&cli.dir)?;
    let query = args.query.to_ascii_lowercase();
    let matches = manifests
        .into_iter()
        .filter(|manifest| searchable_text(manifest).contains(&query))
        .collect::<Vec<_>>();

    if matches.is_empty() {
        if !cli.quiet {
            println!("No ADRs matched '{}'.", args.query);
        }
    } else {
        super::list::print_manifest_rows(&matches);
    }

    Ok(0)
}

fn searchable_text(manifest: &Manifest) -> String {
    let frontmatter = &manifest.frontmatter;
    let mut fields = [
        frontmatter.id.as_deref().unwrap_or_default(),
        frontmatter.title.as_deref().unwrap_or_default(),
        frontmatter.abstract_text.as_deref().unwrap_or_default(),
        frontmatter.status.as_deref().unwrap_or_default(),
        frontmatter.date.as_deref().unwrap_or_default(),
        &manifest.body,
    ]
    .join("\n");

    if let Some(tags) = &frontmatter.tags {
        fields.push('\n');
        fields.push_str(&tags.join("\n"));
    }

    fields.to_ascii_lowercase()
}
