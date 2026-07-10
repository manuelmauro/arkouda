//! List ADR concepts in the collection.

use crate::adr::Manifest;
use crate::cli::{Cli, ListArgs, SortBy};
use crate::error::Result;

/// Run the list command.
pub fn run(args: &ListArgs, cli: &Cli) -> Result<i32> {
    let dirs = super::effective_dirs(cli)?;
    let mut manifests = super::load_manifests(&dirs)?;
    sort_manifests(&mut manifests, args.sort);
    if args.long {
        print_long(&manifests);
    } else {
        print_paths(&manifests);
    }
    Ok(0)
}

fn sort_manifests(manifests: &mut [Manifest], sort: SortBy) {
    manifests.sort_by(|left, right| {
        let primary = match sort {
            SortBy::Id => left.concept_id.cmp(&right.concept_id),
            SortBy::Timestamp => (
                left.frontmatter.display_timestamp(),
                left.concept_id.as_str(),
            )
                .cmp(&(
                    right.frontmatter.display_timestamp(),
                    right.concept_id.as_str(),
                )),
            SortBy::Status => (left.frontmatter.display_status(), left.concept_id.as_str()).cmp(&(
                right.frontmatter.display_status(),
                right.concept_id.as_str(),
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
        .map(|manifest| manifest.concept_id.len())
        .max()
        .unwrap_or(0);
    let status_width = manifests
        .iter()
        .map(|manifest| manifest.frontmatter.display_status().len())
        .max()
        .unwrap_or(0);
    let timestamp_width = manifests
        .iter()
        .map(|manifest| manifest.frontmatter.display_timestamp().len())
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
        let description = truncate(frontmatter.display_description(), 90);
        println!(
            "{:<id_width$}  {:<status_width$}  {:<timestamp_width$}  {:<path_width$}  {} — {}",
            manifest.concept_id,
            frontmatter.display_status(),
            frontmatter.display_timestamp(),
            manifest.path.display(),
            title,
            description,
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
