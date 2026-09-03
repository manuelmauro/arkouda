//! List concepts in the collection.

use crate::cli::{Cli, ListArgs, SortBy};
use crate::concept::Manifest;
use crate::concept::types;
use crate::error::Result;

/// Run the list command.
pub fn run(args: &ListArgs, cli: &Cli) -> Result<i32> {
    let dirs = super::search_dirs(cli)?;
    let mut manifests = super::load_manifests(&dirs)?;
    filter_by_type(&mut manifests, args.concept_type.as_deref());
    sort_manifests(&mut manifests, args.sort);
    if args.long {
        print_long(&manifests);
    } else {
        print_paths(&manifests);
    }
    Ok(0)
}

/// Keep only concepts declaring `slug`'s type. The filter reads frontmatter,
/// not paths: a bundle may hold any mix.
fn filter_by_type(manifests: &mut Vec<Manifest>, slug: Option<&str>) {
    let Some(wanted) = slug.and_then(types::by_slug) else {
        return;
    };
    manifests.retain(|manifest| manifest.concept_type() == Some(wanted));
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
    let width = |value: fn(&Manifest) -> usize| manifests.iter().map(value).max().unwrap_or(0);

    let id_width = width(|manifest| manifest.concept_id.len());
    let type_width = width(|manifest| manifest.frontmatter.display_type_slug().len());
    let status_width = width(|manifest| manifest.frontmatter.display_status().len());
    let timestamp_width = width(|manifest| manifest.frontmatter.display_timestamp().len());
    let path_width = width(|manifest| manifest.path.display().to_string().len());

    for manifest in manifests {
        let frontmatter = &manifest.frontmatter;
        let title = frontmatter.display_title();
        let description = truncate(frontmatter.display_description(), 90);
        println!(
            "{:<id_width$}  {:<type_width$}  {:<status_width$}  {:<timestamp_width$}  {:<path_width$}  {} — {}",
            manifest.concept_id,
            frontmatter.display_type_slug(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn manifest(id: &str, concept_type: &str) -> Manifest {
        let content =
            format!("---\ntype: {concept_type}\ntitle: {id}\nstatus: draft\n---\n\n# {id}\n");
        Manifest::parse_content(
            &Path::new("docs/adr").join(format!("{id}.md")),
            Path::new("docs/adr"),
            &content,
        )
        .expect("valid manifest")
    }

    fn ids(manifests: &[Manifest]) -> Vec<&str> {
        manifests
            .iter()
            .map(|manifest| manifest.concept_id.as_str())
            .collect()
    }

    #[test]
    fn the_type_filter_reads_frontmatter_not_paths() {
        // Both concepts sit in the same bundle; only `type` separates them.
        let mut manifests = vec![
            manifest("a-decision", types::ADR.okf_type),
            manifest("a-requirement", types::PRD.okf_type),
            manifest("a-table", "BigQuery Table"),
        ];

        let mut prds = manifests.clone();
        filter_by_type(&mut prds, Some("prd"));
        assert_eq!(ids(&prds), ["a-requirement"]);

        filter_by_type(&mut manifests, None);
        assert_eq!(ids(&manifests).len(), 3, "no filter keeps everything");
    }

    #[test]
    fn the_type_column_is_a_single_token() {
        // `list -l` is awk-sliceable, so the type column must never be the
        // multi-word OKF type string.
        for manifest in [
            manifest("a", types::ADR.okf_type),
            manifest("b", "BigQuery Table"),
        ] {
            let column = manifest.frontmatter.display_type_slug();
            assert!(
                !column.chars().any(char::is_whitespace),
                "type column `{column}` would shift every later column"
            );
        }
    }
}
