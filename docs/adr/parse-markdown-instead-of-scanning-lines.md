---
type: Architecture Decision Record
title: Parse Markdown instead of scanning lines
description: Replace hand-rolled line scanning with a CommonMark parser (pulldown-cmark), and keep the document-schema layer in-house rather than delegating to an external Markdown schema validator.
tags:
  - parsing
  - validation
  - dependencies
generated: { by: human:manuelmauro, at: 2026-08-07T00:00:00Z }
status: draft
lifecycle: proposed
deciders: []
---

# Parse Markdown instead of scanning lines

## Status

Proposed

## Context

Arkouda decides what a document's structure is by scanning raw lines for a heading prefix. Three places do it:

- `check_required_sections` collects `line.strip_prefix("## ")` into a set and checks the required section names are present.
- `Manifest::section` finds the matching `## ` line, then takes lines `take_while(|line| !line.starts_with("## "))`.
- `check_title_heading` finds the first `# ` line and compares it to `title`.

This is not parsing. Two defects follow, both verified against v0.5.0:

**Fenced code blocks are invisible.** A heading inside a fence counts as a real heading. An ADR whose only `## Decision` and `## Consequences` headings sit inside a `` ```markdown `` fence passes `arkouda check` with no diagnostics — the validator reports four required sections present when the document has two.

**Section bodies swallow other heading levels.** `section` stops only at `## `, so an `#` or `###` between two `##` headings becomes part of the preceding section's body. Extracting a section from a document with any heading nesting returns the wrong text.

There is a third variant of the first defect that a scanner and a naive parser share: a heading nested in a **block container** is not a section of the document either. `> ## Decision` inside a block quote, or `- ## Decision` inside a list item, is quoted or listed content. Quoting a template must not satisfy the requirement to have written one.

