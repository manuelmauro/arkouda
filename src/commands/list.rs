//! List ADRs in the collection.

use crate::adr::Manifest;
use crate::cli::{Cli, ListArgs, SortBy};
use crate::error::Result;
use std::process::ExitCode;

/// Run the list command.
pub fn run(args: &ListArgs, cli: &Cli) -> Result<ExitCode> {
    let dirs = super::effective_dirs(cli)?;
    let mut manifests = super::load_manifests(&dirs)?;
    sort_manifests(&mut manifests, args.sort);
    if args.long {
        print_long(&manifests);
    } else {
        print_paths(&manifests);
    }
    Ok(ExitCode::SUCCESS)
}

fn sort_manifests(manifests: &mut [Manifest], sort: SortBy) {
    manifests.sort_by(|left, right| {
        let primary = match sort {
            SortBy::Id => left
                .frontmatter
                .display_id()
                .cmp(right.frontmatter.display_id()),
            SortBy::Date => (
                left.frontmatter.display_date(),
                left.frontmatter.display_id(),
            )
                .cmp(&(
                    right.frontmatter.display_date(),
                    right.frontmatter.display_id(),
                )),
            SortBy::Status => (
                left.frontmatter.display_status(),
                left.frontmatter.display_id(),
            )
                .cmp(&(
                    right.frontmatter.display_status(),
                    right.frontmatter.display_id(),
                )),
        };
        primary.then_with(|| left.path.cmp(&right.path))
    });
}

fn print_paths(manifests: &[Manifest]) {
    for manifest in manifests {
        println!("{}", manifest.path.display());
    }
}

fn print_long(manifests: &[Manifest]) {
    let id_width = manifests
        .iter()
        .map(|manifest| manifest.frontmatter.display_id().len())
        .max()
        .unwrap_or(0);
    let status_width = manifests
        .iter()
        .map(|manifest| manifest.frontmatter.display_status().len())
        .max()
        .unwrap_or(0);
    let path_width = manifests
        .iter()
        .map(|manifest| manifest.path.display().to_string().len())
        .max()
        .unwrap_or(0);

    for manifest in manifests {
        let frontmatter = &manifest.frontmatter;
        let title = frontmatter.display_title();
        let abstract_text = truncate(frontmatter.display_abstract(), 90);
        println!(
            "{:<id_width$}  {:<status_width$}  {:<10}  {:<path_width$}  {} — {}",
            frontmatter.display_id(),
            frontmatter.display_status(),
            frontmatter.display_date(),
            manifest.path.display(),
            title,
            abstract_text,
        );
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}
