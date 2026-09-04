//! CLI command implementations.

use crate::cli::Cli;
use crate::concept::DiagnosticCode;
use crate::concept::types::{self, ConceptType, Origin};
use crate::concept::{Manifest, discovery};
use crate::config;
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

/// Where discovery starts: `--dir` when given, else the working directory.
///
/// `--dir` narrows the walk to a subtree rather than declaring a root (ADR
/// `discover-bundles`), which is what keeps "check one bundle in CI" working
/// now that nothing configures roots.
fn discovery_root(cli: &Cli) -> PathBuf {
    cli.dir.as_deref().map(Path::to_path_buf).unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    })
}

/// The bundles `list`, `check`, `section`, and `index` read.
///
/// Found by walking, not configured. A concept's type is a fact about its
/// frontmatter and not about where it sits, so every bundle is read whatever
/// type the caller cares about — the same reason the old `dirs` union existed.
pub(crate) fn search_dirs(cli: &Cli) -> Result<Vec<PathBuf>> {
    let start = discovery_root(cli);
    config::check_config(&start)?;
    Ok(discovery::find_bundles(&start)?)
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

/// The directory `new` writes a concept of this type into.
///
/// Three steps, in order:
///
///  1. the first discovered bundle that already holds a concept of this type;
///  2. the `--dir` the caller named, when they named one;
///  3. the type's `default_dir`, relative to where discovery started.
///
/// The first matters because a project whose ADRs live in `knowledge/decisions`
/// would otherwise have them found by `check` and then have `new` start a
/// second bundle in `docs/adr`. The second keeps `--dir` meaning what it always
/// did for `new`: put it there. The third has to be joined onto the discovery
/// root rather than used bare — a relative `default_dir` resolved against the
/// process working directory writes into whatever repository the shell happens
/// to be sitting in.
pub(crate) fn write_dir(cli: &Cli, concept_type: &'static ConceptType) -> Result<PathBuf> {
    for bundle in search_dirs(cli)? {
        let holds_type = discovery::find_concepts(&bundle)?
            .iter()
            .filter_map(|path| Manifest::parse(path, &bundle).ok())
            .any(|manifest| manifest.frontmatter.resolved_type() == Some(concept_type));
        if holds_type {
            return Ok(bundle);
        }
    }
    if let Some(dir) = cli.dir.as_deref() {
        return Ok(dir.to_path_buf());
    }
    Ok(discovery_root(cli).join(&concept_type.default_dir))
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
