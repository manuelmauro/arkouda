//! Generate the OKF §6 `index.md` for each configured bundle.

use crate::adr::index;
use crate::cli::Cli;
use crate::error::{ArkoudaError, Result};
use colored::Colorize;
use std::path::Path;

/// Run the index command.
pub fn run(cli: &Cli) -> Result<i32> {
    let dirs = super::effective_dirs(cli)?;

    for bundle in super::load_bundles(&dirs)? {
        // Writing an index from a partial view of the bundle would silently
        // delete every entry we did not load.
        if !bundle.complete {
            return Err(ArkoudaError::PartialBundle {
                path: bundle.root.display().to_string(),
            });
        }

        let path = write(&bundle.root, &index::render(&bundle.manifests))?;
        if !cli.quiet {
            println!(
                "{} Wrote {} ({} concept(s))",
                "✓".green().bold(),
                path.display(),
                bundle.manifests.len()
            );
        }
    }

    Ok(0)
}

/// Write `content` to `root/index.md`, returning the path written.
pub(crate) fn write(root: &Path, content: &str) -> Result<std::path::PathBuf> {
    let path = root.join("index.md");
    std::fs::write(&path, content)?;
    Ok(path)
}
