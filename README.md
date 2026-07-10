# arkouda

[![CI](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml/badge.svg)](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/arkouda.svg)](https://crates.io/crates/arkouda)

**AI-native CLI for Architecture Decision Records — built for AI coding agents.**

Arkouda ships a portable [agent skill](skills/use-arkouda/SKILL.md) that teaches AI assistants to check prior decisions before making non-trivial choices and capture new ones afterwards. Output is structured for piping, so agents compose ADRs with their existing shell toolkit (`rg`, `cat`, `awk`). The schema is strict and validation diagnostics carry machine-readable error codes (`E000`–`E014`) — easy for an agent to act on, easy for CI to gate on.

ADRs are stored as an **[Open Knowledge Format][okf] (OKF) v0.1 knowledge bundle**: a directory of Markdown concepts with YAML frontmatter, readable by any OKF-aware tool without special-casing arkouda. Arkouda parses the bundle, validates conformance plus its own ADR contract, scaffolds new entries, generates the `index.md` listing, and pulls a named `## Section` out for you. Anything a one-line shell pipeline does well — content search, counting, slicing, full-file printing — is left to `rg`, `grep`, `awk`, `cat`, and friends. See [`docs/adr/adopt-okf.md`](docs/adr/adopt-okf.md), [`docs/adr/defer-to-unix-tools.md`](docs/adr/defer-to-unix-tools.md), and [`docs/adr/ls-style-list-and-decision.md`](docs/adr/ls-style-list-and-decision.md) for the rationale.

[okf]: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md

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
arkouda list -l                      # long form: id, status, timestamp, path, title — description
arkouda check                        # validate OKF conformance + Markdown structure
arkouda new "Use Postgres"           # scaffold docs/adr/use-postgres.md
arkouda index                        # regenerate docs/adr/index.md
arkouda decision use-postgres        # print the Decision section
arkouda decision use-postgres \
  --section context                  # print another section's body
cat docs/adr/index.md                # every decision at a glance
rg postgres docs/adr/                # content search — use rg/grep, not arkouda

# Pipe-friendly: list emits paths, shell takes it from there
arkouda list | xargs rg postgres
arkouda list -l | awk '$2=="accepted" {print $4}' | xargs cat
```

Run `arkouda --help` and `arkouda <subcommand> --help` for the full surface.

## Commands

| Command    | Description                                                                |
| ---------- | -------------------------------------------------------------------------- |
| `list`     | Print one ADR path per line; `-l` for the `id status timestamp path title — description` table |
| `decision` | Print one ADR's `## Decision` section; `--section <name>` to pick another  |
| `check`    | Validate OKF conformance, frontmatter, concept ids, and Markdown structure |
| `new`      | Scaffold a new ADR from the standard template                              |
| `index`    | Regenerate each bundle's `index.md` directory listing (OKF §6)             |
| `self completions` | Print a shell completion script (`bash`, `zsh`, `fish`, `powershell`, `elvish`) |

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`.

## ADR shape

An ADR is an OKF *concept*. Its **concept id is its path within the bundle**, minus the `.md` suffix — so `docs/adr/use-postgres.md` is `use-postgres`, and a nested `docs/adr/security/mtls.md` is `security/mtls`. There is no `id` frontmatter key.

```markdown
---
type: Architecture Decision Record   # required by OKF; arkouda requires this exact value
title: Use Postgres
description: One-line summary of the decision (what was decided).
tags: []                             # optional
timestamp: 2026-05-06                # ISO 8601 date or datetime
status: proposed                     # proposed | accepted | superseded | deprecated | rejected
deciders: []                         # optional
---

# Use Postgres                       # H1 must equal title

## Status

Proposed

## Context

Why we are deciding this.

## Decision

What we decided.

## Consequences

What follows from the decision.
```

`type`, `title`, `description`, `tags`, and `timestamp` are OKF's own fields; `status`, `deciders`, and `superseded_by` are producer extensions, which OKF §4.1 explicitly permits.

Required keys: `type`, `title`, `description`, `status`, `timestamp`. Required body sections (case-insensitive H2): `Status`, `Context`, `Decision`, `Consequences`. `arkouda check` reports each violation with a code (`E000`–`E014`) and a fix hint.

### Bundle layout

```text
docs/adr/                 # the bundle root
├── index.md              # generated by `arkouda index`; declares okf_version
├── log.md                # optional; reserved by OKF, validated but not generated
├── use-postgres.md       # concept id: use-postgres
└── security/
    └── mtls.md           # concept id: security/mtls
```

`index.md` and `log.md` are reserved by OKF §3.1 and are never treated as ADRs. Discovery recurses into subdirectories.

`arkouda index` writes the bundle-root `index.md`: every concept grouped under its status, each with its one-line description — the whole collection legible in one file, which is what OKF calls progressive disclosure. `arkouda new` refreshes an existing index but never creates one, since OKF makes indexes optional.

Following OKF's permissive-consumption rule (§9), two diagnostics are **warnings** and never fail the run: `E013` (the bundle declares an OKF version arkouda doesn't implement) and `E014` (`index.md` is stale — run `arkouda index`).

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

## Shell completions

`arkouda self completions <shell>` prints a completion script to stdout for `bash`, `zsh`, `fish`, `powershell`, or `elvish`.

```bash
# Bash (add to ~/.bashrc)
eval "$(arkouda self completions bash)"

# Zsh (add to ~/.zshrc)
eval "$(arkouda self completions zsh)"

# Fish (add to ~/.config/fish/config.fish)
arkouda self completions fish | source
```

## Telemetry

Arkouda records one JSON event per invocation to a local file under your OS state directory (`~/Library/Application Support/arkouda/telemetry.jsonl` on macOS, `$XDG_STATE_HOME/arkouda/telemetry.jsonl` or `~/.local/state/arkouda/telemetry.jsonl` elsewhere). The data never leaves your machine — there is no network sink. The goal is to learn how AI coding agents actually invoke arkouda so future surface decisions are informed by usage rather than guesses.

Each event captures the subcommand, redacted argv (paths and free-text titles are replaced with `<path>` / `<title>` markers; flag names and short slugs pass through), exit code, duration, and a short agent identifier derived from a small env-var allowlist (`CLAUDECODE` → `claude-code`, `CURSOR_AGENT` → `cursor`, `AIDER` → `aider`). ADR titles, descriptions, and contents are never recorded. Write failures are silently swallowed; the log rotates at 10 MiB keeping one prior file.

Telemetry is on by default. Opt out per-session with `ARKOUDA_TELEMETRY=0` or per-project in `.arkoudarc.toml`:

```toml
telemetry = false
```

See [`docs/adr/telemetry-for-agent-command-invocations.md`](docs/adr/telemetry-for-agent-command-invocations.md) for the full design.

## Agent skill

[`skills/use-arkouda/SKILL.md`](skills/use-arkouda/SKILL.md) is a portable, [skilo](https://github.com/manuelmauro/skilo)-validated agent skill — drop it into any project that uses arkouda. It teaches an AI coding agent to:

- **Before deciding** — search prior ADRs (`arkouda list | xargs rg -i <topic>`) so the agent doesn't redo a debate that's already in the file, or unknowingly undo a deliberate decision.
- **After deciding** — capture the outcome with `arkouda new "<title>" --description "<one-line decision summary>"` and run `arkouda check` to verify the new ADR validates.
- **Use the subcommands correctly** — including the `## Decision`-by-default contract of `arkouda decision`, the structured pipe-friendly output of `arkouda list`, and the `E000`–`E014` validator diagnostics with their fix hints.

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

The ADR body schema (`## Status`, `## Context`, `## Decision`, `## Consequences`) follows [Michael Nygard's template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard) — the de-facto standard for Architecture Decision Records. The frontmatter and bundle structure follow the [Open Knowledge Format][okf] v0.1, published by Google Cloud Platform. Arkouda layers ADR-specific validation on top of both.

## License

MIT OR Apache-2.0
