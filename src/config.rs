//! Configuration loaded from `.arkoudarc.toml`.
//!
//! Discovery walks up from the starting directory (typically `cwd`) until it
//! finds a `.arkoudarc.toml` or hits the filesystem root. Relative paths in
//! `dirs` are resolved against the directory containing the config file, so
//! the same config works regardless of which subdirectory arkouda is invoked
//! from.
//!
//! `dirs` takes two forms. The flat form is a list every concept type shares:
//!
//! ```toml
//! dirs = ["docs/adr"]
//! ```
//!
//! The typed form gives each type its own roots:
//!
//! ```toml
//! [dirs]
//! adr = ["docs/adr"]
//! prd = ["docs/prd"]
//! ```
//!
//! Both parse into one [`Dirs`] model: a type-to-roots map, plus the union
//! that `list`, `check`, and `section` search. The typed table is a default
//! write target and a search scope, never a schema — a concept's type comes
//! from its frontmatter, so any bundle may hold any mix.

use crate::concept::types::{self, ConceptType};
use crate::error::{ArkoudaError, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FILENAME: &str = ".arkoudarc.toml";

#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    /// Left as a raw value so the two accepted shapes can be told apart with a
    /// message that names what is wrong. An untagged enum reports only that
    /// nothing matched, which is no help at all when the mistake is one stray
    /// key in the table.
    #[serde(default)]
    dirs: Option<toml::Value>,
    #[serde(default)]
    telemetry: Option<bool>,
}

/// Bundle roots, per concept type.
///
/// Every known type has an entry, possibly empty. Search commands use
/// [`Dirs::union`]; `new` writes into the first root of the resolved type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    per_type: Vec<(&'static ConceptType, Vec<PathBuf>)>,
}

impl Dirs {
    /// Every root, in type order, with duplicates removed. This is the search
    /// scope: which type a concept is comes from its frontmatter, so every
    /// root has to be read whatever type the caller cares about.
    pub fn union(&self) -> Vec<PathBuf> {
        let mut union: Vec<PathBuf> = Vec::new();
        for (_, dirs) in &self.per_type {
            for dir in dirs {
                if !union.contains(dir) {
                    union.push(dir.clone());
                }
            }
        }
        union
    }

    /// The roots configured for one type, in order. Empty when a typed `dirs`
    /// table does not mention it.
    pub fn for_type(&self, concept_type: &ConceptType) -> &[PathBuf] {
        self.per_type
            .iter()
            .find(|(candidate, _)| *candidate == concept_type)
            .map_or(&[], |(_, dirs)| dirs.as_slice())
    }

    /// Give every known type the same roots.
    fn shared(dirs: Vec<PathBuf>) -> Self {
        Self {
            per_type: types::ALL
                .iter()
                .map(|concept_type| (concept_type, dirs.clone()))
                .collect(),
        }
    }

    /// Every type in its own default directory.
    fn defaults() -> Self {
        Self {
            per_type: types::ALL
                .iter()
                .map(|concept_type| (concept_type, vec![PathBuf::from(concept_type.default_dir)]))
                .collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.per_type.iter().all(|(_, dirs)| dirs.is_empty())
    }
}

/// Effective bundle roots for this invocation, given CLI overrides and any
/// `.arkoudarc.toml` discovered up the tree from `start`.
///
/// Precedence: explicit `cli_dir` > `.arkoudarc.toml` `dirs` > each type's
/// default directory. A `dirs` entry that resolves to nothing counts as
/// unconfigured.
pub fn effective_dirs(cli_dir: Option<&Path>, start: &Path) -> Result<Dirs> {
    if let Some(dir) = cli_dir {
        return Ok(Dirs::shared(vec![dir.to_path_buf()]));
    }
    if let Some(dirs) = discover(start)?
        && !dirs.is_empty()
    {
        return Ok(dirs);
    }
    Ok(Dirs::defaults())
}

fn discover(start: &Path) -> Result<Option<Dirs>> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)?;
            return Ok(Some(parse(&text, ancestor).map_err(|message| {
                ArkoudaError::Config {
                    path: candidate.display().to_string(),
                    message,
                }
            })?));
        }
    }
    Ok(None)
}

