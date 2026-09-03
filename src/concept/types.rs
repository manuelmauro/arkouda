//! Concept type descriptors.
//!
//! Everything arkouda knows about a kind of document — its OKF `type` string,
//! its status vocabulary, the body sections it must have, the section that
//! carries its substance, where new ones are written, and the template
//! `arkouda new` scaffolds — lives in one [`ConceptType`] value. The two
//! built-in types are the only instances, and types are not user-definable.
//!
//! The descriptor is nonetheless shaped as the thing a user-supplied template
//! would deserialize into: adding bring-your-own types later is a config
//! parser over this struct rather than a redesign. See the ADR
//! `support-product-requirements-documents`.

/// One status in a concept type's lifecycle vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Status {
    /// Frontmatter value, lowercase and kebab-case (`in-review`).
    pub name: &'static str,
    /// Title-case rendering used in `index.md` headings and in the body's
    /// `## Status` section (`In Review`).
    pub label: &'static str,
}

/// A body section `arkouda new` scaffolds: its heading and placeholder prose.
///
/// `## Status` is not listed: its body is the status label, so the renderer
/// emits it for every type.
#[derive(Debug, Clone, Copy)]
pub struct TemplateSection {
    /// Heading text, without the leading `##`.
    pub heading: &'static str,
    /// Placeholder body.
    pub body: &'static str,
}

/// Everything that distinguishes one kind of concept document from another.
#[derive(Debug)]
pub struct ConceptType {
    /// Short name used on the CLI (`--type adr`).
    pub slug: &'static str,

    /// The OKF `type` value documents of this kind declare.
    pub okf_type: &'static str,

    /// Status vocabulary, in lifecycle order. The first entry is the status a
    /// freshly scaffolded document starts in, and the order is what `index.md`
    /// grouping uses.
    pub statuses: &'static [Status],

    /// Body sections `arkouda check` requires, in the order they are reported.
    /// Matched against `##` headings case-insensitively.
    pub required_sections: &'static [&'static str],

    /// The section carrying this type's substance, printed by
    /// `arkouda section <id>` when no name is given.
    pub primary_section: &'static str,

    /// Directory `arkouda new` writes into when nothing is configured.
    pub default_dir: &'static str,

    /// Sections scaffolded by `arkouda new`, after `## Status`. A superset of
    /// `required_sections`: some sections are worth prompting for without
    /// being worth failing a bundle over.
    pub template_sections: &'static [TemplateSection],

    /// Producer-extension frontmatter keys scaffolded as empty lists.
    pub template_extensions: &'static [&'static str],
}

