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
//!
//! `[[types]]` tables declare concept types beyond the built-in ADR and PRD:
//!
//! ```toml
//! [[types]]
//! slug = "rfc"
//! okf_type = "Request for Comments"
//! statuses = ["draft", "active", "withdrawn"]
//! required_sections = ["Status", "Summary", "Motivation"]
//! primary_section = "Summary"
//! default_dir = "docs/rfc"
//! template = "docs/templates/rfc.md"
//! ```
//!
//! [`install_types`] resolves them into the process-wide type registry, and
//! must run before `dirs` is resolved: a `[dirs]` key is validated against the
//! registry, so `rfc = [...]` is only meaningful once `rfc` is a type.

use crate::concept::manifest::{ManifestError, split_content};
use crate::concept::markdown;
use crate::concept::types::{self, ConceptType, Origin, Status, TemplateSection};
use crate::concept::{is_valid_id, markdown::SECTION_LEVEL};
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
    /// `[[types]]` tables. Empty when the project declares none, in which case
    /// the registry is exactly the built-ins.
    #[serde(default)]
    types: Vec<TypeDef>,
}

/// One `[[types]]` table, before validation.
///
/// Unknown keys are rejected here, unlike in concept frontmatter: a misspelled
/// config key is a mistake with no reading under which it means something, and
/// silently ignoring it would leave the type subtly wrong.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TypeDef {
    /// CLI slug (`--type rfc`).
    slug: String,
    /// The OKF `type` string documents of this kind declare.
    okf_type: String,
    /// Status vocabulary in lifecycle order. The first entry is what
    /// `arkouda new` starts a document in.
    statuses: Vec<String>,
    /// Sections `arkouda check` requires. Optional: a type with none gets a
    /// status vocabulary and a template but no body contract.
    #[serde(default)]
    required_sections: Vec<String>,
    /// Section `arkouda section <id>` prints when given no name.
    #[serde(default)]
    primary_section: Option<String>,
    /// Directory `arkouda new` writes into when `[dirs]` does not say.
    default_dir: String,
    /// Markdown file whose `##` headings become the scaffold. Resolved
    /// relative to the config file.
    #[serde(default)]
    template: Option<PathBuf>,
    /// Producer-extension frontmatter keys scaffolded as empty lists.
    #[serde(default)]
    extensions: Vec<String>,
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
            per_type: types::all()
                .iter()
                .map(|concept_type| (concept_type, dirs.clone()))
                .collect(),
        }
    }

    /// Every type in its own default directory.
    fn defaults() -> Self {
        Self {
            per_type: types::all()
                .iter()
                .map(|concept_type| (concept_type, vec![PathBuf::from(&concept_type.default_dir)]))
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

/// Resolve the concept-type registry for this invocation and install it.
///
/// Must run before anything resolves `dirs`, resolves a `--type`, or validates
/// a concept: every one of those reads the registry. Commands that never touch
/// a bundle — `self completions` — skip it, so a malformed config cannot stop
/// a shell from starting.
pub fn install_types(start: &Path) -> Result<()> {
    let registry = match discover_file(start)? {
        None => types::builtins(),
        Some((path, base, text)) => {
            let parsed: ConfigFile = toml::from_str(&text).map_err(|err| ArkoudaError::Config {
                path: path.display().to_string(),
                message: err.to_string(),
            })?;
            build_registry(parsed.types, &base).map_err(|message| ArkoudaError::Config {
                path: path.display().to_string(),
                message,
            })?
        }
    };

    if !types::install(registry) {
        return Err(ArkoudaError::RegistryAlreadyInstalled);
    }
    Ok(())
}

/// Built-ins, with every declared type folded in.
///
/// A declared type whose `slug` or `okf_type` matches a built-in replaces it
/// in place, keeping registry order stable; anything else is appended. Partial
/// override is deliberately not offered: a type is a contract, and one
/// assembled from a built-in plus patches is harder to state than one written
/// out.
fn build_registry(
    defs: Vec<TypeDef>,
    base: &Path,
) -> std::result::Result<Vec<ConceptType>, String> {
    let mut registry = types::builtins();
    let mut declared: Vec<(String, String)> = Vec::new();

    for def in defs {
        let concept_type = def.into_concept_type(base)?;

        for (slug, okf_type) in &declared {
            if *slug == concept_type.slug {
                return Err(format!(
                    "two `[[types]]` tables both declare `slug = \"{slug}\"`"
                ));
            }
            if *okf_type == concept_type.okf_type {
                return Err(format!(
                    "two `[[types]]` tables both declare `okf_type = \"{okf_type}\"`"
                ));
            }
        }
        declared.push((concept_type.slug.clone(), concept_type.okf_type.clone()));

        // One table can match two different built-ins at once — a `slug` of
        // `prd` with the ADR's `okf_type`, say. Every match has to go, or the
        // survivor keeps a slug the declared type has also claimed, and
        // `by_slug`, `slugs`, and `ConceptType::eq` all stop identifying one
        // type. The declared type takes the first match's place so registry
        // order, and so `index.md` heading order, stays put.
        let shadows = |existing: &ConceptType| {
            existing.slug == concept_type.slug || existing.okf_type == concept_type.okf_type
        };
        match registry.iter().position(&shadows) {
            Some(index) => {
                registry.retain(|existing| !shadows(existing));
                registry.insert(index, concept_type);
            }
            None => registry.push(concept_type),
        }
    }

    Ok(registry)
}

impl TypeDef {
    /// Validate one declared type and turn it into a descriptor.
    ///
    /// Every invariant the built-ins hold by construction — a slug that is a
    /// slug, a non-empty lifecycle, a primary section `check` guarantees is
    /// present, a template that scaffolds everything it requires — is enforced
    /// here instead, because a declared type has no unit test behind it.
    fn into_concept_type(self, base: &Path) -> std::result::Result<ConceptType, String> {
        let slug = self.slug.trim().to_owned();
        if !is_valid_id(&slug) {
            return Err(format!(
                "`[[types]]` slug `{slug}` must be lowercase alphanumeric words separated by \
                 single hyphens; it is what `--type` takes"
            ));
        }
        let context = |message: String| format!("`[[types]]` `{slug}`: {message}");

        let okf_type = self.okf_type.trim().to_owned();
        if okf_type.is_empty() {
            return Err(context(
                "`okf_type` must not be empty; it is the `type` its documents declare".to_owned(),
            ));
        }

        let mut statuses = Vec::new();
        for status in &self.statuses {
            let status = status.trim();
            if !is_valid_id(status) {
                return Err(context(format!(
                    "status `{status}` must be lowercase alphanumeric words separated by single \
                     hyphens"
                )));
            }
            if statuses
                .iter()
                .any(|existing: &Status| existing.name == status)
            {
                return Err(context(format!("status `{status}` is declared twice")));
            }
            statuses.push(Status::new(status));
        }
        if statuses.is_empty() {
            return Err(context(
                "`statuses` must name at least one status; the first is what `arkouda new` \
                 starts a document in"
                    .to_owned(),
            ));
        }

        let mut required_sections = Vec::new();
        for section in &self.required_sections {
            let section = section.trim();
            if section.is_empty() {
                return Err(context(
                    "`required_sections` must not contain an empty heading".to_owned(),
                ));
            }
            required_sections.push(section.to_owned());
        }

        let primary_section = match self.primary_section.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(primary) => {
                // `arkouda section <id>` with no name prints this section, so
                // a primary section outside the contract would be a default
                // that `check` never guarantees is there.
                if !required_sections.is_empty()
                    && !required_sections
                        .iter()
                        .any(|required| required.eq_ignore_ascii_case(primary))
                {
                    return Err(context(format!(
                        "`primary_section = \"{primary}\"` is not one of `required_sections`, so \
                         `arkouda section` would default to a section `check` does not require"
                    )));
                }
                Some(primary.to_owned())
            }
        };

        let default_dir = self.default_dir.trim().to_owned();
        if default_dir.is_empty() {
            return Err(context("`default_dir` must not be empty".to_owned()));
        }

        let template_sections = match &self.template {
            Some(template) => {
                let path = if template.is_absolute() {
                    template.clone()
                } else {
                    base.join(template)
                };
                let text = std::fs::read_to_string(&path).map_err(|err| {
                    context(format!("cannot read template {}: {err}", path.display()))
                })?;
                let sections = template_sections(&text);
                for required in &required_sections {
                    if required.eq_ignore_ascii_case(STATUS_SECTION) {
                        continue;
                    }
                    if !sections
                        .iter()
                        .any(|section| section.heading.eq_ignore_ascii_case(required))
                    {
                        return Err(context(format!(
                            "template {} has no `## {required}`, so `arkouda new` would scaffold \
                             a document that fails `arkouda check`",
                            path.display()
                        )));
                    }
                }
                sections
            }
            None => required_sections
                .iter()
                .filter(|required| !required.eq_ignore_ascii_case(STATUS_SECTION))
                .map(|required| TemplateSection {
                    heading: required.clone(),
                    body: format!("TODO: write the {} section.", required.to_lowercase()),
                })
                .collect(),
        };

        let mut template_extensions = Vec::new();
        for extension in &self.extensions {
            let extension = extension.trim();
            if extension.is_empty() {
                return Err(context(
                    "`extensions` must not contain an empty key".to_owned(),
                ));
            }
            template_extensions.push(extension.to_owned());
        }

        Ok(ConceptType {
            slug,
            okf_type,
            statuses,
            required_sections,
            primary_section,
            default_dir,
            template_sections,
            template_extensions,
            origin: Origin::Custom,
        })
    }
}

/// `## Status` is rendered by `arkouda new` from the status itself, so a
/// template neither needs it nor may contribute it.
const STATUS_SECTION: &str = "Status";

/// The `##` sections of a template file, in document order.
///
/// Frontmatter is ignored — `arkouda new` generates frontmatter from the
/// descriptor — and a template without any is read whole.
fn template_sections(text: &str) -> Vec<TemplateSection> {
    let body = match split_content(text) {
        Ok((_, body, _)) => body,
        Err(ManifestError::MissingFrontmatter) => text.to_owned(),
        Err(_) => text.to_owned(),
    };

    markdown::headings(&body)
        .into_iter()
        .filter(|heading| heading.level == SECTION_LEVEL)
        .filter(|heading| !heading.text.eq_ignore_ascii_case(STATUS_SECTION))
        .map(|heading| TemplateSection {
            body: markdown::section(&body, &heading.text).unwrap_or_default(),
            heading: heading.text,
        })
        .collect()
}

/// Discover the nearest `.arkoudarc.toml` at or above `start`, returning its
/// path, the directory relative paths resolve against, and its text.
fn discover_file(start: &Path) -> Result<Option<(PathBuf, PathBuf, String)>> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(FILENAME);
        if candidate.is_file() {
            let text = std::fs::read_to_string(&candidate)?;
            return Ok(Some((candidate, ancestor.to_path_buf(), text)));
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
        roots.insert(concept_type.slug.as_str(), resolve(dirs, base));
    }

    Ok(Dirs {
        per_type: types::all()
            .iter()
            .map(|concept_type| {
                let dirs = roots.remove(concept_type.slug.as_str()).unwrap_or_default();
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

    fn adr() -> &'static ConceptType {
        types::by_slug("adr").expect("built in")
    }

    fn prd() -> &'static ConceptType {
        types::by_slug("prd").expect("built in")
    }

    fn temp_dir(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("arkouda-config-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create dir");
        root
    }

    fn registry(toml: &str) -> std::result::Result<Vec<ConceptType>, String> {
        let parsed: ConfigFile = toml::from_str(toml).map_err(|err| err.to_string())?;
        build_registry(parsed.types, Path::new("/repo"))
    }

    #[test]
    fn the_flat_form_is_shared_by_every_type() {
        let dirs = parsed("dirs = [\"docs/adr\", \"services/foo/adr\"]\n");
        let expected = vec![
            PathBuf::from("/repo/docs/adr"),
            PathBuf::from("/repo/services/foo/adr"),
        ];
        assert_eq!(dirs.union(), expected);
        assert_eq!(dirs.for_type(adr()), expected);
        assert_eq!(dirs.for_type(prd()), expected);
    }

    #[test]
    fn the_typed_form_gives_each_type_its_own_roots() {
        let dirs = parsed("[dirs]\nadr = [\"docs/adr\"]\nprd = [\"docs/prd\"]\n");
        assert_eq!(dirs.for_type(adr()), [PathBuf::from("/repo/docs/adr")]);
        assert_eq!(dirs.for_type(prd()), [PathBuf::from("/repo/docs/prd")]);
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
        assert_eq!(dirs.for_type(adr()), [PathBuf::from("/repo/docs/adr")]);
        assert!(
            dirs.for_type(prd()).is_empty(),
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
        assert_eq!(defaults.for_type(adr()), [PathBuf::from("docs/adr")]);
        assert_eq!(defaults.for_type(prd()), [PathBuf::from("docs/prd")]);
    }

    #[test]
    fn the_cli_override_replaces_every_root() {
        let dirs = effective_dirs(Some(Path::new("/tmp/x")), Path::new("/repo")).expect("ok");
        assert_eq!(dirs.union(), vec![PathBuf::from("/tmp/x")]);
        assert_eq!(dirs.for_type(prd()), [PathBuf::from("/tmp/x")]);
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
    fn declared_types_extend_the_builtins() {
        let registry = registry(
            "[[types]]\n\
             slug = \"rfc\"\n\
             okf_type = \"Request for Comments\"\n\
             statuses = [\"draft\", \"active\", \"withdrawn\"]\n\
             required_sections = [\"Status\", \"Summary\"]\n\
             primary_section = \"Summary\"\n\
             default_dir = \"docs/rfc\"\n",
        )
        .expect("valid");

        let slugs: Vec<&str> = registry.iter().map(|t| t.slug.as_str()).collect();
        assert_eq!(slugs, ["adr", "prd", "rfc"], "built-ins keep their place");

        let rfc = registry.last().expect("declared");
        assert_eq!(rfc.origin, Origin::Custom);
        assert_eq!(rfc.default_status().name, "draft");
        assert_eq!(
            rfc.status("active").map(|s| s.label.as_str()),
            Some("Active")
        );
        assert_eq!(rfc.primary_section.as_deref(), Some("Summary"));
    }

    #[test]
    fn a_declared_type_shadows_a_built_in_wholesale() {
        // Replacing in place rather than appending keeps registry order — and
        // so `index.md` heading order — stable.
        let registry = registry(
            "[[types]]\n\
             slug = \"adr\"\n\
             okf_type = \"Architecture Decision Record\"\n\
             statuses = [\"open\", \"closed\"]\n\
             default_dir = \"decisions\"\n",
        )
        .expect("valid");

        let slugs: Vec<&str> = registry.iter().map(|t| t.slug.as_str()).collect();
        assert_eq!(slugs, ["adr", "prd"]);

        let adr = &registry[0];
        assert_eq!(adr.origin, Origin::Custom);
        assert_eq!(adr.status_list(), "open, closed");
        assert!(
            adr.required_sections.is_empty(),
            "shadowing replaces the contract; it does not merge with it"
        );
    }

    #[test]
    fn shadowing_matches_on_the_okf_type_too() {
        // Same documents, different CLI name: the built-in must not survive
        // alongside, or two types would claim the same `type` string.
        let registry = registry(
            "[[types]]\n\
             slug = \"decision\"\n\
             okf_type = \"Architecture Decision Record\"\n\
             statuses = [\"open\"]\n\
             default_dir = \"decisions\"\n",
        )
        .expect("valid");

        let slugs: Vec<&str> = registry.iter().map(|t| t.slug.as_str()).collect();
        assert_eq!(slugs, ["decision", "prd"]);
    }

    #[test]
    fn one_table_may_shadow_two_built_ins_at_once() {
        // `slug` matches the built-in PRD while `okf_type` matches the
        // built-in ADR. Replacing only the first match would leave the built-in
        // PRD behind, and the registry would hold two types with slug `prd` —
        // breaking the uniqueness `by_slug`, `slugs`, and `ConceptType::eq`
        // all rely on.
        let registry = registry(
            "[[types]]\n\
             slug = \"prd\"\n\
             okf_type = \"Architecture Decision Record\"\n\
             statuses = [\"open\"]\n\
             default_dir = \"docs/prd\"\n",
        )
        .expect("valid");

        let slugs: Vec<&str> = registry.iter().map(|t| t.slug.as_str()).collect();
        assert_eq!(slugs, ["prd"], "both shadowed built-ins are gone");

        let okf_types: Vec<&str> = registry.iter().map(|t| t.okf_type.as_str()).collect();
        assert_eq!(okf_types, ["Architecture Decision Record"]);
    }

    #[test]
    fn two_tables_may_not_claim_the_same_slug_or_okf_type() {
        let table = |slug: &str, okf: &str| {
            format!(
                "[[types]]\nslug = \"{slug}\"\nokf_type = \"{okf}\"\n\
                 statuses = [\"draft\"]\ndefault_dir = \"docs/x\"\n"
            )
        };

        let error = registry(&format!("{}{}", table("rfc", "A"), table("rfc", "B")))
            .expect_err("duplicate slug");
        assert!(error.contains("rfc"), "{error}");

        let error = registry(&format!("{}{}", table("one", "Same"), table("two", "Same")))
            .expect_err("duplicate okf_type");
        assert!(error.contains("Same"), "{error}");
    }

    #[test]
    fn a_declared_type_is_validated_at_load() {
        let cases = [
            // A slug that `--type` could not take.
            ("slug = \"RFC 2119\"", "slug"),
            // A lifecycle is what `new` and `index` are built on.
            ("statuses = []", "statuses"),
            // A status that could not be written in frontmatter as declared.
            ("statuses = [\"In Review\"]", "In Review"),
            // A default that `check` never guarantees is present.
            (
                "required_sections = [\"Status\", \"Summary\"]\nprimary_section = \"Rationale\"",
                "primary_section",
            ),
            // A typo with no reading under which it means anything.
            ("requried_sections = [\"Summary\"]", "requried_sections"),
        ];

        for (line, expected) in cases {
            let toml = format!(
                "[[types]]\nslug = \"rfc\"\nokf_type = \"Request for Comments\"\n\
                 statuses = [\"draft\"]\ndefault_dir = \"docs/rfc\"\n{line}\n"
            );
            let error = registry(&toml).expect_err("{line} must not load");
            assert!(
                error.contains(expected),
                "expected `{expected}` in error for `{line}`, got: {error}"
            );
        }
    }

    #[test]
    fn a_type_may_decline_to_require_any_section() {
        // The knob that makes arkouda usable for concepts whose shape is not
        // worth failing a bundle over: a lifecycle without a body contract.
        let registry = registry(
            "[[types]]\n\
             slug = \"note\"\n\
             okf_type = \"Note\"\n\
             statuses = [\"draft\", \"published\"]\n\
             default_dir = \"docs/notes\"\n",
        )
        .expect("valid");

        let note = registry.last().expect("declared");
        assert!(note.required_sections.is_empty());
        assert_eq!(note.primary_section, None);
        assert!(note.template_sections.is_empty());
    }

    #[test]
    fn without_a_template_the_required_sections_are_scaffolded() {
        let registry = registry(
            "[[types]]\n\
             slug = \"rfc\"\n\
             okf_type = \"Request for Comments\"\n\
             statuses = [\"draft\"]\n\
             required_sections = [\"Status\", \"Summary\", \"Motivation\"]\n\
             default_dir = \"docs/rfc\"\n",
        )
        .expect("valid");

        let headings: Vec<&str> = registry
            .last()
            .expect("declared")
            .template_sections
            .iter()
            .map(|section| section.heading.as_str())
            .collect();
        assert_eq!(
            headings,
            ["Summary", "Motivation"],
            "`## Status` is rendered from the status itself, never scaffolded"
        );
    }

    #[test]
    fn a_template_file_supplies_the_scaffold_and_may_exceed_the_contract() {
        let dir = temp_dir("template");
        let template = dir.join("rfc.md");
        std::fs::write(
            &template,
            "# Title\n\n## Status\n\nDraft\n\n## Summary\n\nTODO: one paragraph.\n\n\
             ## Motivation\n\nTODO: why now.\n\n## Open Questions\n\nTODO: list them.\n",
        )
        .expect("write template");

        let parsed: ConfigFile = toml::from_str(&format!(
            "[[types]]\n\
             slug = \"rfc\"\n\
             okf_type = \"Request for Comments\"\n\
             statuses = [\"draft\"]\n\
             required_sections = [\"Status\", \"Summary\", \"Motivation\"]\n\
             default_dir = \"docs/rfc\"\n\
             template = \"{}\"\n",
            template.display()
        ))
        .expect("valid toml");
        let registry = build_registry(parsed.types, &dir).expect("valid");

        let sections = &registry.last().expect("declared").template_sections;
        let headings: Vec<&str> = sections
            .iter()
            .map(|section| section.heading.as_str())
            .collect();
        assert_eq!(
            headings,
            ["Summary", "Motivation", "Open Questions"],
            "the scaffold is a superset of the contract, and never carries Status"
        );
        assert_eq!(sections[0].body, "TODO: one paragraph.");

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_template_that_omits_a_required_section_is_rejected() {
        // Otherwise `arkouda new` would scaffold a document that `arkouda
        // check` rejects the moment it is written.
        let dir = temp_dir("bad-template");
        let template = dir.join("rfc.md");
        std::fs::write(&template, "# Title\n\n## Summary\n\nTODO.\n").expect("write template");

        let parsed: ConfigFile = toml::from_str(&format!(
            "[[types]]\n\
             slug = \"rfc\"\n\
             okf_type = \"Request for Comments\"\n\
             statuses = [\"draft\"]\n\
             required_sections = [\"Summary\", \"Motivation\"]\n\
             default_dir = \"docs/rfc\"\n\
             template = \"{}\"\n",
            template.display()
        ))
        .expect("valid toml");

        let error = build_registry(parsed.types, &dir).expect_err("template is short a section");
        assert!(error.contains("Motivation"), "{error}");

        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_template_may_carry_frontmatter_of_its_own() {
        // `arkouda new` generates frontmatter from the descriptor, so whatever
        // the template declares is ignored rather than merged.
        let sections =
            template_sections("---\ntype: Anything\n---\n\n# Title\n\n## Summary\n\nTODO.\n");
        let headings: Vec<&str> = sections
            .iter()
            .map(|section| section.heading.as_str())
            .collect();
        assert_eq!(headings, ["Summary"]);
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
