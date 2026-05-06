# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-05-06

Initial release.

### Added

- `arkouda list` — table of every ADR in the directory, with `--sort id|date|status` and a `--section <name>` flag that prints a Markdown digest of that section across all ADRs.
- `arkouda show <id>` — print one ADR by frontmatter id, filename stem, or filename. With `--section <name>`, print only that section's body.
- `arkouda check` — validate frontmatter, filename, and required Markdown structure across the collection. Reports diagnostics with codes `E000`–`E010` and fix hints; exits 1 on any error.
- `arkouda new "<title>"` — scaffold a new ADR from the standard template, with optional `--id`, `--status`, and `--abstract` flags.
- Frontmatter schema: required `id`, `title`, `abstract`, `status`, `date`; optional `deciders`, `tags`, `superseded_by`. Status is one of `proposed | accepted | superseded | deprecated | rejected`.
- Body schema: `# <title>` H1 plus required `## Status`, `## Context`, `## Decision`, `## Consequences` sections.
- Configurable ADR directory via `--dir <path>` or `ADR_DIR=<path>` (default `docs/adr`).
- GitHub Actions CI (fmt, clippy `-D warnings`, tests, build) and tagged-release workflow that ships Linux x86_64, macOS aarch64, and Windows x86_64 binaries with sha256 checksums.
- `install.sh` quick installer that prefers a pre-built release binary and falls back to `cargo install arkouda`.
- Dual MIT/Apache-2.0 license.
- Agent skills: `skills/arkouda` (how to use the CLI) and `skills/prepare-release` (how to cut a release).

[0.1.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.1.0
