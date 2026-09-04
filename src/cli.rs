//! Command-line interface definitions.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Main CLI application.
#[derive(Parser)]
#[command(name = "arkouda")]
#[command(author, version, about = "Navigate and validate decision and requirements documents", long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Command,

    /// Directory containing concept Markdown files. Overrides any `dirs` from
    /// `.arkoudarc.toml`, for every type. When neither is set, each type
    /// defaults to its own directory (`docs/adr`, `docs/prd`).
    #[arg(long, global = true, env = "ADR_DIR")]
    pub dir: Option<PathBuf>,

    /// Suppress non-essential informational output.
    #[arg(long, short, global = true)]
    pub quiet: bool,
}

/// Available CLI commands.
#[derive(Subcommand)]
pub enum Command {
    /// List concepts in the collection. Prints one path per line; `-l` for the
    /// table.
    List(ListArgs),

    /// Print one concept's primary section by id, or a section you name.
    Section(SectionArgs),

    /// Validate OKF conformance, frontmatter, and Markdown structure.
    Check,

    /// Create a new concept from its type's template.
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
    /// Sort concepts by this field.
    #[arg(long, default_value = "id", value_enum)]
    pub sort: SortBy,

    /// Show only concepts of this type. Slugs come from the built-in types
    /// and any `[[types]]` declared in `.arkoudarc.toml`, so the valid set is
    /// resolved after the config is read rather than by clap.
    #[arg(long = "type", value_name = "TYPE")]
    pub concept_type: Option<String>,

    /// Long form: print `ID TYPE STATUS TIMESTAMP PATH TITLE — DESCRIPTION`
    /// instead of just paths. Headerless either way.
    #[arg(short = 'l', long)]
    pub long: bool,
}

/// Arguments for the `section` command.
#[derive(clap::Args)]
pub struct SectionArgs {
    /// Concept id, filename stem, or filename.
    pub id: String,

    /// Section to print. Defaults to the concept type's primary section:
    /// `Decision` for an ADR, `Requirements` for a PRD. Errors if the concept
    /// has no such section.
    pub name: Option<String>,
}

/// Arguments for the `new` command.
#[derive(clap::Args)]
pub struct NewArgs {
    /// Concept title.
    pub title: String,

    /// Type of concept to create. Selects the template, the `type` string, the
    /// status vocabulary, and the target directory. Defaults to `adr`, which a
    /// project that shadows or omits the built-in must override explicitly.
    #[arg(long = "type", value_name = "TYPE", default_value = "adr")]
    pub concept_type: String,

    /// Explicit concept id, used as the filename stem. Defaults to a slug
    /// generated from the title.
    #[arg(long)]
    pub id: Option<String>,

    /// Initial status. Must be one of the chosen type's values; defaults to
    /// the first of its lifecycle (`proposed` for an ADR, `draft` for a PRD).
    #[arg(long)]
    pub status: Option<String>,

    /// One-line summary of what was decided, or what is being built. Defaults
    /// to a TODO placeholder.
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

/// Field used to sort concepts.
#[derive(ValueEnum, Clone, Copy, Debug)]
#[value(rename_all = "kebab-case")]
pub enum SortBy {
    /// Sort by concept id.
    Id,
    /// Sort by timestamp.
    Timestamp,
    /// Sort by status. Statuses compare as strings, so a mixed collection
    /// interleaves two vocabularies; pair it with `--type`.
    Status,
}
