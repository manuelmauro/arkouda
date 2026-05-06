use arkouda::cli::{Cli, Command};
use arkouda::{Result, commands};
use clap::Parser;
use colored::Colorize;

fn main() {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("{} {}", "error:".red().bold(), error);
            std::process::exit(1);
        }
    }
}

fn run(cli: &Cli) -> Result<i32> {
    match &cli.command {
        Command::List(args) => commands::list::run(args.clone(), cli),
        Command::Show(args) => commands::show::run(args.clone(), cli),
        Command::Search(args) => commands::search::run(args.clone(), cli),
        Command::Check(args) => commands::check::run(args.clone(), cli),
        Command::New(args) => commands::new::run(args.clone(), cli),
    }
}
