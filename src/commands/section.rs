//! Print one concept's primary section, or another named section.
//!
//! Extraction earns its subcommand because a shell pipeline cannot do it
//! correctly: `awk '/^## Decision/{f=1;next} /^## /{f=0} f'` stops at the
//! first heading inside a fenced code block, and runs past an intervening `#`.
//! Arkouda parses the Markdown instead.

use crate::cli::{Cli, SectionArgs};
use crate::commands::Outcome;
use crate::error::{ArkoudaError, Result};

/// Run the section command.
pub fn run(args: &SectionArgs, cli: &Cli) -> Result<Outcome> {
    let dirs = super::search_dirs(cli)?;
    let manifests = super::load_manifests(&dirs)?;
    let manifest = super::resolve_one(&manifests, &args.id)?;

    // With no name, the concept's own type says which section carries its
    // substance: `Decision` for an ADR, `Requirements` for a PRD. A type that
    // names no primary section has no default to fall back on, and neither
    // does a type arkouda does not know.
    let name = match args.name.as_deref() {
        Some(name) => name.to_owned(),
        None => {
            let concept_type =
                manifest
                    .concept_type()
                    .ok_or_else(|| ArkoudaError::UnknownConceptType {
                        id: manifest.concept_id.clone(),
                        concept_type: manifest.frontmatter.display_type().to_owned(),
                    })?;
            concept_type
                .primary_section
                .clone()
                .ok_or_else(|| ArkoudaError::NoPrimarySection {
                    id: manifest.concept_id.clone(),
                    slug: concept_type.slug.clone(),
                })?
        }
    };

    let body = manifest
        .section(&name)
        .ok_or_else(|| ArkoudaError::SectionNotFound {
            id: manifest.concept_id.clone(),
            section: name.clone(),
        })?;
    println!("{body}");

    // The concept's own type is what picked the section, so it is the type
    // this invocation resolved.
    Ok(Outcome {
        exit: 0,
        codes: Vec::new(),
        type_kind: manifest
            .concept_type()
            .map(|concept_type| concept_type.origin),
    })
}
