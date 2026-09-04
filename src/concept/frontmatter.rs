//! OKF concept frontmatter.
//!
//! Field names follow the [Open Knowledge Format][okf] §4.1. `type` is the
//! only field OKF requires; `title`, `description`, `resource`, `tags`, and
//! `timestamp` are its recommended set. `status`, `deciders`, `superseded_by`,
//! `owner`, `target_release`, and `decisions` are producer-defined extensions
//! that carry the metadata OKF leaves open.
//!
//! One struct serves every concept type. Which keys are meaningful depends on
//! the type — `deciders` is an ADR's, `owner` a PRD's — but the required set
//! is the same for all of them, and unknown keys are tolerated either way, so
//! there is nothing per type to model here.
//!
//! [okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md

use crate::concept::types::{self, ConceptType};
use serde::{Deserialize, Serialize};

/// YAML frontmatter from a concept document.
///
/// Unknown keys are ignored rather than rejected, per OKF §9: consumers must
/// not refuse a document because it carries fields they do not recognize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Frontmatter {
    /// OKF concept type. Required by the spec; arkouda expects one of the
    /// types in [`crate::concept::types`].
    #[serde(rename = "type")]
    pub concept_type: Option<String>,

    /// Human-readable title.
    pub title: Option<String>,

    /// Single-sentence summary: what was decided, or what is being built.
    pub description: Option<String>,

    /// URI of an underlying resource this concept is bound to, when one
    /// exists — a ticket, RFC, or design doc. Most ADRs are abstract concepts
    /// and omit it; a PRD's tracker item goes here.
    pub resource: Option<String>,

    /// Searchable tags.
    pub tags: Vec<String>,

    /// ISO 8601 date or datetime.
    pub timestamp: Option<String>,

    /// Lifecycle status, from the concept type's vocabulary. Producer
    /// extension.
    pub status: Option<String>,

    /// People or groups involved in the decision. ADR extension.
    pub deciders: Vec<String>,

    /// Concept id that supersedes this concept. Producer extension.
    pub superseded_by: Option<String>,

    /// People or groups accountable for the requirement. PRD extension,
    /// mirroring `deciders`.
    pub owner: Vec<String>,

    /// Target release date. PRD extension.
    pub target_release: Option<String>,

    /// Concept ids of the decisions that shaped this document. PRD extension.
    pub decisions: Vec<String>,
}

impl Frontmatter {
    /// The concept type this document declares, when arkouda knows it.
    pub fn resolved_type(&self) -> Option<&'static ConceptType> {
        types::by_okf_type(self.concept_type.as_deref()?)
    }

    /// Display type or a placeholder.
    pub fn display_type(&self) -> &str {
        self.concept_type.as_deref().unwrap_or("<missing>")
    }

    /// The type's CLI slug, for the `list -l` table. A single token either
    /// way: the column has to stay awk-sliceable, so an unrecognized type is
    /// reported as such rather than printed in full.
    pub fn display_type_slug(&self) -> &str {
        match self.concept_type.as_deref().map(str::trim) {
            None | Some("") => "<missing>",
            Some(declared) => types::by_okf_type(declared)
                .map_or("<unknown>", |concept_type| concept_type.slug.as_str()),
        }
    }

    /// Display title or a placeholder.
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("<missing title>")
    }

    /// Display description or a placeholder.
    pub fn display_description(&self) -> &str {
        self.description
            .as_deref()
            .unwrap_or("<missing description>")
    }

    /// Display status or a placeholder.
    pub fn display_status(&self) -> &str {
        self.status.as_deref().unwrap_or("<missing>")
    }

    /// Display timestamp or a placeholder.
    pub fn display_timestamp(&self) -> &str {
        self.timestamp.as_deref().unwrap_or("<missing>")
    }

    /// Every frontmatter concept-id reference this document makes, paired with
    /// the key it came from.
    pub fn references(&self) -> Vec<(&'static str, &str)> {
        let superseded_by = self
            .superseded_by
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(|id| ("superseded_by", id));

        superseded_by
            .into_iter()
            .chain(
                self.decisions
                    .iter()
                    .map(String::as_str)
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .map(|id| ("decisions", id)),
            )
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Frontmatter {
        serde_yaml::from_str(yaml).expect("valid frontmatter")
    }

    #[test]
    fn resolves_the_declared_type() {
        let frontmatter = parse("type: Product Requirements Document\n");
        assert_eq!(
            frontmatter.resolved_type().map(|t| t.slug.as_str()),
            Some("prd")
        );
        assert_eq!(frontmatter.display_type_slug(), "prd");
    }

    #[test]
    fn an_unknown_or_missing_type_is_a_single_token_in_the_table() {
        assert_eq!(
            parse("type: BigQuery Table\n").display_type_slug(),
            "<unknown>"
        );
        assert_eq!(parse("title: X\n").display_type_slug(), "<missing>");
    }

    #[test]
    fn collects_references_from_both_keys() {
        let frontmatter = parse(
            "type: Product Requirements Document\nsuperseded_by: newer\ndecisions:\n  - adopt-okf\n  - ' '\n",
        );
        assert_eq!(
            frontmatter.references(),
            vec![("superseded_by", "newer"), ("decisions", "adopt-okf")],
            "blank entries are not references"
        );
    }

    #[test]
    fn prd_extensions_round_trip() {
        let frontmatter = parse(
            "type: Product Requirements Document\nowner:\n  - alice\ntarget_release: 2026-09-01\n",
        );
        assert_eq!(frontmatter.owner, vec!["alice".to_owned()]);
        assert_eq!(frontmatter.target_release.as_deref(), Some("2026-09-01"));
    }
}