Both defects bite hardest when a document explains a document format — because that means showing headings. [Supporting PRDs](https://github.com/manuelmauro/arkouda/pull/11) carries a PRD template in a fence, and demonstrates both: `arkouda decision support-product-requirements-documents` truncates at the template's first heading, and the template's headings are indistinguishable from the ADR's own.

The scanner is also wrong in quieter ways. Setext headings (`Title` over `=====`) are not recognized. Indented code blocks have the same problem as fenced ones. `#Heading` without a space is correctly ignored, but only by accident. None of this has been reported, because arkouda's own ADRs happen not to exercise it — which is exactly how a bug class survives.

Fixing the reported case is a few lines: track whether a fence is open. The question this raises is bigger than the fix. **Should arkouda own this layer at all, or is there a production-ready Markdown schema tool to delegate to?**

A survey of that space (August 2026):

| Project                                                        | Language | State                                                                          |
| -------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------ |
| [mdschema](https://github.com/jackchuka/mdschema)               | Go       | v0.15.1, MIT, ~77 stars, started 2025-07, actively maintained, one main author |
| [remark-lint](https://github.com/remarkjs/remark-lint)          | JS       | Mature, AST-based, large plugin ecosystem                                       |
| [markdownlint](https://github.com/DavidAnson/markdownlint)      | JS       | Mature, but style rules rather than document schema                             |
| —                                                               | Rust     | Nothing in this space                                                           |

mdschema is the closest match by a distance. Its `.mdschema.yml` declares heading structure and hierarchy, frontmatter fields with types and enums, and link validity; `generate` scaffolds a document from a schema and `derive` infers a schema from existing documents. Those map almost item-for-item onto what arkouda does — enums are the status vocabulary, heading rules are the required sections, link checking is the proposed `E015`, `generate` is `arkouda new`.

Parsing, by contrast, is thoroughly solved in Rust: [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) (130M downloads, used by rustdoc and mdBook), [`markdown`](https://crates.io/crates/markdown) (markdown-rs, a CommonMark parser producing an mdast syntax tree), and `comrak` (GitHub Flavored Markdown).

Worth noting for context: the [OKF toolbox](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/toolbox) upstream ships a metadata-as-code tool and an enrichment agent, and no conformance validator. Arkouda's validator is, as far as can be determined, the implementation of OKF conformance checking. That is a reason to invest in it rather than route around it.

## Decision

Split the problem: **delegate the parser, own the schema.**

### Parse with `pulldown-cmark`

Replace all three line-scanning sites with a single pass over parser events, yielding each heading's level, text, and source range. Fenced and indented code blocks, setext headings, and HTML blocks are then handled by the CommonMark specification rather than by luck.

Only **document-level** headings count. The pass tracks open block containers — block quotes, lists, list items, footnote definitions — and ignores headings nested inside one, because those belong to the container rather than to the document. A parser alone does not give this: `pulldown-cmark` reports `> ## Decision` as a genuine heading, correctly, and it is arkouda's job to decide that a quoted heading is not a section.

`Manifest::section` keeps its signature and its case-insensitive matching. Its termination rule changes from "the next line starting with `## `" to **"the next heading of the same or higher level"**, which is what the current rule was approximating.

GFM tables are enabled. Arkouda's own ADRs use them, and a table row beginning with `|` must never be mistaken for anything else.

`pulldown-cmark` over the alternatives because arkouda needs heading events and byte offsets, not a tree: a pull parser allocates no syntax tree, and the crate is the most widely deployed of the three. See Alternatives for when that reasoning would flip.

### Do not delegate the schema layer

No dependency on mdschema, remark-lint, or a comparable tool. Four reasons, in decreasing order of weight:

- **The rules are not per-document.** A concept id is its bundle-relative path, `index.md` and `log.md` are reserved, ids must be unique across bundles, an index is stale relative to the concepts around it. These are properties of a bundle. A document schema validator validates documents.
- **The diagnostics are the product.** `E000`–`E014`, each with a fix hint, is a contract that CI gates on and agents parse. Delegating means either adopting another tool's vocabulary or translating it, and the translation layer would be most of what was saved.
- **Distribution.** Arkouda is one binary from `install.sh` or `cargo install`, and its CI integration is two lines. A Go or Node dependency makes it two installs.
- **Arkouda is the schema.** For built-in types the schema is a constant, not user input. A tool whose purpose is to let users declare schemas solves a problem arkouda does not have yet.

That last point is the interesting one, because it has an expiry date. If [user-definable concept types](https://github.com/manuelmauro/arkouda/pull/11) ship, arkouda will need a schema file format, and mdschema will have already answered several of those questions — `derive` and `generate` in particular are features that proposal would want. It is prior art to read, not a dependency to take.

## Consequences

### Positive

- Both defects are fixed at the root rather than one construct at a time, and a document may safely contain a template of its own format. That is a prerequisite for [PRD support](https://github.com/manuelmauro/arkouda/pull/11), where documenting a second type means showing its headings.
- Heading levels become available for the first time, so section extraction respects nesting instead of assuming a flat `##` document.
- Source positions come from the parser, so diagnostics can report real line numbers without the `body_start_line + index` arithmetic they do today.
- The quiet wrongness — setext headings, indented code — goes away without anyone having to notice it first.

### Negative

- A dependency is added to a crate that has ten, all of them load-bearing. `pulldown-cmark` is well maintained and widely deployed, but it is still a parser's worth of surface for a tool that currently gets by on `str::strip_prefix`.
- **Documents that pass today may fail after the upgrade.** A concept whose required headings only exist inside a fence is currently valid and becomes an `E009` error. That is the fix behaving correctly, and it can still break a pipeline on upgrade, so it belongs in the release notes rather than in a patch release.
- The rejection of nested section structures in [the PRD ADR](https://github.com/manuelmauro/arkouda/pull/11) loses its mechanical half. That ADR declines Figma's H1 phase grouping partly because `section` cannot handle an intervening H1; after this change it can. The design argument stands on its own — `status` already encodes that lifecycle — but it now stands alone, and should be re-read on that basis rather than treated as settled.

### Neutral

- No CLI surface change, no new diagnostic codes, no schema change. This is entirely an implementation swap.
- Arkouda keeps its position as the OKF conformance implementation, since upstream ships none.
- Frontmatter validation is untouched. It is already a typed `serde` struct, which is the same thing a JSON Schema would buy at more cost.

## Alternatives Considered

### Track fence state in the existing scanner

Toggle a boolean on every line starting with `` ``` `` and skip headings while it is set. A dozen lines, no dependency, and it fixes the reported bug today.

It also fixes exactly one construct. Indented code blocks are next, then setext headings, then HTML blocks, then the heading-level bug that is already known and not addressed by fence tracking at all. Each is individually cheap and collectively a CommonMark parser, written incrementally, by people who are not trying to write one. The bug class survives; only this instance dies.

### Depend on mdschema

Its feature set overlaps arkouda's more than any other tool's, and `derive`/`generate` are genuinely beyond what arkouda has. Rejected because it is a Go binary — arkouda cannot link it, only shell out to it, which turns one install into two and makes every diagnostic a parsed subprocess output. It is also pre-1.0 with one main author, which is a reasonable risk for a linter in CI and an unreasonable one for the core of another tool's validator.

### Depend on the remark ecosystem

The most mature option by a wide margin, AST-based, with `remark-lint-frontmatter-schema` already validating frontmatter against JSON Schema. Same structural objection as mdschema, more so: it is Node, so the dependency is a runtime rather than a binary.

### Use markdown-rs (mdast) instead

Produces a proper syntax tree, which is more pleasant to work with than an event stream and would matter if arkouda needed to traverse or transform document structure. It does not — it needs to know where the headings are. The tree is an allocation and an API surface bought for nothing. This is the alternative to revisit first if validation ever grows rules about content inside a section, such as "`## Requirements` must contain a list".

### Use comrak

The right choice for rendering GitHub Flavored Markdown faithfully, which is not what this is for.
