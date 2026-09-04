//! CLI command implementations.

use crate::cli::Cli;
use crate::concept::DiagnosticCode;
use crate::concept::types::{self, ConceptType, Origin};
use crate::concept::{Manifest, discovery};
use crate::config::{self, Dirs};
use crate::error::{ArkoudaError, Result};
use std::path::{Path, PathBuf};

pub mod check;
pub mod completions;
pub mod index;
pub mod list;
pub mod new;
pub mod section;

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

/// What a command did, beyond its exit code.
///
/// Carries the two facts telemetry records that an exit code cannot: which
/// diagnostics a `check` produced, and whether a resolved `--type` was built
/// in or declared by the project. Commands that have neither convert from
/// their exit code.
pub struct Outcome {
    /// Process exit code: 0 success, 1 failure.
    pub exit: i32,
    /// Diagnostic codes produced, sorted and deduplicated.
    pub codes: Vec<DiagnosticCode>,
    /// Origin of the type this invocation resolved, when it resolved one.
    pub type_kind: Option<Origin>,
}

impl From<i32> for Outcome {
    fn from(exit: i32) -> Self {
        Self {
            exit,
            codes: Vec::new(),
            type_kind: None,
        }
    }
}

/// Resolve a `--type` slug against the configured registry.
///
/// `--type` is a free string rather than a clap `PossibleValuesParser`,
/// because which slugs are valid depends on a `.arkoudarc.toml` that has not
/// been read when clap parses argv.
pub(crate) fn resolve_type(slug: &str) -> Result<&'static ConceptType> {
    types::by_slug(slug).ok_or_else(|| ArkoudaError::UnknownType {
        slug: slug.to_owned(),
        known: types::slugs().join(", "),
    })
}

/// Resolve the effective bundle roots for this invocation: CLI flag wins,
/// then `.arkoudarc.toml`, then each type's default directory.
pub(crate) fn effective_dirs(cli: &Cli) -> Result<Dirs> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    config::effective_dirs(cli.dir.as_deref(), &cwd)
}

/// The roots `list`, `check`, `section`, and `index` read: every configured
/// directory, whatever type it was configured for. A concept's type is a fact
/// about its frontmatter, not about where it sits, so nothing may be skipped
/// on the strength of a directory alone.
pub(crate) fn search_dirs(cli: &Cli) -> Result<Vec<PathBuf>> {
    Ok(effective_dirs(cli)?.union())
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
        return Err(ArkoudaError::NoConceptsFound {
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

/// The directory `new` writes a concept of this type into: the first root
/// configured for it.
pub(crate) fn write_dir<'a>(dirs: &'a Dirs, concept_type: &ConceptType) -> Result<&'a Path> {
    dirs.for_type(concept_type)
        .first()
        .map(PathBuf::as_path)
        .ok_or_else(|| ArkoudaError::NoDirForType {
            slug: concept_type.slug.to_owned(),
        })
}

/// Find the one concept `query` names, by concept id, filename stem, or
/// filename.
pub(crate) fn resolve_one<'a>(manifests: &'a [Manifest], query: &str) -> Result<&'a Manifest> {
    let matches: Vec<&Manifest> = manifests
        .iter()
        .filter(|manifest| matches_lookup(manifest, query))
        .collect();

    match matches.as_slice() {
        [] => Err(ArkoudaError::ConceptNotFound(query.to_owned())),
        [manifest] => Ok(manifest),
        _ => Err(ArkoudaError::AmbiguousConcept {
            query: query.to_owned(),
            count: matches.len(),
        }),
    }
}
