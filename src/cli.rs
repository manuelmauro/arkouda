//! Command-line interface definitions.

use crate::adr::AdrStatus;
use clap::{Parser, Subcommand, ValueEnum};
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

    /// Directory containing ADR Markdown files. Overrides any `dirs` from
    /// `.arkoudarc.toml`. When neither is set, defaults to `docs/adr`.
    #[arg(long, global = true, env = "ADR_DIR")]
    pub dir: Option<PathBuf>,

    /// Suppress non-essential informational output.
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

/// Available CLI commands.
#[derive(Subcommand)]
pub enum Command {
    /// List ADRs in the collection. Prints one path per line; `-l` for the
    /// table.
    List(ListArgs),

    /// Print one ADR's `## Decision` section by id; `--section` to pick another.
    Decision(DecisionArgs),

    /// Validate OKF conformance, ADR frontmatter, and Markdown structure.
    Check,

    /// Create a new ADR from the standard template.
    New(NewArgs),

    /// Regenerate each bundle's `index.md` directory listing.
    Index,

    /// Manage the arkouda installation.
    #[command(name = "self")]
    SelfCmd(SelfArgs),
}

/// Arguments for the `list` command.
#[derive(clap::Args)]
pub struct ListArgs {
    /// Sort ADRs by this field.
    #[arg(long, default_value = "id", value_enum)]
    pub sort: SortBy,

    /// Long form: print `ID STATUS TIMESTAMP PATH TITLE` columns instead of
    /// just paths. Headerless either way.
    #[arg(short = 'l', long)]
    pub long: bool,
}

/// Arguments for the `decision` command.
#[derive(clap::Args)]
pub struct DecisionArgs {
    /// ADR concept id, filename stem, or filename.
    pub id: String,

    /// Print this `## <name>` section instead of `## Decision`. Errors if the
    /// ADR has no such section. Common values: `context`, `consequences`,
    /// `status`.
    #[arg(long)]
    pub section: Option<String>,
}

/// Arguments for the `new` command.
#[derive(clap::Args)]
pub struct NewArgs {
    /// ADR title.
    pub title: String,

    /// Explicit concept id, used as the filename stem. Defaults to a slug
    /// generated from the title.
    #[arg(long)]
    pub id: Option<String>,

    /// Initial ADR status.
    #[arg(long, default_value = "proposed", value_enum)]
    pub status: AdrStatus,

    /// One-line summary of the decision (what was decided, not just the
    /// topic). Defaults to a TODO placeholder.
    #[arg(long)]
    pub description: Option<String>,
}

/// Arguments for the `self` command.
#[derive(clap::Args)]
pub struct SelfArgs {
    /// The `self` subcommand to run.
    #[command(subcommand)]
    pub command: SelfCommand,
}

/// `self` subcommands.
#[derive(Subcommand)]
pub enum SelfCommand {
    /// Generate shell completions for the given shell, printed to stdout.
    Completions(CompletionsArgs),
}

/// Arguments for the `self completions` command.
#[derive(clap::Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for.
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Supported shells for completion generation.
#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Shell {
    /// Bash shell.
    Bash,
    /// Zsh shell.
    Zsh,
    /// Fish shell.
    Fish,
    /// PowerShell.
    #[value(name = "powershell")]
    PowerShell,
    /// Elvish shell.
    Elvish,
}

/// Field used to sort ADRs.
#[derive(ValueEnum, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum SortBy {
    /// Sort by concept id.
    Id,
    /// Sort by decision timestamp.
    Timestamp,
    /// Sort by ADR status.
    Status,
}
