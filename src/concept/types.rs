//! Concept type descriptors and the registry that holds them.
//!
//! Everything arkouda knows about a kind of document — its OKF `type` string,
//! its status vocabulary, the body sections it must have, the section that
//! carries its substance, where new ones are written, and the template
//! `arkouda new` scaffolds — lives in one [`ConceptType`] value.
//!
//! Architecture Decision Record and Product Requirements Document are built
//! in. A project adds its own with `[[types]]` tables in `.arkoudarc.toml`,
//! which [`crate::config`] parses into these same descriptors and hands to
//! [`install`]. A declared type whose `slug` or `okf_type` matches a built-in
//! replaces it wholesale. See the ADR `support-user-defined-concept-types`.
//!
//! The registry is resolved once at startup and leaked, so descriptors are
//! `&'static` for the rest of the process. One process-lifetime allocation in
//! a short-lived CLI buys `&'static ConceptType` throughout, instead of
//! threading a registry handle through every command.

use std::sync::OnceLock;

/// One status in a concept type's lifecycle vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// Frontmatter value, lowercase and kebab-case (`in-review`).
    pub name: String,
    /// Title-case rendering used in `index.md` headings and in the body's
    /// `## Status` section (`In Review`).
    pub label: String,
}

impl Status {
    /// A status whose label is derived from its kebab-case name.
    ///
    /// Deriving rather than declaring reproduces every built-in label and
    /// keeps a declared vocabulary a plain list of strings.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            label: title_case(name),
        }
    }
}

/// Title-case a kebab-case status name: `in-review` becomes `In Review`.
fn title_case(name: &str) -> String {
    name.split('-')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A body section `arkouda new` scaffolds: its heading and placeholder prose.
///
/// `## Status` is not listed: its body is the status label, so the renderer
/// emits it for every type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateSection {
    /// Heading text, without the leading `##`.
    pub heading: String,
    /// Placeholder body.
    pub body: String,
}

/// Where a concept type came from. Recorded in telemetry as `type_kind`; the
/// slug itself is project-specific free text and is never recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Shipped with arkouda.
    Builtin,
    /// Declared in `.arkoudarc.toml`.
    Custom,
}

impl Origin {
    /// Stable short id used in telemetry events.
    pub fn name(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Custom => "custom",
        }
    }
}

/// Everything that distinguishes one kind of concept document from another.
#[derive(Debug, Clone)]
pub struct ConceptType {
    /// Short name used on the CLI (`--type adr`).
    pub slug: String,

    /// The OKF `type` value documents of this kind declare.
    pub okf_type: String,

    /// Status vocabulary, in lifecycle order. Never empty: the first entry is
    /// the status a freshly scaffolded document starts in, and the order is
    /// what `index.md` grouping uses.
    pub statuses: Vec<Status>,

    /// Body sections `arkouda check` requires, in the order they are reported.
    /// Matched against `##` headings case-insensitively.
    ///
    /// May be empty. A declared type that names no required sections gets a
    /// status vocabulary and a template without a body contract, which is the
    /// point of letting a project describe its own types.
    pub required_sections: Vec<String>,

    /// The section carrying this type's substance, printed by
    /// `arkouda section <id>` when no name is given. `None` when the type
    /// names none, in which case `section` requires an explicit name.
    pub primary_section: Option<String>,

    /// Directory `arkouda new` writes into when nothing is configured.
    pub default_dir: String,

    /// Sections scaffolded by `arkouda new`, after `## Status`. A superset of
    /// `required_sections`: some sections are worth prompting for without
    /// being worth failing a bundle over.
    pub template_sections: Vec<TemplateSection>,

    /// Producer-extension frontmatter keys scaffolded as empty lists.
    pub template_extensions: Vec<String>,

    /// Whether this type is built in or declared by the project.
    pub origin: Origin,
}

impl ConceptType {
    /// The status a new document of this type starts in: the first entry of
    /// the lifecycle.
    pub fn default_status(&self) -> &Status {
        self.statuses
            .first()
            .expect("every concept type declares at least one status")
    }

    /// Look up a status by its frontmatter value.
    pub fn status(&self, name: &str) -> Option<&Status> {
        let name = name.trim();
        self.statuses.iter().find(|status| status.name == name)
    }

