//! List ADRs in the collection.

use crate::adr::Manifest;
use crate::cli::{Cli, ListArgs, SortBy};
use crate::error::Result;

/// Run the list command.
pub fn run(args: ListArgs, cli: &Cli) -> Result<i32> {
    let mut manifests = super::load_manifests(&cli.dir)?;
    sort_manifests(&mut manifests, args.sort);
    print_manifest_rows(&manifests);
    Ok(0)
}

pub(crate) fn sort_manifests(manifests: &mut [Manifest], sort: SortBy) {
    manifests.sort_by(|left, right| {
        let left_key = sort_key(left, sort);
        let right_key = sort_key(right, sort);
        left_key
            .cmp(&right_key)
            .then_with(|| left.path.cmp(&right.path))
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

fn sort_key(manifest: &Manifest, sort: SortBy) -> String {
    match sort {
        SortBy::Id => manifest.frontmatter.display_id().to_string(),
        SortBy::Date => format!(
            "{}\u{0}{}",
            manifest.frontmatter.display_date(),
            manifest.frontmatter.display_id()
        ),
        SortBy::Status => format!(
            "{}\u{0}{}",
            manifest.frontmatter.display_status(),
            manifest.frontmatter.display_id()
        ),
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
