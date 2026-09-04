use arkouda::cli::{Cli, Command, SelfCommand};
use arkouda::commands::Outcome;
use arkouda::telemetry::{Event, Telemetry};
use arkouda::{Result, commands, config};
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let argv: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let telemetry = Telemetry::from_env(cli.quiet);
    let start = Instant::now();

    let result = run(&cli);
    let elapsed = start.elapsed();

    let (outcome, error_message) = match result {
        Ok(outcome) => (outcome, None),
        Err(error) => (Outcome::from(1), Some(error.to_string())),
    };
    let exit_int = outcome.exit;

    if cli.command.is_recorded() {
        telemetry.record(&Event::capture(&cli, &argv, &outcome, elapsed));
    }

    if let Some(message) = error_message {
        eprintln!("{} {}", "error:".red().bold(), message);
        return ExitCode::FAILURE;
    }
    if exit_int == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn run(cli: &Cli) -> Result<Outcome> {
    // Everything below reads the concept-type registry — `dirs` resolution, a
    // `--type` slug, every diagnostic — so it has to exist first. Completion
    // generation is the exception: shells run it from their startup file, and
    // a malformed config in some directory must not stop a shell from opening.
    if !matches!(cli.command, Command::SelfCmd(_)) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        config::install_types(&cwd)?;
    }

    match &cli.command {
        Command::List(args) => commands::list::run(args, cli),
        Command::Section(args) => commands::section::run(args, cli),
        Command::Check => commands::check::run(cli),
        Command::New(args) => commands::new::run(args, cli),
        Command::Index => commands::index::run(cli),
        Command::SelfCmd(args) => match &args.command {
            SelfCommand::Completions(args) => commands::completions::run(args),
        },
    }
}