    /// The status vocabulary as a comma-separated list, for diagnostics and
    /// help text.
    pub fn status_list(&self) -> String {
        self.statuses
            .iter()
            .map(|status| status.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl PartialEq for ConceptType {
    /// Slugs are unique across the registry, so they identify a type.
    fn eq(&self, other: &Self) -> bool {
        self.slug == other.slug
    }
}

impl Eq for ConceptType {}

/// Architecture Decision Record — Michael Nygard's template, and the type
/// arkouda started as.
pub fn adr() -> ConceptType {
    ConceptType {
        slug: "adr".to_owned(),
        okf_type: "Architecture Decision Record".to_owned(),
        statuses: [
            "proposed",
            "accepted",
            "superseded",
            "deprecated",
            "rejected",
        ]
        .map(Status::new)
        .to_vec(),
        required_sections: ["Status", "Context", "Decision", "Consequences"]
            .map(str::to_owned)
            .to_vec(),
        primary_section: Some("Decision".to_owned()),
        default_dir: "docs/adr".to_owned(),
        template_sections: vec![
            section(
                "Context",
                "TODO: describe the forces, constraints, and background for this decision.",
            ),
            section("Decision", "TODO: describe the decision."),
            section(
                "Consequences",
                "TODO: describe the positive, negative, and neutral consequences.",
            ),
        ],
        template_extensions: vec!["deciders".to_owned()],
        origin: Origin::Builtin,
    }
}

/// Product Requirements Document — what the software is supposed to do, as
/// against the decisions that serve it.
pub fn prd() -> ConceptType {
    ConceptType {
        slug: "prd".to_owned(),
        okf_type: "Product Requirements Document".to_owned(),
        statuses: [
            "draft",
            "in-review",
            "approved",
            "shipped",
            "abandoned",
            "superseded",
        ]
        .map(Status::new)
        .to_vec(),
        required_sections: [
            "Status",
            "Problem",
            "Requirements",
            "Non-Goals",
            "Success Metrics",
        ]
        .map(str::to_owned)
        .to_vec(),
        primary_section: Some("Requirements".to_owned()),
        default_dir: "docs/prd".to_owned(),
        template_sections: vec![
            section(
                "Problem",
                "TODO: describe who has the problem, why it matters, and why now.",
            ),
            section(
                "Approach",
                "TODO: describe the shape of the solution, in a paragraph.",
            ),
            section("Requirements", "TODO: describe what the software must do."),
            section(
                "Non-Goals",
                "TODO: describe what this explicitly does not cover.",
            ),
            section(
                "Success Metrics",
                "TODO: describe how we will know it worked.",
            ),
            section("Open Questions", "TODO: list what is still undecided."),
        ],
        template_extensions: vec!["owner".to_owned(), "decisions".to_owned()],
        origin: Origin::Builtin,
    }
}

fn section(heading: &str, body: &str) -> TemplateSection {
    TemplateSection {
        heading: heading.to_owned(),
        body: body.to_owned(),
    }
}

/// The types arkouda ships, in presentation order.
pub fn builtins() -> Vec<ConceptType> {
    vec![adr(), prd()]
}

static REGISTRY: OnceLock<&'static [ConceptType]> = OnceLock::new();

/// The built-ins, leaked on first use. Deliberately a separate cell from
/// [`REGISTRY`]: reading [`all`] before a registry is installed must not
/// install one, or a caller that happens to look at the types first would
/// silently pin the built-ins and make the project's `[[types]]` disappear.
static FALLBACK: OnceLock<&'static [ConceptType]> = OnceLock::new();

/// Install the registry for this process.
///
/// Returns `false` when one is already installed, in which case the existing
/// registry stands and the caller should treat it as the error it is. Reading
/// [`all`] first does not count as installing, so a `false` here means
/// `install` was genuinely called twice.
#[must_use]
pub fn install(types: Vec<ConceptType>) -> bool {
    let leaked: &'static [ConceptType] = Box::leak(types.into_boxed_slice());
    REGISTRY.set(leaked).is_ok()
}

/// Every concept type this invocation knows, in presentation order.
///
/// Falls back to the built-ins when nothing has been installed, so library
/// callers and unit tests see the shipped types without resolving a config.
pub fn all() -> &'static [ConceptType] {
    REGISTRY
        .get()
        .copied()
        .unwrap_or_else(|| *FALLBACK.get_or_init(|| Box::leak(builtins().into_boxed_slice())))
}

/// Resolve a type by its CLI slug.
pub fn by_slug(slug: &str) -> Option<&'static ConceptType> {
    all().iter().find(|concept_type| concept_type.slug == slug)
}

/// Resolve a type by the OKF `type` string a document declares. Matching is
/// exact: `type` is the one field OKF requires, and a near miss is a mistake
/// worth reporting rather than guessing at.
pub fn by_okf_type(okf_type: &str) -> Option<&'static ConceptType> {
    let okf_type = okf_type.trim();
    all()
        .iter()
        .find(|concept_type| concept_type.okf_type == okf_type)
}

