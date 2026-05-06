//! Show one ADR.

use crate::cli::{Cli, ShowArgs};
use crate::error::{ArkoudaError, Result};

/// Run the show command.
pub fn run(args: ShowArgs, cli: &Cli) -> Result<i32> {
    let manifests = super::load_manifests(&cli.dir)?;
    let matches = manifests
        .iter()
        .filter(|manifest| super::matches_lookup(manifest, &args.id))
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(ArkoudaError::AdrNotFound(args.id)),
        [manifest] => {
            let content = std::fs::read_to_string(&manifest.path)?;
            print!("{content}");
            Ok(0)
        }
        _ => Err(ArkoudaError::AmbiguousAdr {
            query: args.id,
            count: matches.len(),
        }),
    }
}
