# arkouda

[![CI](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml/badge.svg)](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/arkouda.svg)](https://crates.io/crates/arkouda)

A small CLI for navigating and validating Architecture Decision Records (ADRs).

Arkouda treats ADRs as plain Markdown files with YAML frontmatter. It parses the collection, validates the schema, scaffolds new entries, resolves an id to a file, and pulls a named `## Section` out for you. Anything a one-line shell pipeline does well — content search, counting, slicing — is left to `rg`, `grep`, `awk`, and friends. See [`docs/adr/defer-to-unix-tools.md`](docs/adr/defer-to-unix-tools.md) for the rationale.

## Installation

```bash
# Quick install (downloads a release binary, falls back to cargo)
curl -sSfL https://raw.githubusercontent.com/manuelmauro/arkouda/main/install.sh | sh

# Or from crates.io
cargo install arkouda

# Or from a clone
make install
```

## Quick Start

```bash
arkouda list                         # table: id, status, date, path, title
arkouda check                        # validate frontmatter + Markdown structure
arkouda new "Use Postgres"           # scaffold docs/adr/use-postgres.md
arkouda show use-postgres            # print the full ADR
arkouda show use-postgres \
  --section decision                 # print just one section's body
rg postgres docs/adr/                # content search — use rg/grep, not arkouda

# Pipe-friendly: list emits paths, shell takes it from there
arkouda list | awk 'NR>1 && $2=="proposed" {print $4}' | xargs cat
```

Run `arkouda --help` and `arkouda <subcommand> --help` for the full surface.

## Commands

| Command   | Description                                                                |
| --------- | -------------------------------------------------------------------------- |
| `list`    | Print a table of ADRs (id, status, date, path, title)                      |
| `show`    | Print one ADR by id, or only its `--section <name>` body                   |
| `check`   | Validate every ADR's frontmatter, filename, and required Markdown sections |
| `new`     | Scaffold a new ADR from the standard template                              |

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`.

## ADR shape

`arkouda check` enforces the contract.

```markdown
---
id: "use-postgres"            # lowercase slug, must match filename stem
title: "Use Postgres"
abstract: "One-line summary."
status: "proposed"            # proposed | accepted | superseded | deprecated | rejected
date: "2026-05-06"            # ISO YYYY-MM-DD, must be a real date
deciders: []                  # optional
tags: []                      # optional
---

# Use Postgres                # H1 must equal title

## Status

Proposed

## Context

Why we are deciding this.

## Decision

What we decided.

## Consequences

What follows from the decision.
```

Required keys: `id`, `title`, `abstract`, `status`, `date`. Required body sections (case-insensitive H2): `Status`, `Context`, `Decision`, `Consequences`. Filename stem must equal the frontmatter `id`. `arkouda check` reports each violation with a code (`E000`–`E010`) and a fix hint.

## Configuration

| Setting   | Default     | Override                          |
| --------- | ----------- | --------------------------------- |
| ADR dir   | `docs/adr`  | `--dir <path>` or `ADR_DIR=<path>` |

## Agent skill

`skills/arkouda/SKILL.md` is a [skilo](https://github.com/manuelmauro/skilo)-validated agent skill that teaches an AI assistant when to reach for arkouda, the four subcommands, the schema, common workflows, and how to read the diagnostic codes.

## CI integration

```yaml
- name: Validate ADRs
  run: |
    curl -sSfL https://raw.githubusercontent.com/manuelmauro/arkouda/main/install.sh | sh
    arkouda check
```

`arkouda check` exits 0 on a clean collection, 1 on any error.

## Acknowledgements

The ADR body schema (`## Status`, `## Context`, `## Decision`, `## Consequences`) follows [Michael Nygard's template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard) — the de-facto standard for Architecture Decision Records. Arkouda layers structured frontmatter, a slug-based id scheme, and validation on top of that template.

## License

MIT OR Apache-2.0