/// Every CLI slug, for diagnostics and error messages.
pub fn slugs() -> Vec<&'static str> {
    all()
        .iter()
        .map(|concept_type| concept_type.slug.as_str())
        .collect()
}

/// Every OKF `type` string arkouda knows, as a comma-separated list.
pub fn okf_type_list() -> String {
    all()
        .iter()
        .map(|concept_type| format!("`{}`", concept_type.okf_type))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_and_okf_types_are_unique() {
        let mut slugs = slugs();
        slugs.sort_unstable();
        let count = slugs.len();
        slugs.dedup();
        assert_eq!(slugs.len(), count, "a slug identifies exactly one type");

        let mut okf_types: Vec<&str> = all().iter().map(|t| t.okf_type.as_str()).collect();
        okf_types.sort_unstable();
        let count = okf_types.len();
        okf_types.dedup();
        assert_eq!(okf_types.len(), count);
    }

    #[test]
    fn resolves_by_slug_and_by_okf_type() {
        assert_eq!(by_slug("adr").map(|t| t.slug.as_str()), Some("adr"));
        assert_eq!(by_slug("prd").map(|t| t.slug.as_str()), Some("prd"));
        assert_eq!(by_slug("rfc"), None, "undeclared types do not resolve");
        assert_eq!(
            by_okf_type("Product Requirements Document").map(|t| t.slug.as_str()),
            Some("prd")
        );
        assert_eq!(by_okf_type("BigQuery Table"), None);
    }

    #[test]
    fn the_default_status_opens_the_lifecycle() {
        assert_eq!(adr().default_status().name, "proposed");
        assert_eq!(prd().default_status().name, "draft");
    }

    #[test]
    fn statuses_resolve_to_their_labels() {
        assert_eq!(
            prd().status("in-review").map(|s| s.label.as_str()),
            Some("In Review")
        );
        assert_eq!(adr().status("shipped"), None, "vocabularies are per type");
    }

    #[test]
    fn reading_the_registry_does_not_install_one() {
        // `all()` falls back to the built-ins without touching REGISTRY, so a
        // caller that looks at the types before resolving a config does not
        // silently lock the project's `[[types]]` out. The fallback and the
        // registry are separate cells precisely so this holds.
        assert!(!all().is_empty());
        assert!(
            REGISTRY.get().is_none(),
            "reading the registry must leave it uninstalled"
        );
    }

    #[test]
    fn labels_are_derived_from_kebab_case_names() {
        assert_eq!(Status::new("draft").label, "Draft");
        assert_eq!(Status::new("in-review").label, "In Review");
        assert_eq!(
            Status::new("needs-more-thought").label,
            "Needs More Thought"
        );
    }

    #[test]
    fn every_required_section_is_scaffolded() {
        // A template that omits a section it requires would scaffold a
        // document that fails `arkouda check` on creation.
        for concept_type in builtins() {
            for required in &concept_type.required_sections {
                let scaffolded = required.eq_ignore_ascii_case("Status")
                    || concept_type
                        .template_sections
                        .iter()
                        .any(|section| section.heading.eq_ignore_ascii_case(required));
                assert!(
                    scaffolded,
                    "{} requires `## {required}` but does not scaffold it",
                    concept_type.slug
                );
            }
        }
    }

    #[test]
    fn the_primary_section_is_a_required_one() {
        for concept_type in builtins() {
            let primary = concept_type
                .primary_section
                .as_deref()
                .expect("both built-ins name a primary section");
            assert!(
                concept_type
                    .required_sections
                    .iter()
                    .any(|required| required.eq_ignore_ascii_case(primary)),
                "{}'s primary section must be one `check` guarantees is there",
                concept_type.slug
            );
        }
    }
}
