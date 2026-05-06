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

    match matches.as_slice() {
        [] => Err(ArkoudaError::AdrNotFound(args.id.clone())),
        [manifest] => {
            let content = std::fs::read_to_string(&manifest.path)?;
            print!("{content}");
            Ok(ExitCode::SUCCESS)
        }
        _ => Err(ArkoudaError::AmbiguousAdr {
            query: args.id.clone(),
            count: matches.len(),
        }),
    }
}
