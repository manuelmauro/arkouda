//! Print one ADR's decision (or another named section).

use crate::cli::{Cli, DecisionArgs};
use crate::error::{ArkoudaError, Result};
use std::process::ExitCode;

const DEFAULT_SECTION: &str = "decision";

/// Run the decision command.
pub fn run(args: &DecisionArgs, cli: &Cli) -> Result<ExitCode> {
    let dirs = super::effective_dirs(cli)?;
    let manifests = super::load_manifests(&dirs)?;
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

    let section = args.section.as_deref().unwrap_or(DEFAULT_SECTION);
    let body = manifest
        .section(section)
        .ok_or_else(|| ArkoudaError::SectionNotFound {
            id: manifest.frontmatter.display_id().to_owned(),
            section: section.to_owned(),
        })?;
    println!("{body}");

    Ok(ExitCode::SUCCESS)
}
