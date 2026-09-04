//! OKF concept frontmatter.
//!
//! Field names follow the [Open Knowledge Format][okf] §4.1. `type` is the
//! only field OKF requires; `title`, `description`, `resource`, and `tags` are
//! its recommended set. `deciders`, `superseded_by`, `owner`,
//! `target_release`, and `decisions` are producer-defined extensions that
//! carry the metadata OKF leaves open.
//!
//! OKF v0.2 adds the provenance, trust, and lifecycle families (§5):
//! `sources`, `generated`, `verified`, and `stale_after`. All are optional —
//! §11 forbids rejecting a concept for missing any of them — and arkouda
//! parses them so a v0.2 bundle is understood rather than merely tolerated.
//!
//! `generated.at` supersedes v0.1's `timestamp` as the record of a concept's
//! last meaningful change (§13.1). Arkouda reads `generated.at` first and
//! falls back to a legacy `timestamp`, which the spec permits, so v0.1
//! documents keep working.
//!
//! One struct serves every concept type. Which keys are meaningful depends on
//! the type — `deciders` is an ADR's, `owner` a PRD's — but the required set
//! is the same for all of them, and unknown keys are tolerated either way, so
//! there is nothing per type to model here.
//!
//! [okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md

use crate::concept::types::{self, ConceptType};
use serde::{Deserialize, Deserializer, Serialize};

/// YAML frontmatter from a concept document.
///
/// Unknown keys are ignored rather than rejected, per OKF §11: consumers must
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

    /// ISO 8601 date or datetime. **Legacy v0.1.** Superseded by
    /// `generated.at`; still read when `generated` is absent.
    pub timestamp: Option<String>,

    /// How the current content was produced (OKF §5.2). `generated.at` is the
    /// concept's last meaningful change.
    pub generated: Option<Generated>,

    /// Verification events (OKF §5.2), newest last. A single verifier may be
    /// written as one bare `{ by, at }` mapping; §11 requires consumers to
    /// treat that as a one-element list, which [`one_or_many`] does.
    #[serde(default, deserialize_with = "one_or_many")]
    pub verified: Vec<Actor>,

    /// Materials this concept derives from (OKF §5.1).
    pub sources: Vec<Source>,

    /// Window framing every `sources[].usage_count` (OKF §5.1).
    pub usage_window: Option<UsageWindow>,

    /// Absolute instant at which the content goes stale (OKF §5.5).
    pub stale_after: Option<String>,

    /// OKF §5.4 lifecycle: `draft | stable | deprecated`. Absent means
    /// `stable`.
    ///
    /// This is the coarse, portable signal every OKF consumer understands. It
    /// is the projection of [`Self::lifecycle`], not an independent axis —
    /// `arkouda new` writes it, and `check` reports a `status` that
    /// contradicts the lifecycle it should have come from.
    ///
    /// Before arkouda 0.7 this key held the per-type vocabulary that now
    /// lives in `lifecycle`; see [`Self::resolved_lifecycle`].
    pub status: Option<String>,

    /// Lifecycle from the concept type's own vocabulary — `accepted` for an
    /// ADR, `shipped` for a PRD, whatever a project declares for its types.
    ///
    /// Finer than OKF's three values, which is the point: `superseded` and
    /// `rejected` both project onto `deprecated`, and the difference is the
    /// thing a reader of decisions actually wants.
    pub lifecycle: Option<String>,

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

/// An actor and the instant it acted (OKF §5.2). Used for `generated` and for
/// each `verified` entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Actor {
    /// Who or what acted, in the OKF §7 actor convention:
    /// `<producer>/<version>`, `human:<id>`, or `process:<id>`.
    pub by: Option<String>,
    /// ISO 8601 datetime.
    pub at: Option<String>,
}

/// Alias kept for readability at the `generated` field, which is one actor
/// rather than a list of them.
pub type Generated = Actor;

