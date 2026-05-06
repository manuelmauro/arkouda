//! CLI command implementations.

use crate::adr::{Discovery, Manifest};
use crate::error::{ArkoudaError, Result};
use std::path::Path;

pub mod check;
pub mod list;
pub mod new;
pub mod search;
pub mod show;

pub(crate) fn discover_paths(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let paths = Discovery::find_adrs(dir)?;
    if paths.is_empty() {
        return Err(ArkoudaError::NoAdrsFound {
            path: dir.display().to_string(),
        });
    }
    Ok(paths)
}

pub(crate) fn load_manifests(dir: &Path) -> Result<Vec<Manifest>> {
    discover_paths(dir)?
        .into_iter()
        .map(|path| {
            Manifest::parse(path.clone()).map_err(|source| ArkoudaError::Manifest {
                path: path.display().to_string(),
                source,
            })
        })
        .collect()
}

pub(crate) fn matches_lookup(manifest: &Manifest, query: &str) -> bool {
    let file_name = manifest
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let file_stem = manifest
        .path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    manifest.frontmatter.id.as_deref() == Some(query) || file_stem == query || file_name == query
}
