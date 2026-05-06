//! List ADRs in the collection.

use crate::adr::Manifest;
use crate::cli::{Cli, ListArgs, SortBy};
use crate::error::Result;
use std::process::ExitCode;

/// Run the list command.
pub fn run(args: &ListArgs, cli: &Cli) -> Result<ExitCode> {
    let mut manifests = super::load_manifests(&cli.dir)?;
    sort_manifests(&mut manifests, args.sort);
    match args.section.as_deref() {
        Some(section) => print_section_digest(&manifests, section),
        None => print_manifest_rows(&manifests),
    }
    Ok(ExitCode::SUCCESS)
}

fn print_section_digest(manifests: &[Manifest], section: &str) {
    let mut first = true;
    for manifest in manifests {
        let Some(body) = manifest.section(section) else {
            continue;
        };
        if !first {
            println!();
        }
        first = false;

        let id = manifest.frontmatter.display_id();
        let title = manifest.frontmatter.display_title();
        println!("## {id}: {title}\n\n{body}");
    }
}

pub(crate) fn sort_manifests(manifests: &mut [Manifest], sort: SortBy) {
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

pub(crate) fn print_manifest_rows(manifests: &[Manifest]) {
    let id_width = manifests
        .iter()
        .map(|manifest| manifest.frontmatter.display_id().len())
        .max()
        .unwrap_or(2)
        .max(2);
    let status_width = manifests
        .iter()
        .map(|manifest| manifest.frontmatter.display_status().len())
        .max()
        .unwrap_or(6)
        .max(6);

    println!(
        "{:<id_width$}  {:<status_width$}  {:<10}  TITLE",
        "ID", "STATUS", "DATE"
    );

    for manifest in manifests {
        let frontmatter = &manifest.frontmatter;
        let title = frontmatter.display_title();
        let abstract_text = truncate(frontmatter.display_abstract(), 90);
        println!(
            "{:<id_width$}  {:<status_width$}  {:<10}  {} — {}",
            frontmatter.display_id(),
            frontmatter.display_status(),
            frontmatter.display_date(),
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
