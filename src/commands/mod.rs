//! CLI command implementations.

use crate::adr::{Manifest, discovery};
use crate::cli::Cli;
use crate::config;
use crate::error::{ArkoudaError, Result};
use std::path::{Path, PathBuf};

pub mod check;
pub mod completions;
pub mod decision;
pub mod list;
pub mod new;

/// Resolve the effective ADR directories for this invocation: CLI flag wins,
/// then `.arkoudarc.toml`, then the default.
pub(crate) fn effective_dirs(cli: &Cli) -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    config::effective_dirs(cli.dir.as_deref(), &cwd)
}

pub(crate) fn discover_paths(dirs: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        paths.extend(discovery::find_adrs(dir)?);
    }
    if paths.is_empty() {
        return Err(ArkoudaError::NoAdrsFound {
            path: format_dirs(dirs),
        });
    }
    Ok(paths)
}

pub(crate) fn load_manifests(dirs: &[PathBuf]) -> Result<Vec<Manifest>> {
    discover_paths(dirs)?
        .into_iter()
        .map(|path| {
            Manifest::parse(&path).map_err(|source| ArkoudaError::Manifest {
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

fn format_dirs(dirs: &[PathBuf]) -> String {
    if dirs.len() == 1 {
        dirs[0].display().to_string()
    } else {
        dirs.iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The first effective directory — used by `new` as the write target.
pub(crate) fn primary_dir(dirs: &[PathBuf]) -> &Path {
    dirs.first()
        .map(PathBuf::as_path)
        .expect("effective_dirs always returns at least one entry")
}
