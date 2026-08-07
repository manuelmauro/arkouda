//! Markdown structure extraction over a concept body.
//!
//! Bodies are parsed with a CommonMark parser rather than scanned for `## `
//! line prefixes. Two things follow that a scanner cannot get right: a heading
//! inside a fenced or indented code block is not a heading, and a section runs
//! until the next heading of the same or higher level rather than until the
//! next `##`.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// Heading level at which concept sections live: `## Decision`, not `# Title`
/// and not `### Positive`.
pub const SECTION_LEVEL: u8 = 2;

/// Heading level of a concept's title heading.
pub const TITLE_LEVEL: u8 = 1;

/// A Markdown heading found in a concept body.
#[derive(Debug, Clone)]
pub struct Heading {
    /// Heading level: 1 for `#`, 2 for `##`, and so on.
    pub level: u8,

    /// Heading text with inline markup flattened, trimmed. A closing `#`
    /// sequence is removed by the parser, so `## Goals ##` yields `Goals`.
    pub text: String,

    /// Byte range of the whole heading within the body.
    pub span: Range<usize>,
}

impl Heading {
    /// Whether this heading names `title`, ignoring case and surrounding
    /// whitespace.
    pub fn is_named(&self, title: &str) -> bool {
        self.text.eq_ignore_ascii_case(title.trim())
    }
}

/// Parser options. Tables are enabled because arkouda's own concepts use them,
/// and a row starting with `|` must not be parsed as some other construct.
fn options() -> Options {
    Options::ENABLE_TABLES
}

/// Every heading in `body`, in document order.
pub fn headings(body: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut open: Option<(u8, Range<usize>, String)> = None;

    for (event, span) in Parser::new_ext(body, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                open = Some((level_number(level), span, String::new()));
            }
            // Inline markup is flattened: a heading may legitimately contain
            // code spans or emphasis, and callers match on its plain text.
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, buffer)) = open.as_mut() {
                    buffer.push_str(&text);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((level, span, text)) = open.take() {
                    headings.push(Heading {
                        level,
                        text: text.trim().to_owned(),
                        span,
                    });
                }
            }
            _ => {}
        }
    }

    headings
}

/// Body of the `## <name>` section of `body`, with surrounding blank lines
/// trimmed, or `None` when no such section exists.
///
/// The section ends at the next heading of the same or higher level, so a
/// nested `###` belongs to the section that contains it while an intervening
/// `#` ends it.
pub fn section(body: &str, name: &str) -> Option<String> {
    let headings = headings(body);
    let index = headings
        .iter()
        .position(|heading| heading.level == SECTION_LEVEL && heading.is_named(name))?;

    let start = headings[index].span.end;
    let end = headings[index + 1..]
        .iter()
        .find(|heading| heading.level <= SECTION_LEVEL)
        .map_or(body.len(), |heading| heading.span.start);

    Some(body[start..end].trim().to_owned())
}

/// Zero-based index of the line containing byte `offset` in `body`.
pub fn line_index(body: &str, offset: usize) -> usize {
    body.as_bytes()[..offset.min(body.len())]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
}

fn level_number(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_headings_with_levels() {
        let body = "# Title\n\n## Status\n\nAccepted\n\n### Nested\n";
        let found = headings(body);
        let levels: Vec<u8> = found.iter().map(|heading| heading.level).collect();
        let texts: Vec<&str> = found.iter().map(|heading| heading.text.as_str()).collect();
        assert_eq!(levels, [1, 2, 3]);
        assert_eq!(texts, ["Title", "Status", "Nested"]);
    }

    #[test]
    fn headings_inside_a_fence_are_not_headings() {
        let body = "# Title\n\n## Context\n\n```markdown\n# Not a title\n\n## Not a section\n```\n";
        let texts: Vec<String> = headings(body)
            .into_iter()
            .map(|heading| heading.text)
            .collect();
        assert_eq!(texts, ["Title", "Context"], "a fenced heading is code");
    }

    #[test]
    fn headings_inside_an_indented_block_are_not_headings() {
        let body = "# Title\n\n## Context\n\n    ## Indented code\n";
        let texts: Vec<String> = headings(body)
            .into_iter()
            .map(|heading| heading.text)
            .collect();
        assert_eq!(texts, ["Title", "Context"]);
    }

    #[test]
    fn setext_headings_are_recognized() {
        let body = "Title\n=====\n\nStatus\n------\n";
        let found = headings(body);
        let levels: Vec<u8> = found.iter().map(|heading| heading.level).collect();
        let texts: Vec<&str> = found.iter().map(|heading| heading.text.as_str()).collect();
        assert_eq!(levels, [1, 2]);
        assert_eq!(texts, ["Title", "Status"]);
    }

    #[test]
    fn inline_markup_in_a_heading_is_flattened() {
        let body = "## The `index.md` *file*\n";
        assert_eq!(headings(body)[0].text, "The index.md file");
    }

    #[test]
    fn a_closing_hash_sequence_is_not_part_of_the_text() {
        assert_eq!(headings("## Goals ##\n")[0].text, "Goals");
    }

    #[test]
    fn section_stops_at_the_next_same_level_heading() {
        let body = "## Decision\n\nWe will adopt X.\n\nIt scales.\n\n## Consequences\n\nFaster.\n";
        assert_eq!(
            section(body, "decision").as_deref(),
            Some("We will adopt X.\n\nIt scales.")
        );
    }

    #[test]
    fn section_stops_at_a_higher_level_heading() {
        let body = "## Goals\n\nShip it.\n\n# Solution Alignment\n\n## Key Features\n\nA feed.\n";
        assert_eq!(
            section(body, "goals").as_deref(),
            Some("Ship it."),
            "an intervening `#` ends the section"
        );
    }

    #[test]
    fn section_keeps_nested_subsections() {
        let body = "## Consequences\n\n### Positive\n\nGood.\n\n### Negative\n\nBad.\n\n## Next\n";
        let extracted = section(body, "consequences").expect("section present");
        assert!(extracted.contains("### Positive"), "{extracted}");
        assert!(extracted.contains("### Negative"), "{extracted}");
        assert!(!extracted.contains("## Next"), "{extracted}");
    }

    #[test]
    fn section_keeps_a_fenced_template_intact() {
        let body = "## Decision\n\nUse this shape:\n\n```markdown\n## Problem\n\n## Requirements\n```\n\nThat is all.\n\n## Consequences\n\nDone.\n";
        let extracted = section(body, "decision").expect("section present");
        assert!(
            extracted.contains("## Requirements") && extracted.ends_with("That is all."),
            "the fence must not truncate the section: {extracted}"
        );
    }

    #[test]
    fn section_matching_ignores_case_and_returns_none_when_absent() {
        let body = "## Decision\n\nX.\n";
        assert_eq!(section(body, "DECISION").as_deref(), Some("X."));
        assert_eq!(section(body, "missing"), None);
    }

    #[test]
    fn a_title_heading_is_not_a_section() {
        let body = "# Decision\n\nX.\n";
        assert_eq!(
            section(body, "decision"),
            None,
            "sections live at level 2 only"
        );
    }

    #[test]
    fn line_index_counts_newlines_before_the_offset() {
        let body = "a\nb\nc";
        assert_eq!(line_index(body, 0), 0);
        assert_eq!(line_index(body, 2), 1);
        assert_eq!(line_index(body, 4), 2);
        assert_eq!(line_index(body, usize::MAX), 2, "offsets are clamped");
    }
}
