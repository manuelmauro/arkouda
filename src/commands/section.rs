//! Print one concept's primary section, or another named section.
//!
//! Extraction earns its subcommand because a shell pipeline cannot do it
//! correctly: `awk '/^## Decision/{f=1;next} /^## /{f=0} f'` stops at the
//! first heading inside a fenced code block, and runs past an intervening `#`.
//! Arkouda parses the Markdown instead.

use crate::cli::{Cli, SectionArgs};
use crate::error::{ArkoudaError, Result};

/// Run the section command.
pub fn run(args: &SectionArgs, cli: &Cli) -> Result<i32> {
    let dirs = super::search_dirs(cli)?;
    let manifests = super::load_manifests(&dirs)?;
    let manifest = super::resolve_one(&manifests, &args.id)?;

    // With no name, the concept's own type says which section carries its
    // substance: `Decision` for an ADR, `Requirements` for a PRD.
    let name = match args.name.as_deref() {
        Some(name) => name.to_owned(),
        None => manifest
            .concept_type()
            .ok_or_else(|| ArkoudaError::UnknownConceptType {
                id: manifest.concept_id.clone(),
                concept_type: manifest.frontmatter.display_type().to_owned(),
            })?
            .primary_section
            .to_owned(),
    };

    let body = manifest
        .section(&name)
        .ok_or_else(|| ArkoudaError::SectionNotFound {
            id: manifest.concept_id.clone(),
            section: name.clone(),
        })?;
    println!("{body}");

    Ok(0)
}
