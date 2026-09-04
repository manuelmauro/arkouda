---
okf_version: "0.2"
---

# Architecture Decision Record

## Proposed

* [Parse Markdown instead of scanning lines](parse-markdown-instead-of-scanning-lines.md) - Replace hand-rolled line scanning with a CommonMark parser (pulldown-cmark), and keep the document-schema layer in-house rather than delegating to an external Markdown schema validator.
* [Support Product Requirements Documents](support-product-requirements-documents.md) - Generalize arkouda from an ADR-only tool into a concept-type-aware OKF tool, adding Product Requirements Documents as a second built-in concept type.

## Accepted

* [Adopt the Open Knowledge Format](adopt-okf.md) - Store ADRs as an OKF v0.1 knowledge bundle, replacing arkouda's bespoke frontmatter schema with OKF's type/title/description/timestamp fields.
* [Adopt OKF 0.2](adopt-okf-0-2.md) - Move arkouda from OKF v0.1 to v0.2: generated.at supersedes timestamp, the provenance families are parsed, status adopts OKF's vocabulary, and arkouda's per-type values move to a new lifecycle key.
* [Provide a basic ADR navigation and validation CLI](basic-adr-cli.md) - Introduce a small CLI to navigate ADRs and validate their YAML metadata and Markdown structure.
* [Defer to Unix tools](defer-to-unix-tools.md) - Arkouda exposes structured access to the ADR collection but defers content search and other shell-friendly operations to standard Unix tools.
* [ls-style list and a decision subcommand](ls-style-list-and-decision.md) - Make arkouda list headerless and ls-style (paths by default, -l for the table) and replace show with arkouda decision <id>, defaulting to the decision section.
* [Support user-defined concept types](support-user-defined-concept-types.md) - Let a project declare its own concept types in .arkoudarc.toml, and tier check's rules so template conformance applies only where a template defines it.
* [Telemetry for agent command invocations](telemetry-for-agent-command-invocations.md) - Record arkouda command invocations to a local JSONL file, on by default, attributing each event to the agent (or human) that ran it.
