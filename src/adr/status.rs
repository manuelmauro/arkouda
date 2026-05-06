//! ADR status values.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Controlled ADR status values.
#[derive(ValueEnum, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "kebab-case")]
pub enum AdrStatus {
    /// Decision is proposed but not yet accepted.
    Proposed,
    /// Decision is accepted and active.
    Accepted,
    /// Decision has been superseded by another ADR.
    Superseded,
    /// Decision is no longer recommended.
    Deprecated,
    /// Decision was rejected.
    Rejected,
}

impl AdrStatus {
    /// Every status variant, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::Proposed,
        Self::Accepted,
        Self::Superseded,
        Self::Deprecated,
        Self::Rejected,
    ];

    /// Render the title-case label (e.g. `"Proposed"`) used in ADR body sections.
    pub fn label(self) -> Label {
        Label(self)
    }
}

impl FromStr for AdrStatus {
    type Err = UnknownStatus;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "superseded" => Ok(Self::Superseded),
            "deprecated" => Ok(Self::Deprecated),
            "rejected" => Ok(Self::Rejected),
            _ => Err(UnknownStatus),
        }
    }
}

impl fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
            Self::Deprecated => "deprecated",
            Self::Rejected => "rejected",
        })
    }
}

/// Title-case rendering of [`AdrStatus`], obtained via [`AdrStatus::label`].
#[derive(Debug, Clone, Copy)]
pub struct Label(AdrStatus);

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            AdrStatus::Proposed => "Proposed",
            AdrStatus::Accepted => "Accepted",
            AdrStatus::Superseded => "Superseded",
            AdrStatus::Deprecated => "Deprecated",
            AdrStatus::Rejected => "Rejected",
        })
    }
}

/// Returned when a string does not name a known ADR status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownStatus;

impl fmt::Display for UnknownStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown ADR status")
    }
}

impl std::error::Error for UnknownStatus {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_names() {
        assert_eq!("proposed".parse::<AdrStatus>(), Ok(AdrStatus::Proposed));
        assert_eq!("rejected".parse::<AdrStatus>(), Ok(AdrStatus::Rejected));
        assert_eq!("Proposed".parse::<AdrStatus>(), Err(UnknownStatus));
    }

    #[test]
    fn display_is_canonical_form() {
        assert_eq!(AdrStatus::Proposed.to_string(), "proposed");
    }

    #[test]
    fn label_is_title_case() {
        assert_eq!(AdrStatus::Proposed.label().to_string(), "Proposed");
    }
}
