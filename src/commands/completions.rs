//! Generate shell completions.

use crate::cli::{Cli, CompletionsArgs, Shell};
use crate::error::Result;
use clap::CommandFactory;
use clap_complete::{Shell as ClapShell, generate};
use std::io;

/// Run the completions command.
pub fn run(args: &CompletionsArgs) -> Result<i32> {
    let mut cmd = Cli::command();
    let shell = match args.shell {
        Shell::Bash => ClapShell::Bash,
        Shell::Zsh => ClapShell::Zsh,
        Shell::Fish => ClapShell::Fish,
        Shell::PowerShell => ClapShell::PowerShell,
        Shell::Elvish => ClapShell::Elvish,
    };

    generate(shell, &mut cmd, "arkouda", &mut io::stdout());
    Ok(0)
}