impl ConceptType {
    /// The status a new document of this type starts in: the first entry of
    /// the lifecycle.
    pub fn default_status(&self) -> &'static Status {
        self.statuses
            .first()
            .expect("every concept type declares at least one status")
    }

    /// Look up a status by its frontmatter value.
    pub fn status(&self, name: &str) -> Option<&'static Status> {
        let name = name.trim();
        self.statuses.iter().find(|status| status.name == name)
    }

    /// The status vocabulary as a comma-separated list, for diagnostics and
    /// help text.
    pub fn status_list(&self) -> String {
        self.statuses
            .iter()
            .map(|status| status.name)
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
pub const ADR: ConceptType = ConceptType {
    slug: "adr",
    okf_type: "Architecture Decision Record",
    statuses: &[
        Status {
            name: "proposed",
            label: "Proposed",
        },
        Status {
            name: "accepted",
            label: "Accepted",
        },
        Status {
            name: "superseded",
            label: "Superseded",
        },
        Status {
            name: "deprecated",
            label: "Deprecated",
        },
        Status {
            name: "rejected",
            label: "Rejected",
        },
    ],
    required_sections: &["Status", "Context", "Decision", "Consequences"],
    primary_section: "Decision",
    default_dir: "docs/adr",
    template_sections: &[
        TemplateSection {
            heading: "Context",
            body: "TODO: describe the forces, constraints, and background for this decision.",
        },
        TemplateSection {
            heading: "Decision",
            body: "TODO: describe the decision.",
        },
        TemplateSection {
            heading: "Consequences",
            body: "TODO: describe the positive, negative, and neutral consequences.",
        },
    ],
    template_extensions: &["deciders"],
};

/// Product Requirements Document — what the software is supposed to do, as
/// against the decisions that serve it.
pub const PRD: ConceptType = ConceptType {
    slug: "prd",
    okf_type: "Product Requirements Document",
    statuses: &[
        Status {
            name: "draft",
            label: "Draft",
        },
        Status {
            name: "in-review",
            label: "In Review",
        },
        Status {
            name: "approved",
            label: "Approved",
        },
        Status {
            name: "shipped",
            label: "Shipped",
        },
        Status {
            name: "abandoned",
            label: "Abandoned",
        },
        Status {
            name: "superseded",
            label: "Superseded",
        },
    ],
    required_sections: &[
        "Status",
        "Problem",
        "Requirements",
        "Non-Goals",
        "Success Metrics",
    ],
    primary_section: "Requirements",
    default_dir: "docs/prd",
    template_sections: &[
        TemplateSection {
            heading: "Problem",
            body: "TODO: describe who has the problem, why it matters, and why now.",
        },
        TemplateSection {
            heading: "Approach",
            body: "TODO: describe the shape of the solution, in a paragraph.",
        },
        TemplateSection {
            heading: "Requirements",
            body: "TODO: describe what the software must do.",
        },
        TemplateSection {
            heading: "Non-Goals",
            body: "TODO: describe what this explicitly does not cover.",
        },
        TemplateSection {
            heading: "Success Metrics",
            body: "TODO: describe how we will know it worked.",
        },
        TemplateSection {
            heading: "Open Questions",
            body: "TODO: list what is still undecided.",
        },
    ],
    template_extensions: &["owner", "decisions"],
};

/// Every concept type arkouda knows, in the order they are presented.
pub static ALL: &[ConceptType] = &[ADR, PRD];

/// Resolve a type by its CLI slug.
pub fn by_slug(slug: &str) -> Option<&'static ConceptType> {
    ALL.iter().find(|concept_type| concept_type.slug == slug)
}

/// Resolve a type by the OKF `type` string a document declares. Matching is
/// exact: `type` is the one field OKF requires, and a near miss is a mistake
/// worth reporting rather than guessing at.
pub fn by_okf_type(okf_type: &str) -> Option<&'static ConceptType> {
    let okf_type = okf_type.trim();
    ALL.iter()
        .find(|concept_type| concept_type.okf_type == okf_type)
}

/// Every CLI slug, for clap's value parser and for diagnostics.
pub fn slugs() -> Vec<&'static str> {
    ALL.iter().map(|concept_type| concept_type.slug).collect()
}

/// Every OKF `type` string arkouda knows, as a comma-separated list.
pub fn okf_type_list() -> String {
    ALL.iter()
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

        let mut okf_types: Vec<&str> = ALL.iter().map(|t| t.okf_type).collect();
        okf_types.sort_unstable();
        let count = okf_types.len();
        okf_types.dedup();
        assert_eq!(okf_types.len(), count);
    }

    #[test]
    fn resolves_by_slug_and_by_okf_type() {
        assert_eq!(by_slug("adr"), Some(&ADR));
        assert_eq!(by_slug("prd"), Some(&PRD));
        assert_eq!(by_slug("rfc"), None);
        assert_eq!(by_okf_type("Product Requirements Document"), Some(&PRD));
        assert_eq!(by_okf_type("BigQuery Table"), None);
    }

    #[test]
    fn the_default_status_opens_the_lifecycle() {
        assert_eq!(ADR.default_status().name, "proposed");
        assert_eq!(PRD.default_status().name, "draft");
    }

    #[test]
    fn statuses_resolve_to_their_labels() {
        assert_eq!(PRD.status("in-review").map(|s| s.label), Some("In Review"));
        assert_eq!(ADR.status("shipped"), None, "vocabularies are per type");
    }

    #[test]
    fn every_required_section_is_scaffolded() {
        // A template that omits a section it requires would scaffold a
        // document that fails `arkouda check` on creation.
        for concept_type in ALL {
            for required in concept_type.required_sections {
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
        for concept_type in ALL {
            assert!(
                concept_type
                    .required_sections
                    .iter()
                    .any(|required| required.eq_ignore_ascii_case(concept_type.primary_section)),
                "{}'s primary section must be one `check` guarantees is there",
                concept_type.slug
            );
        }
    }
}