/// Telemetry toggle from `.arkoudarc.toml` discovered above `start`. Returns
/// `Ok(None)` when there is no config or no `telemetry` key — telemetry
/// resolution should then fall back to its default.
pub fn telemetry_from_config(start: &Path) -> Result<Option<bool>> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)?;
            let parsed: ConfigFile = toml::from_str(&text).map_err(|err| ArkoudaError::Config {
                path: candidate.display().to_string(),
                message: err.to_string(),
            })?;
            return Ok(parsed.telemetry);
        }
    }
    Ok(None)
}

fn parse(text: &str, base: &Path) -> std::result::Result<Dirs, String> {
    let parsed: ConfigFile = toml::from_str(text).map_err(|err| err.to_string())?;

    match parsed.dirs {
        None => Ok(Dirs {
            per_type: Vec::new(),
        }),
        Some(toml::Value::Array(items)) => {
            let dirs: Vec<PathBuf> = toml::Value::Array(items)
                .try_into()
                .map_err(|err| format!("`dirs`: {err}"))?;
            Ok(Dirs::shared(resolve(dirs, base)))
        }
        Some(toml::Value::Table(table)) => typed(table, base),
        Some(_) => Err(
            "`dirs` must be a list of paths (`dirs = [\"docs/adr\"]`) or a table of per-type \
             lists (`[dirs]` with `adr = [...]`)"
                .to_owned(),
        ),
    }
}

/// `[dirs]` with one key per type slug.
fn typed(
    table: toml::map::Map<String, toml::Value>,
    base: &Path,
) -> std::result::Result<Dirs, String> {
    let mut roots: BTreeMap<&'static str, Vec<PathBuf>> = BTreeMap::new();

    for (key, value) in table {
        // A key that is not a type slug is a typo, and silently searching
        // nowhere is the worst possible response to one.
        let Some(concept_type) = types::by_slug(&key) else {
            return Err(format!(
                "`dirs` has no concept type `{key}`; known types are {}",
                types::slugs().join(", ")
            ));
        };
        let dirs: Vec<PathBuf> = value
            .try_into()
            .map_err(|err| format!("`dirs.{key}` must be a list of paths: {err}"))?;
        roots.insert(concept_type.slug, resolve(dirs, base));
    }

    Ok(Dirs {
        per_type: types::ALL
            .iter()
            .map(|concept_type| {
                let dirs = roots.remove(concept_type.slug).unwrap_or_default();
                (concept_type, dirs)
            })
            .collect(),
    })
}

