//! Rendering of the OKF §8 `index.md` directory listing.
//!
//! The index exists for progressive disclosure: it lets a human or an agent
//! see every concept in the bundle, grouped by type and then by status,
//! without opening a single document. Bundle-root indexes also declare the OKF
//! version the bundle targets (OKF §12) — the one place frontmatter is
//! permitted in an `index.md`.
//!
//! Nesting is uniform: a single-type bundle still pays the type heading, so
//! consumers parse one shape instead of two.

use crate::concept::types::{self, ConceptType};
use crate::concept::{Manifest, OKF_VERSION};

/// Heading used for concepts whose type or status is missing or unrecognized.
/// OKF consumers must tolerate both, so they get a bucket rather than an error.
const OTHER_HEADING: &str = "Other";

/// Render the bundle-root `index.md` for `manifests`.
pub fn render(manifests: &[Manifest]) -> String {
    let mut out = format!("---\nokf_version: \"{OKF_VERSION}\"\n---\n");

    for (concept_type, group) in by_type(manifests) {
        out.push_str(&format!("\n# {concept_type}\n"));
        for (status, entries) in by_status(concept_type, &group) {
            out.push_str(&format!("\n## {status}\n\n"));
            for manifest in entries {
                out.push_str(&entry(manifest));
            }
        }
    }

    out
}

/// Partition concepts under their type heading, in registry order, with
/// unrecognized and missing types last. Empty groups are dropped.
fn by_type(manifests: &[Manifest]) -> Vec<(&str, Vec<&Manifest>)> {
    let headings = types::all()
        .iter()
        .map(|concept_type| concept_type.okf_type.as_str())
        .chain(std::iter::once(OTHER_HEADING));

    let mut groups: Vec<(&str, Vec<&Manifest>)> =
        headings.map(|heading| (heading, Vec::new())).collect();

    for manifest in manifests {
        let heading = manifest
            .concept_type()
            .map_or(OTHER_HEADING, |concept_type| concept_type.okf_type.as_str());
        let group = groups
            .iter_mut()
            .find(|(name, _)| *name == heading)
            .expect("every heading is pre-seeded");
        group.1.push(manifest);
    }

    groups.retain(|(_, group)| !group.is_empty());
    groups
}

/// Group one type's concepts under their status heading, in lifecycle order,
/// dropping empty groups. Concepts sort by concept id within a group.
///
/// Concepts of an unrecognized type have no vocabulary to group by, so they
/// all land under `Other`.
fn by_status<'a>(
    concept_type: &str,
    manifests: &[&'a Manifest],
) -> Vec<(&'static str, Vec<&'a Manifest>)> {
    let descriptor: Option<&'static ConceptType> = types::by_okf_type(concept_type);
    let labels = descriptor
        .into_iter()
        .flat_map(|descriptor| descriptor.statuses.iter())
        .map(|status| status.label.as_str());

    let mut groups: Vec<(&'static str, Vec<&Manifest>)> = labels
        .chain(std::iter::once(OTHER_HEADING))
        .map(|heading| (heading, Vec::new()))
        .collect();

    for manifest in manifests {
        let heading = descriptor
            .and_then(|descriptor| {
                let lifecycle = manifest.frontmatter.resolved_lifecycle(Some(descriptor))?;
                descriptor.status(lifecycle)
            })
            .map_or(OTHER_HEADING, |status| status.label.as_str());

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

    fn concept(
        id: &str,
        concept_type: &str,
        title: &str,
        status: &str,
        description: Option<&str>,
    ) -> Manifest {
        let description = description
            .map(|text| format!("description: {text}\n"))
            .unwrap_or_default();
        let content = format!(
            "---\ntype: {concept_type}\ntitle: {title}\n{description}status: {status}\n---\n\n# {title}\n"
        );
        Manifest::parse_content(
            &Path::new("docs/adr").join(format!("{id}.md")),
            Path::new("docs/adr"),
            &content,
        )
        .expect("valid manifest")
    }

    fn adr(id: &str, title: &str, status: &str, description: Option<&str>) -> Manifest {
        concept(id, &types::adr().okf_type, title, status, description)
    }

    fn prd(id: &str, title: &str, status: &str, description: Option<&str>) -> Manifest {
        concept(id, &types::prd().okf_type, title, status, description)
    }

    #[test]
    fn renders_type_groups_then_status_groups_in_lifecycle_order() {
        let manifests = vec![
            adr("zeta", "Zeta", "accepted", Some("Third.")),
            prd("import", "Import", "draft", Some("Fourth.")),
            adr("alpha", "Alpha", "proposed", Some("First.")),
            adr("beta", "Beta", "accepted", Some("Second.")),
        ];

        assert_eq!(
            render(&manifests),
            "---\nokf_version: \"0.2\"\n---\n\
             \n# Architecture Decision Record\n\
             \n## Proposed\n\n\
             * [Alpha](alpha.md) - First.\n\
             \n## Accepted\n\n\
             * [Beta](beta.md) - Second.\n\
             * [Zeta](zeta.md) - Third.\n\
             \n# Product Requirements Document\n\
             \n## Draft\n\n\
             * [Import](import.md) - Fourth.\n"
        );
    }

    #[test]
    fn a_single_type_bundle_still_gets_its_type_heading() {
        let rendered = render(&[adr("alpha", "Alpha", "accepted", None)]);
        assert!(
            rendered.contains("# Architecture Decision Record\n\n## Accepted\n"),
            "uniform nesting beats conditional nesting: {rendered}"
        );
    }

    #[test]
    fn unknown_and_missing_statuses_land_in_other() {
        let manifests = vec![
            adr("weird", "Weird", "invented", None),
            adr("known", "Known", "accepted", None),
            // `shipped` is a PRD status, so it is unknown to an ADR.
            adr("borrowed", "Borrowed", "shipped", None),
        ];

        let rendered = render(&manifests);
        assert!(rendered.contains("## Accepted\n\n* [Known](known.md)\n"));
        assert!(rendered.contains("## Other\n\n* [Borrowed](borrowed.md)\n* [Weird](weird.md)\n"));
    }

    #[test]
    fn an_unknown_type_gets_the_other_heading_and_is_listed_last() {
        let manifests = vec![
            concept("table", "BigQuery Table", "Table", "accepted", None),
            adr("known", "Known", "accepted", None),
        ];

        let rendered = render(&manifests);
        let known = rendered.find("[Known]").expect("listed");
        let table = rendered.find("[Table]").expect("listed");
        assert!(known < table, "{rendered}");
        assert!(rendered.contains("# Other\n\n## Other\n\n* [Table](table.md)\n"));
    }

    #[test]
    fn nested_concepts_link_by_bundle_relative_path() {
        let manifests = vec![adr("security/mtls", "mTLS", "accepted", None)];
        assert!(render(&manifests).contains("* [mTLS](security/mtls.md)\n"));
    }

    #[test]
    fn an_empty_bundle_still_declares_the_okf_version() {
        assert_eq!(render(&[]), "---\nokf_version: \"0.2\"\n---\n");
    }
}
