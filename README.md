# arkouda

[![CI](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml/badge.svg)](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/arkouda.svg)](https://crates.io/crates/arkouda)

**AI-native CLI for Architecture Decision Records — built for AI coding agents.**

Arkouda ships a portable [agent skill](skills/use-arkouda/SKILL.md) that teaches AI assistants to check prior decisions before making non-trivial choices and capture new ones afterwards. Output is structured for piping, so agents compose ADRs with their existing shell toolkit (`rg`, `cat`, `awk`). The schema is strict and validation diagnostics carry machine-readable error codes (`E000`–`E010`) — easy for an agent to act on, easy for CI to gate on.

Under the hood arkouda treats ADRs as plain Markdown files with YAML frontmatter: it parses the collection, validates the schema, scaffolds new entries, and pulls a named `## Section` out for you. Anything a one-line shell pipeline does well — content search, counting, slicing, full-file printing — is left to `rg`, `grep`, `awk`, `cat`, and friends. See [`docs/adr/defer-to-unix-tools.md`](docs/adr/defer-to-unix-tools.md) and [`docs/adr/ls-style-list-and-decision.md`](docs/adr/ls-style-list-and-decision.md) for the rationale.

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
arkouda list                         # one ADR path per line — pipe straight to xargs/rg/cat
arkouda list -l                      # long form: id, status, date, path, title (no header)
arkouda check                        # validate frontmatter + Markdown structure
arkouda new "Use Postgres"           # scaffold docs/adr/use-postgres.md
arkouda decision use-postgres        # print the Decision section
arkouda decision use-postgres \
  --section context                  # print another section's body
cat docs/adr/use-postgres.md         # full file — that's just cat
rg postgres docs/adr/                # content search — use rg/grep, not arkouda

# Pipe-friendly: list emits paths, shell takes it from there
arkouda list | xargs rg postgres
arkouda list -l | awk '$2=="accepted" {print $4}' | xargs cat
```

Run `arkouda --help` and `arkouda <subcommand> --help` for the full surface.

## Commands

| Command    | Description                                                                |
| ---------- | -------------------------------------------------------------------------- |
| `list`     | Print one ADR path per line; `-l` for the `id status date path title` table |
| `decision` | Print one ADR's `## Decision` section; `--section <name>` to pick another  |
| `check`    | Validate every ADR's frontmatter, filename, and required Markdown sections |
| `new`      | Scaffold a new ADR from the standard template                              |

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`.

## ADR shape

`arkouda check` enforces the contract.

```markdown
---
id: "use-postgres"            # lowercase slug, must match filename stem
title: "Use Postgres"
abstract: "One-line summary of the decision (what was decided)."
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

`--dir <path>` (and the `ADR_DIR` env var) point arkouda at a single directory and override everything else. With neither set, arkouda walks up from the working directory looking for `.arkoudarc.toml`; if found, its `dirs` list is used. With nothing configured, the default is `docs/adr`.

`.arkoudarc.toml` lists one or more directories — useful for monorepos that keep ADRs per service or area:

```toml
dirs = [
  "docs/adr",
  "services/billing/docs/adr",
  "services/identity/docs/adr",
]
```

Relative paths resolve against the location of the config file, so the same file works from any subdirectory. `arkouda list`, `check`, and `decision` aggregate across every listed directory; `arkouda new` writes into the first one (use `--dir` to target another).

| Setting   | Default     | Override (low → high precedence)                         |
| --------- | ----------- | -------------------------------------------------------- |
| ADR dirs  | `docs/adr`  | `.arkoudarc.toml` `dirs` → `ADR_DIR=<path>` → `--dir <path>` |

## Telemetry

Arkouda records one JSON event per invocation to a local file under your OS state directory (`~/Library/Application Support/arkouda/telemetry.jsonl` on macOS, `$XDG_STATE_HOME/arkouda/telemetry.jsonl` or `~/.local/state/arkouda/telemetry.jsonl` elsewhere). The data never leaves your machine — there is no network sink. The goal is to learn how AI coding agents actually invoke arkouda so future surface decisions are informed by usage rather than guesses.

Each event captures the subcommand, redacted argv (paths and free-text titles are replaced with `<path>` / `<title>` markers; flag names and short slugs pass through), exit code, duration, and a short agent identifier derived from a small env-var allowlist (`CLAUDECODE` → `claude-code`, `CURSOR_AGENT` → `cursor`, `AIDER` → `aider`). ADR titles, abstracts, and contents are never recorded. Write failures are silently swallowed; the log rotates at 10 MiB keeping one prior file.

Telemetry is on by default. Opt out per-session with `ARKOUDA_TELEMETRY=0` or per-project in `.arkoudarc.toml`:

```toml
telemetry = false
```

See [`docs/adr/telemetry-for-agent-command-invocations.md`](docs/adr/telemetry-for-agent-command-invocations.md) for the full design.

## Agent skill

[`skills/use-arkouda/SKILL.md`](skills/use-arkouda/SKILL.md) is a portable, [skilo](https://github.com/manuelmauro/skilo)-validated agent skill — drop it into any project that uses arkouda. It teaches an AI coding agent to:

- **Before deciding** — search prior ADRs (`arkouda list | xargs rg -i <topic>`) so the agent doesn't redo a debate that's already in the file, or unknowingly undo a deliberate decision.
- **After deciding** — capture the outcome with `arkouda new "<title>" --abstract "<one-line decision summary>"` and run `arkouda check` to verify the new ADR validates.
- **Use the four subcommands correctly** — including the `## Decision`-by-default contract of `arkouda decision`, the structured pipe-friendly output of `arkouda list`, and the `E000`–`E010` validator diagnostics with their fix hints.

The skill is repo-agnostic: it discovers ADR paths via `arkouda list` rather than hardcoding `docs/adr/`, so it works across monorepos that use `.arkoudarc.toml` to point at multiple ADR directories.

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
