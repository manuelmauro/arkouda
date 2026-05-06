//! Show one ADR.

use crate::cli::{Cli, ShowArgs};
use crate::error::{ArkoudaError, Result};
use std::process::ExitCode;

/// Run the show command.
pub fn run(args: &ShowArgs, cli: &Cli) -> Result<ExitCode> {
    let manifests = super::load_manifests(&cli.dir)?;
    let matches: Vec<_> = manifests
        .iter()
        .filter(|manifest| super::matches_lookup(manifest, &args.id))
        .collect();

    let manifest = match matches.as_slice() {
        [] => return Err(ArkoudaError::AdrNotFound(args.id.clone())),
        [manifest] => *manifest,
        _ => {
            return Err(ArkoudaError::AmbiguousAdr {
                query: args.id.clone(),
                count: matches.len(),
            });
        }
    };

    if let Some(section) = args.section.as_deref() {
        let body = manifest
            .section(section)
            .ok_or_else(|| ArkoudaError::SectionNotFound {
                id: manifest.frontmatter.display_id().to_owned(),
                section: section.to_owned(),
            })?;
        println!("{body}");
    } else {
        let content = std::fs::read_to_string(&manifest.path)?;
        print!("{content}");
    }

    Ok(ExitCode::SUCCESS)
}
