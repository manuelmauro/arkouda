use arkouda::cli::{Cli, Command};
use arkouda::{Result, commands};
use clap::Parser;
use colored::Colorize;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{} {}", "error:".red().bold(), error);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<ExitCode> {
    match &cli.command {
        Command::List(args) => commands::list::run(args, cli),
        Command::Show(args) => commands::show::run(args, cli),
        Command::Check => commands::check::run(cli),
        Command::New(args) => commands::new::run(args, cli),
    }
}
