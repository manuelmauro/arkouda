//! Rendering of the OKF §6 `index.md` directory listing.
//!
//! The index exists for progressive disclosure: it lets a human or an agent
//! see every decision in the bundle, grouped by status, without opening a
//! single concept document. Bundle-root indexes also declare the OKF version
//! the bundle targets (OKF §11) — the one place frontmatter is permitted in an
//! `index.md`.

use crate::adr::{AdrStatus, Manifest, OKF_VERSION};

/// Heading used for concepts whose status is missing or unrecognized. OKF
/// consumers must tolerate both, so they get a bucket rather than an error.
const OTHER_HEADING: &str = "Other";

/// Render the bundle-root `index.md` for `manifests`.
pub fn render(manifests: &[Manifest]) -> String {
    let mut out = format!("---\nokf_version: \"{OKF_VERSION}\"\n---\n");

    for (heading, group) in grouped(manifests) {
        out.push_str(&format!("\n# {heading}\n\n"));
        for manifest in group {
            out.push_str(&entry(manifest));
        }
    }

    out
}

/// Group concepts under their status heading, in lifecycle order, dropping
/// empty groups. Concepts sort by concept id within a group.
fn grouped(manifests: &[Manifest]) -> Vec<(String, Vec<&Manifest>)> {
    let mut groups: Vec<(String, Vec<&Manifest>)> = AdrStatus::ALL
        .iter()
        .map(|status| (status.label().to_string(), Vec::new()))
        .chain(std::iter::once((OTHER_HEADING.to_owned(), Vec::new())))
        .collect();

    for manifest in manifests {
        let heading = manifest
            .frontmatter
            .status
            .as_deref()
            .and_then(|status| status.trim().parse::<AdrStatus>().ok())
            .map_or(OTHER_HEADING.to_owned(), |status| {
                status.label().to_string()
            });

        let group = groups
            .iter_mut()
            .find(|(name, _)| *name == heading)
            .expect("every heading is pre-seeded");
        group.1.push(manifest);
    }

    for (_, group) in &mut groups {
        group.sort_by(|left, right| left.concept_id.cmp(&right.concept_id));
    }

    groups.retain(|(_, group)| !group.is_empty());
    groups
}

/// One `* [Title](path) - description` list entry. The description is omitted
/// when absent rather than rendered as a placeholder.
fn entry(manifest: &Manifest) -> String {
    let title = manifest
        .frontmatter
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(&manifest.concept_id);

    let link = format!("{}.md", manifest.concept_id);

    match manifest
        .frontmatter
        .description
        .as_deref()
        .map(str::trim)
        .filter(|description| !description.is_empty())
    {
        Some(description) => format!("* [{title}]({link}) - {description}\n"),
        None => format!("* [{title}]({link})\n"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn manifest(id: &str, title: &str, status: &str, description: Option<&str>) -> Manifest {
        let description = description
            .map(|text| format!("description: {text}\n"))
            .unwrap_or_default();
        let content = format!(
            "---\ntype: Architecture Decision Record\ntitle: {title}\n{description}status: {status}\n---\n\n# {title}\n"
        );
        Manifest::parse_content(
            &Path::new("docs/adr").join(format!("{id}.md")),
            Path::new("docs/adr"),
            &content,
        )
        .expect("valid manifest")
    }

    #[test]
    fn renders_status_groups_in_lifecycle_order() {
        let manifests = vec![
            manifest("zeta", "Zeta", "accepted", Some("Third.")),
            manifest("alpha", "Alpha", "proposed", Some("First.")),
            manifest("beta", "Beta", "accepted", Some("Second.")),
        ];

        let rendered = render(&manifests);

        assert_eq!(
            rendered,
            "---\nokf_version: \"0.1\"\n---\n\
             \n# Proposed\n\n\
             * [Alpha](alpha.md) - First.\n\
             \n# Accepted\n\n\
             * [Beta](beta.md) - Second.\n\
             * [Zeta](zeta.md) - Third.\n"
        );
    }

    #[test]
    fn unknown_and_missing_statuses_land_in_other() {
        let manifests = vec![
            manifest("weird", "Weird", "invented", None),
            manifest("known", "Known", "accepted", None),
        ];

        let rendered = render(&manifests);
        assert!(rendered.contains("# Accepted\n\n* [Known](known.md)\n"));
        assert!(rendered.contains("# Other\n\n* [Weird](weird.md)\n"));
    }

    #[test]
    fn nested_concepts_link_by_bundle_relative_path() {
        let manifests = vec![manifest("security/mtls", "mTLS", "accepted", None)];
        assert!(render(&manifests).contains("* [mTLS](security/mtls.md)\n"));
    }

    #[test]
    fn an_empty_bundle_still_declares_the_okf_version() {
        assert_eq!(render(&[]), "---\nokf_version: \"0.1\"\n---\n");
    }
}
