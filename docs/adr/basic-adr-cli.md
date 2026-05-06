---
id: "basic-adr-cli"
title: "Provide a basic ADR navigation and validation CLI"
abstract: "Introduce a small CLI to navigate ADRs and validate their YAML metadata and Markdown structure."
status: "proposed"
date: "2026-05-06"
deciders: []
tags:
  - cli
  - adr
  - documentation
---

# Provide a basic ADR navigation and validation CLI

## Status

Proposed

## Context

This project will keep Architecture Decision Records (ADRs) in `docs/adr`. As the number of ADRs grows, contributors need a lightweight way to discover, inspect, and validate the collection without relying on manual file inspection or inconsistent conventions.

We want a basic CLI that supports local development workflows and CI checks. The CLI should make ADRs easy to navigate while enforcing a small, explicit ADR contract:

- ADR files are Markdown files in a configured ADR directory, defaulting to `docs/adr`.
- ADR filenames are readable lowercase slugs, not sequence numbers.
- Each ADR starts with YAML frontmatter delimited by `---`.
- Frontmatter contains required metadata such as `id`, `title`, `abstract`, `status`, and `date`.
- Each ADR body follows a minimal Markdown template with standard sections.
- Validation errors are actionable and suitable for CI output.

## Decision

Build a small CLI tool for ADR navigation and validation.

The initial command set will be:

- `arkouda list`: list ADRs sorted by `id`, date, or status, showing title, status, and abstract.
- `arkouda show <id>`: display one ADR by id or filename.
- `arkouda search <query>`: search titles, abstracts, tags, statuses, and Markdown content.
- `arkouda check`: validate all ADR files and return a non-zero exit code on failure.
- `arkouda new <title>`: optionally create a new ADR from the standard template.

The default ADR directory will be `docs/adr`, with a global `--dir` option or `ADR_DIR` environment variable to override it.

The validator will require:

1. YAML frontmatter at the beginning of the file.
2. Required frontmatter keys:
   - `id`
   - `title`
   - `abstract`
   - `status`
   - `date` in `YYYY-MM-DD` format
3. Valid `status` values from a small controlled list, initially:
   - `proposed`
   - `accepted`
   - `superseded`
   - `deprecated`
   - `rejected`
4. A unique slug-style `id` for each ADR.
5. A filename stem matching the ADR `id`.
6. A Markdown heading matching the ADR title or otherwise clearly identifying the ADR.
7. Required body sections:
   - `Status`
   - `Context`
   - `Decision`
   - `Consequences`

The CLI should be conservative at first: validate structure and metadata, but avoid enforcing subjective writing style. Checks can become stricter later if the ADR collection needs stronger consistency.

## Consequences

### Positive

- Contributors can quickly find and inspect ADRs from the terminal.
- CI can prevent malformed ADRs from entering the repository.
- ADR metadata becomes machine-readable for future indexing or publishing.
- A small standard template lowers the cost of writing new ADRs.

### Negative

- The project must maintain a custom CLI and validation rules.
- Strict validation may reject useful ADRs if the template is too narrow.
- Existing or imported ADRs may need migration before validation can run cleanly.

### Neutral

- The first version does not need to render a website or replace full-text tools such as `grep` or IDE search.
- The CLI can start as a local developer tool and later be integrated into CI.

## Alternatives Considered

### Use manual conventions only

We could document the ADR template and rely on reviewers to enforce it. This has no implementation cost, but it is easy for formatting and metadata drift to accumulate.

### Use an existing ADR manager

We could adopt an existing ADR tool. This may provide more features immediately, but it can impose naming, template, or workflow conventions that do not match this project. A small custom CLI keeps the first iteration focused.

### Validate only in CI

We could write a CI-only script instead of an interactive CLI. This would catch invalid ADRs before merge, but would not help contributors navigate or fix ADRs locally.

## Open Questions

- Should `superseded` ADRs require a `superseded_by` frontmatter key?
- Should the tool support multiple ADR directories in monorepos?