/// One entry in `sources` (OKF §5.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Source {
    /// Stable key a body footnote cites for per-claim attribution.
    pub id: Option<String>,
    /// Required within an entry: the artifact or scope this derives from.
    pub resource: Option<String>,
    /// Human-readable label.
    pub title: Option<String>,
    /// Who produced the source, in the actor convention.
    pub author: Option<String>,
    /// How often the source was exercised over the usage window.
    pub usage_count: Option<u64>,
    /// When the source itself last changed.
    pub last_modified: Option<String>,
    /// Per-entry override of the shared `usage_window`.
    pub usage_window: Option<UsageWindow>,
}

/// The `{ from, to }` datetime range framing a `usage_count` (OKF §5.1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UsageWindow {
    /// Start of the window.
    pub from: Option<String>,
    /// End of the window.
    pub to: Option<String>,
}

/// Deserialize a field that OKF permits as either one mapping or a list of
/// them. OKF §11: "Consumers MUST treat a bare `verified` mapping as a
/// one-element list."
fn one_or_many<'de, D>(deserializer: D) -> Result<Vec<Actor>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(Actor),
        Many(Vec<Actor>),
    }

    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(actor) => vec![actor],
        OneOrMany::Many(actors) => actors,
    })
}

/// Trim a value and discard it when nothing is left.
fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then_some(trimmed)
    })
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

    /// The concept's per-type lifecycle, honouring the pre-0.7 spelling.
    ///
    /// `lifecycle` when present. Otherwise a `status` holding a value from
    /// `concept_type`'s vocabulary is the old spelling of the same thing, and
    /// is read as one — so a bundle written before 0.7 keeps sorting,
    /// grouping, and displaying exactly as it did.
    pub fn resolved_lifecycle<'a>(&'a self, concept_type: Option<&ConceptType>) -> Option<&'a str> {
        if let Some(lifecycle) = non_empty(self.lifecycle.as_deref()) {
            return Some(lifecycle);
        }
        let status = non_empty(self.status.as_deref())?;
        concept_type
            .filter(|concept_type| concept_type.status(status).is_some())
            .map(|_| status)
    }

    /// True when the per-type lifecycle is only in `status`, the pre-0.7 key.
    pub fn uses_legacy_status(&self, concept_type: Option<&ConceptType>) -> bool {
        non_empty(self.lifecycle.as_deref()).is_none()
            && self.resolved_lifecycle(concept_type).is_some()
    }

    /// Display lifecycle or a placeholder. This is the `list -l` status column
    /// and the `index.md` grouping key: the fine per-type value, not OKF's
    /// coarse projection of it, because the coarse one collapses distinctions
    /// a reader of decisions came for.
    pub fn display_status(&self) -> &str {
        self.resolved_lifecycle(self.resolved_type())
            .unwrap_or("<missing>")
    }

    /// The concept's last meaningful change: `generated.at` (OKF v0.2 §5.2),
    /// falling back to a legacy v0.1 `timestamp`.
    ///
    /// §13.1 supersedes `timestamp` with `generated.at` and permits consumers
    /// to fall back, so a v0.1 document keeps its timestamp rather than
    /// looking undated.
    pub fn content_timestamp(&self) -> Option<&str> {
        let generated = self
            .generated
            .as_ref()
            .and_then(|generated| generated.at.as_deref())
            .map(str::trim)
            .filter(|at| !at.is_empty());

        generated.or_else(|| {
            self.timestamp
                .as_deref()
                .map(str::trim)
                .filter(|timestamp| !timestamp.is_empty())
        })
    }

    /// True when this concept dates itself only the v0.1 way.
    pub fn uses_legacy_timestamp(&self) -> bool {
        self.timestamp.is_some()
            && self
                .generated
                .as_ref()
                .and_then(|generated| generated.at.as_deref())
                .is_none()
    }

    /// Display timestamp or a placeholder.
    pub fn display_timestamp(&self) -> &str {
        self.content_timestamp().unwrap_or("<missing>")
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
