//! ADR frontmatter types.

use serde::{Deserialize, Serialize};

/// YAML frontmatter from an ADR Markdown file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Frontmatter {
    /// Stable ADR id. Expected to match the filename stem.
    pub id: Option<String>,

    /// Human-readable decision title.
    pub title: Option<String>,

    /// Short summary of the decision.
    #[serde(rename = "abstract")]
    pub abstract_text: Option<String>,

    /// Decision status.
    pub status: Option<String>,

    /// Decision date in YYYY-MM-DD format.
    pub date: Option<String>,

    /// People or groups involved in the decision.
    pub deciders: Option<Vec<String>>,

    /// Searchable tags.
    pub tags: Option<Vec<String>>,

    /// ADR id that supersedes this ADR, when relevant.
    pub superseded_by: Option<String>,
}

impl Frontmatter {
    /// Required frontmatter keys for this project.
    pub const REQUIRED_KEYS: &'static [&'static str] =
        &["id", "title", "abstract", "status", "date"];

    /// Display id or a placeholder.
    pub fn display_id(&self) -> &str {
        self.id.as_deref().unwrap_or("<missing>")
    }

    /// Display title or a placeholder.
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("<missing title>")
    }

    /// Display abstract or a placeholder.
    pub fn display_abstract(&self) -> &str {
        self.abstract_text
            .as_deref()
            .unwrap_or("<missing abstract>")
    }

    /// Display status or a placeholder.
    pub fn display_status(&self) -> &str {
        self.status.as_deref().unwrap_or("<missing>")
    }

    /// Display date or a placeholder.
    pub fn display_date(&self) -> &str {
        self.date.as_deref().unwrap_or("<missing>")
    }
}
