//! OKF concept frontmatter for Architecture Decision Records.
//!
//! Field names follow the [Open Knowledge Format][okf] §4.1. `type` is the
//! only field OKF requires; `title`, `description`, `resource`, `tags`, and
//! `timestamp` are its recommended set. `status`, `deciders`, and
//! `superseded_by` are producer-defined extensions that carry the ADR-specific
//! metadata OKF leaves open.
//!
//! [okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md

use serde::{Deserialize, Serialize};

/// The OKF `type` value every arkouda ADR declares.
pub const ADR_TYPE: &str = "Architecture Decision Record";

/// YAML frontmatter from an ADR concept document.
///
/// Unknown keys are ignored rather than rejected, per OKF §9: consumers must
/// not refuse a document because it carries fields they do not recognize.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Frontmatter {
    /// OKF concept type. Required by the spec; arkouda expects [`ADR_TYPE`].
    #[serde(rename = "type")]
    pub concept_type: Option<String>,

    /// Human-readable decision title.
    pub title: Option<String>,

    /// Single-sentence summary of the decision.
    pub description: Option<String>,

    /// URI of an underlying resource this decision is bound to, when one
    /// exists — a ticket, RFC, or design doc. Most ADRs are abstract concepts
    /// and omit it.
    pub resource: Option<String>,

    /// Searchable tags.
    pub tags: Vec<String>,

    /// ISO 8601 date or datetime the decision was taken.
    pub timestamp: Option<String>,

    /// Decision status. ADR extension.
    pub status: Option<String>,

    /// People or groups involved in the decision. ADR extension.
    pub deciders: Vec<String>,

    /// Concept id that supersedes this ADR. ADR extension.
    pub superseded_by: Option<String>,
}

impl Frontmatter {
    /// Display type or a placeholder.
    pub fn display_type(&self) -> &str {
        self.concept_type.as_deref().unwrap_or("<missing>")
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
}
