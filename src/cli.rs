//! Command-line interface definitions.

use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;
use std::path::PathBuf;

/// Main CLI application.
#[derive(Parser)]
#[command(name = "arkouda")]
#[command(author, version, about = "Navigate and validate Architecture Decision Records", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Command,

    /// Directory containing ADR Markdown files.
    #[arg(long, global = true, default_value = "docs/adr", env = "ADR_DIR")]
    pub dir: PathBuf,

    /// Suppress non-essential informational output.
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

/// Available CLI commands.
#[derive(Subcommand)]
pub enum Command {
    /// List ADRs in the collection.
    List(ListArgs),

    /// Show one ADR by id or filename.
    Show(ShowArgs),

    /// Search ADR metadata and Markdown content.
    Search(SearchArgs),

    /// Validate ADR frontmatter, filenames, and Markdown structure.
    Check(CheckArgs),

    /// Create a new ADR from the standard template.
    New(NewArgs),
}

/// Arguments for the `list` command.
#[derive(clap::Args, Clone)]
pub struct ListArgs {
    /// Sort ADRs by this field.
    #[arg(long, default_value = "id", value_enum)]
    pub sort: SortBy,
}

/// Arguments for the `show` command.
#[derive(clap::Args, Clone)]
pub struct ShowArgs {
    /// ADR id, filename stem, or filename.
    pub id: String,
}

/// Arguments for the `search` command.
#[derive(clap::Args, Clone)]
pub struct SearchArgs {
    /// Case-insensitive search query.
    pub query: String,
}

/// Arguments for the `check` command.
#[derive(clap::Args, Clone)]
pub struct CheckArgs {}

/// Arguments for the `new` command.
#[derive(clap::Args, Clone)]
pub struct NewArgs {
    /// ADR title.
    pub title: String,

    /// Explicit ADR id. Defaults to a slug generated from the title.
    #[arg(long)]
    pub id: Option<String>,

    /// Initial ADR status.
    #[arg(long, default_value = "proposed", value_enum)]
    pub status: AdrStatus,

    /// ADR abstract. Defaults to a TODO placeholder.
    #[arg(long = "abstract")]
    pub abstract_text: Option<String>,
}

/// Field used to sort ADRs.
#[derive(ValueEnum, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum SortBy {
    /// Sort by ADR id.
    Id,
    /// Sort by ADR date.
    Date,
    /// Sort by ADR status.
    Status,
}

/// Controlled ADR status values.
#[derive(ValueEnum, Clone, Copy, Debug)]
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
    /// Return the frontmatter representation for this status.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
            Self::Deprecated => "deprecated",
            Self::Rejected => "rejected",
        }
    }

    /// Return the human-readable section value for this status.
    pub fn label(self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::Accepted => "Accepted",
            Self::Superseded => "Superseded",
            Self::Deprecated => "Deprecated",
            Self::Rejected => "Rejected",
        }
    }
}

impl fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
