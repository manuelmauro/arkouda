use arkouda::cli::{Cli, Command};
use arkouda::telemetry::{Event, Telemetry};
use arkouda::{Result, commands};
use clap::Parser;
use colored::Colorize;
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

    let (exit_int, error_message) = match result {
        Ok(code) => (code, None),
        Err(error) => (1, Some(error.to_string())),
    };

    telemetry.record(&Event::capture(&cli, &argv, exit_int, elapsed));

    if let Some(message) = error_message {
        eprintln!("{} {}", "error:".red().bold(), message);
        return ExitCode::FAILURE;
    }
    match u8::try_from(exit_int) {
        Ok(code) => ExitCode::from(code),
        Err(_) => ExitCode::FAILURE,
    }
}

fn run(cli: &Cli) -> Result<i32> {
    match &cli.command {
        Command::List(args) => commands::list::run(args, cli),
        Command::Decision(args) => commands::decision::run(args, cli),
        Command::Check => commands::check::run(cli),
        Command::New(args) => commands::new::run(args, cli),
    }
}