fn resolve(dirs: Vec<PathBuf>, base: &Path) -> Vec<PathBuf> {
    dirs.into_iter()
        .map(|dir| {
            if dir.is_absolute() {
                dir
            } else {
                base.join(dir)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(text: &str) -> Dirs {
        parse(text, Path::new("/repo")).expect("ok")
    }

    #[test]
    fn the_flat_form_is_shared_by_every_type() {
        let dirs = parsed("dirs = [\"docs/adr\", \"services/foo/adr\"]\n");
        let expected = vec![
            PathBuf::from("/repo/docs/adr"),
            PathBuf::from("/repo/services/foo/adr"),
        ];
        assert_eq!(dirs.union(), expected);
        assert_eq!(dirs.for_type(&types::ADR), expected);
        assert_eq!(dirs.for_type(&types::PRD), expected);
    }

    #[test]
    fn the_typed_form_gives_each_type_its_own_roots() {
        let dirs = parsed("[dirs]\nadr = [\"docs/adr\"]\nprd = [\"docs/prd\"]\n");
        assert_eq!(
            dirs.for_type(&types::ADR),
            [PathBuf::from("/repo/docs/adr")]
        );
        assert_eq!(
            dirs.for_type(&types::PRD),
            [PathBuf::from("/repo/docs/prd")]
        );
        assert_eq!(
            dirs.union(),
            vec![
                PathBuf::from("/repo/docs/adr"),
                PathBuf::from("/repo/docs/prd"),
            ],
            "the union is what list, check, and section search"
        );
    }

    #[test]
    fn a_typed_table_may_mention_one_type_only() {
        let dirs = parsed("[dirs]\nadr = [\"docs/adr\"]\n");
        assert_eq!(
            dirs.for_type(&types::ADR),
            [PathBuf::from("/repo/docs/adr")]
        );
        assert!(
            dirs.for_type(&types::PRD).is_empty(),
            "an explicit table is a complete declaration; arkouda must not \
             invent a root the project never named"
        );
        assert_eq!(dirs.union(), vec![PathBuf::from("/repo/docs/adr")]);
    }

    #[test]
    fn an_unknown_type_key_is_an_error() {
        let error = parse("[dirs]\nrfc = [\"docs/rfc\"]\n", Path::new("/repo"))
            .expect_err("a typo must not silently search nowhere");
        assert!(error.contains("rfc"), "{error}");
    }

    #[test]
    fn the_union_deduplicates_shared_roots() {
        let dirs = parsed("[dirs]\nadr = [\"docs\"]\nprd = [\"docs\"]\n");
        assert_eq!(dirs.union(), vec![PathBuf::from("/repo/docs")]);
    }

    #[test]
    fn keeps_absolute_paths_as_is() {
        assert_eq!(
            parsed("dirs = [\"/abs/adr\"]\n").union(),
            vec![PathBuf::from("/abs/adr")]
        );
    }

    #[test]
    fn an_empty_or_missing_dirs_key_falls_back_to_the_defaults() {
        for text in ["dirs = []\n", "[dirs]\n", ""] {
            assert!(parsed(text).is_empty(), "{text:?}");
        }

        let defaults = effective_dirs(None, Path::new("/nonexistent/repo")).expect("ok");
        assert_eq!(defaults.for_type(&types::ADR), [PathBuf::from("docs/adr")]);
        assert_eq!(defaults.for_type(&types::PRD), [PathBuf::from("docs/prd")]);
    }

    #[test]
    fn the_cli_override_replaces_every_root() {
        let dirs = effective_dirs(Some(Path::new("/tmp/x")), Path::new("/repo")).expect("ok");
        assert_eq!(dirs.union(), vec![PathBuf::from("/tmp/x")]);
        assert_eq!(dirs.for_type(&types::PRD), [PathBuf::from("/tmp/x")]);
    }

    #[test]
    fn malformed_toml_is_an_error() {
        assert!(parse("dirs = not-a-list\n", Path::new("/repo")).is_err());
    }

    #[test]
    fn a_malformed_dirs_value_says_what_is_wrong() {
        // The classic mistake is a stray key after `[dirs]`, which TOML reads
        // as part of the table. Naming it beats "no variant matched".
        let error = parse(
            "[dirs]\nadr = [\"docs/adr\"]\ntelemetry = false\n",
            Path::new("/repo"),
        )
        .expect_err("`telemetry` is not a concept type");
        assert!(error.contains("telemetry"), "{error}");

        let error = parse("[dirs]\nadr = \"docs/adr\"\n", Path::new("/repo"))
            .expect_err("a type's roots are a list");
        assert!(error.contains("dirs.adr"), "{error}");

        let error =
            parse("dirs = 3\n", Path::new("/repo")).expect_err("neither a list nor a table");
        assert!(error.contains("list of paths"), "{error}");
    }

    #[test]
    fn telemetry_key_is_round_tripped() {
        let parsed: ConfigFile = toml::from_str("telemetry = false\n").unwrap();
        assert_eq!(parsed.telemetry, Some(false));
        let parsed: ConfigFile = toml::from_str("telemetry = true\n").unwrap();
        assert_eq!(parsed.telemetry, Some(true));
        let parsed: ConfigFile = toml::from_str("dirs = [\"docs/adr\"]\n").unwrap();
        assert_eq!(parsed.telemetry, None);
    }
}
