//! CLI command implementations.

use crate::adr::{Manifest, discovery};
use crate::cli::Cli;
use crate::config;
use crate::error::{ArkoudaError, Result};
use std::path::{Path, PathBuf};

pub mod check;
pub mod completions;
pub mod decision;
pub mod index;
pub mod list;
pub mod new;

/// An OKF knowledge bundle: a root directory and the concepts inside it.
/// Concept ids are relative to the root, so every concept must be loaded
/// alongside the bundle it belongs to.
pub(crate) struct Bundle {
    /// Bundle root directory.
    pub root: PathBuf,
    /// Concepts discovered in the bundle, in path order.
    pub manifests: Vec<Manifest>,
    /// Whether `manifests` covers the whole bundle. False when the caller
    /// pointed arkouda at a single concept file, in which case anything that
    /// reasons about the bundle as a whole — the `index.md`, most of all —
    /// must be left alone.
    pub complete: bool,
}

/// Concept paths discovered under one bundle root.
pub(crate) struct DiscoveredBundle {
    /// Bundle root directory.
    pub root: PathBuf,
    /// Concept paths, in path order.
    pub paths: Vec<PathBuf>,
    /// Whether `paths` covers the whole bundle. See [`Bundle::complete`].
    pub complete: bool,
}

/// Resolve the effective ADR bundle roots for this invocation: CLI flag wins,
/// then `.arkoudarc.toml`, then the default.
pub(crate) fn effective_dirs(cli: &Cli) -> Result<Vec<PathBuf>> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    config::effective_dirs(cli.dir.as_deref(), &cwd)
}

/// The bundle root for a configured directory. When the directory is really a
/// single Markdown file, its parent is the bundle.
pub(crate) fn bundle_root(dir: &Path) -> PathBuf {
    if dir.is_file() {
        dir.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        dir.to_path_buf()
    }
}

/// Discover concept paths per bundle root. Errors when no concept is found
/// anywhere. `check` uses this directly so it can report parse failures as
/// diagnostics rather than aborting the run.
pub(crate) fn discover_bundles(dirs: &[PathBuf]) -> Result<Vec<DiscoveredBundle>> {
    let mut bundles = Vec::new();
    let mut total_concepts = 0;

    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        let paths = discovery::find_concepts(dir)?;
        total_concepts += paths.len();
        bundles.push(DiscoveredBundle {
            root: bundle_root(dir),
            paths,
            complete: !dir.is_file(),
        });
    }

    if total_concepts == 0 {
        return Err(ArkoudaError::NoAdrsFound {
            path: format_dirs(dirs),
        });
    }

    Ok(bundles)
}

/// Load every configured bundle, failing on the first unparseable concept.
pub(crate) fn load_bundles(dirs: &[PathBuf]) -> Result<Vec<Bundle>> {
    discover_bundles(dirs)?
        .into_iter()
        .map(|bundle| {
            let manifests = bundle
                .paths
                .into_iter()
                .map(|path| {
                    Manifest::parse(&path, &bundle.root).map_err(|source| ArkoudaError::Manifest {
                        path: path.display().to_string(),
                        source,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(Bundle {
                root: bundle.root,
                manifests,
                complete: bundle.complete,
            })
        })
        .collect()
}

/// Load every concept across every configured bundle, flattened.
pub(crate) fn load_manifests(dirs: &[PathBuf]) -> Result<Vec<Manifest>> {
    Ok(load_bundles(dirs)?
        .into_iter()
        .flat_map(|bundle| bundle.manifests)
        .collect())
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

    manifest.concept_id == query || file_stem == query || file_name == query
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
