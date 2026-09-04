# arkouda

[![CI](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml/badge.svg)](https://github.com/manuelmauro/arkouda/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/arkouda.svg)](https://crates.io/crates/arkouda)

**AI-native CLI for architecture decisions and product requirements — built for AI coding agents.**

Arkouda ships a portable [agent skill](skills/use-arkouda/SKILL.md) that teaches AI assistants to check what a project already decided — and what it is already trying to build — before making non-trivial choices, and to capture the outcome afterwards. Output is structured for piping, so agents compose these documents with their existing shell toolkit (`rg`, `cat`, `awk`). The schema is strict and validation diagnostics carry machine-readable error codes (`E000`–`E015`) — easy for an agent to act on, easy for CI to gate on.

Arkouda has **two built-in concept types**: the **Architecture Decision Record** (ADR), for why the software is built the way it is, and the **Product Requirements Document** (PRD), for what it is supposed to do. A project [declares its own](#declaring-your-own-concept-types) with `[[types]]` in `.arkoudarc.toml`. Each has its own status vocabulary, required sections, template, and default directory; a PRD's `decisions` frontmatter key points at the ADRs that shaped it, so a requirement and its rationale are one lookup apart. See [`docs/adr/support-product-requirements-documents.md`](docs/adr/support-product-requirements-documents.md).

Documents are stored as an **[Open Knowledge Format][okf] (OKF) v0.2 knowledge bundle**: a directory of Markdown concepts with YAML frontmatter, readable by any OKF-aware tool without special-casing arkouda. A concept's `type` is a frontmatter field, so one bundle may hold both kinds. Arkouda parses the bundle, validates conformance plus its own contract for whichever type each concept declares, scaffolds new entries, generates the `index.md` listing, and pulls a named `## Section` out for you. Anything a one-line shell pipeline does well — content search, counting, slicing, full-file printing — is left to `rg`, `grep`, `awk`, `cat`, and friends. See [`docs/adr/adopt-okf.md`](docs/adr/adopt-okf.md), [`docs/adr/defer-to-unix-tools.md`](docs/adr/defer-to-unix-tools.md), and [`docs/adr/ls-style-list-and-decision.md`](docs/adr/ls-style-list-and-decision.md) for the rationale.

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
arkouda list                         # one path per line — pipe straight to xargs/rg/cat
arkouda list -l                      # long form: id, type, status, timestamp, path, title — description
arkouda list --type prd              # just the requirements documents
arkouda check                        # validate OKF conformance + Markdown structure
arkouda new "Use Postgres"           # scaffold docs/adr/use-postgres.md (an ADR)
arkouda new "Bulk Import" --type prd # scaffold docs/prd/bulk-import.md (a PRD)
arkouda index                        # regenerate each bundle's index.md
arkouda section use-postgres         # print the primary section — Decision for an ADR
arkouda section bulk-import          # ...and Requirements for a PRD
arkouda section use-postgres context # print another section's body
cat docs/adr/index.md                # every concept at a glance
rg postgres docs/adr/                # content search — use rg/grep, not arkouda

# Pipe-friendly: list emits paths, shell takes it from there
arkouda list | xargs rg postgres
arkouda list -l | awk '$3=="accepted" {print $5}' | xargs cat
```

Run `arkouda --help` and `arkouda <subcommand> --help` for the full surface.

## Commands

| Command    | Description                                                                |
| ---------- | -------------------------------------------------------------------------- |
| `list`     | Print one path per line; `-l` for the `id type status timestamp path title — description` table; `--type` to filter |
| `section`  | Print one concept's primary section (`Decision` for an ADR, `Requirements` for a PRD); name a section to pick another |
| `check`    | Validate OKF conformance, frontmatter, concept ids, and Markdown structure |
| `new`      | Scaffold a new concept from its type's template (`--type adr\|prd`)         |
| `index`    | Regenerate each bundle's `index.md` directory listing (OKF §6)             |
| `self completions` | Print a shell completion script (`bash`, `zsh`, `fish`, `powershell`, `elvish`) |

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`.

## Concept shape

Every document is an OKF *concept*. Its **concept id is its path within the bundle**, minus the `.md` suffix — so `docs/adr/use-postgres.md` is `use-postgres`, and a nested `docs/adr/security/mtls.md` is `security/mtls`. There is no `id` frontmatter key.

`type` decides which contract a concept is checked against. The required frontmatter keys are the same for both built-in types — `type`, `title`, `description`, `status`, and a content timestamp — and that set is *arkouda's* profile, not OKF's: OKF requires only `type`, and everything else arkouda insists on is layered on top.

A concept's last meaningful change is `generated: { by, at }` (OKF §5.2). OKF v0.2 supersedes v0.1's `timestamp` with it, and arkouda reads `generated.at` first, falling back to a legacy `timestamp` — so **v0.1 documents keep working**, with an `E017` warning as the only prompt to migrate. Every instant v0.2 defines must carry an explicit offset, e.g. `2026-05-06T14:30:00Z` — `generated.at`, each `verified[].at`, `stale_after`, each `sources[].last_modified`, and both ends of a `usage_window`. The retired `timestamp` still accepts a plain date.

The v0.2 provenance, trust, and lifecycle families — `sources` with its credibility signals, `usage_window`, `generated`, `verified`, `stale_after` — are all parsed. None is required, and §11 forbids rejecting a concept for missing any of them.

| | ADR | PRD |
| --- | --- | --- |
| `--type` | `adr` | `prd` |
| OKF `type` | `Architecture Decision Record` | `Product Requirements Document` |
| statuses | `proposed`, `accepted`, `superseded`, `deprecated`, `rejected` | `draft`, `in-review`, `approved`, `shipped`, `abandoned`, `superseded` |
| required sections | `Status`, `Context`, `Decision`, `Consequences` | `Status`, `Problem`, `Requirements`, `Non-Goals`, `Success Metrics` |
| primary section | `Decision` | `Requirements` |
| default directory | `docs/adr` | `docs/prd` |
| extensions | `deciders`, `superseded_by` | `owner`, `target_release`, `decisions`, `superseded_by` |

Apart from `Status`, the two share no section headings — a PRD's problem statement is not a decision record's context, and pretending otherwise would make the ADR vocabulary look universal.

OKF v0.2 §5.4 also defines a `status`, with the coarse vocabulary `draft | stable | deprecated`. Arkouda keeps its own per-type vocabularies, which refine rather than contradict it — see [`docs/adr/adopt-okf-0-2.md`](docs/adr/adopt-okf-0-2.md). A project that wants OKF's exact three values can declare a type whose `statuses` are `["draft", "stable", "deprecated"]`.

### An ADR

```markdown
---
type: Architecture Decision Record   # required by OKF; selects this contract
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

### A PRD

```markdown
---
type: Product Requirements Document
title: Bulk ADR Import
description: One-line summary of what is being built and for whom.
tags: []
timestamp: 2026-08-07
status: draft                        # draft | in-review | approved | shipped | abandoned | superseded
resource: https://github.com/org/repo/issues/42   # optional: the tracker item
owner: []                            # optional; mirrors an ADR's deciders
target_release: 2026-09-01           # optional
decisions:                           # optional; concept ids of the ADRs that shaped this
  - adopt-okf
---

# Bulk ADR Import                    # H1 must equal title

## Status

Draft

## Problem

Who has it, why it matters, and why now.

## Approach                          # scaffolded, not required

The shape of the solution, in a paragraph.

## Requirements

What the software must do. The primary section.

## Non-Goals

What this explicitly does not cover.

## Success Metrics

How we will know it worked.

## Open Questions                    # scaffolded, not required
```

`decisions` is frontmatter rather than a `## Decisions` section on purpose: a concept id is bundle-relative, so it survives pointing `prd` at a different directory, and a list of ids can be *checked*, where Markdown prose cannot. `arkouda check` resolves both `decisions` and `superseded_by` against the loaded collection and reports a dangling reference as `E015`.

`arkouda check` reports each violation with a code (`E000`–`E015`) and a fix hint.

### Bundle layout

```text
docs/adr/                 # a bundle root
├── index.md              # generated by `arkouda index`; declares okf_version
├── log.md                # optional; reserved by OKF, validated but not generated
├── use-postgres.md       # concept id: use-postgres
└── security/
    └── mtls.md           # concept id: security/mtls
docs/prd/                 # another bundle root
└── bulk-adr-import.md    # concept id: bulk-adr-import
```

`index.md` and `log.md` are reserved by OKF §3.1 and are never treated as concepts. Discovery recurses into subdirectories.

The per-type directories are a default write target and a search scope, not a schema: type comes from frontmatter, so a single bundle may hold ADRs and PRDs side by side if you prefer.

`arkouda index` writes each bundle-root `index.md`: every concept under `# <Type>` and then `## <Status>`, each with its one-line description — the whole collection legible in one file, which is what OKF calls progressive disclosure. Nesting is uniform, so a single-type bundle still carries the type heading and consumers parse one shape. `arkouda new` refreshes an existing index but never creates one, since OKF makes indexes optional.

### What `check` enforces

`arkouda check` validates in three tiers, and which of them a concept is judged by depends on whether its `type` resolves to a configured type:

| Tier | Codes | Applies to |
| --- | --- | --- |
| OKF conformance | `E000`, `E004`, `E010`, `E011`, `E012` | every concept, always |
| arkouda profile | `E001`, `E002`, `E006`, `E007`, `E008` | concepts whose `type` resolves to a configured type |
| template contract | `E003` (status vocabulary), `E009` (required sections) | that type's vocabulary, and its sections when it names any |

A concept declaring a type no `[[types]]` table configures is checked for OKF conformance, reported as an `E005` **warning**, and left to pass. It is not skipped — its concept id, its heading, and its place in the bundle are still arkouda's business, and it still appears in `list` and `index`. Since types are user-definable, an unrecognized `type` is ordinarily one you have not declared rather than a mistake, so arkouda tells you about it instead of failing your build over a bundle that is perfectly conformant to the format it implements.

`E007` and `E008` sit in arkouda's profile rather than the OKF tier because OKF §4.2 says plainly that there are no required body sections: a concept with no `#` heading is conformant, and failing one would mean rejecting a bundle the spec accepts.

Following OKF's permissive-consumption rule (§11), six diagnostics are **warnings** and never fail the run: `E005` (no configured type declares this concept's `type`), `E013` (the bundle declares an OKF version arkouda doesn't implement), `E014` (`index.md` is stale — run `arkouda index`), `E015` (a `decisions` or `superseded_by` reference does not resolve — which may simply mean it points into a bundle this invocation did not load), `E016` (the concept is past its `stale_after` instant), and `E017` (the concept dates itself with v0.1's `timestamp` rather than `generated.at`).

## Configuration

`--dir <path>` (and the `ADR_DIR` env var) point arkouda at a single directory and override everything else, for every type. With neither set, arkouda walks up from the working directory looking for `.arkoudarc.toml`; if found, its `dirs` entry is used. With nothing configured, each type falls back to its own default: `docs/adr` and `docs/prd`.

`dirs` takes two forms. The **flat** form is one list every type shares — useful for monorepos that keep documents per service or area:

```toml
dirs = [
  "docs/adr",
  "services/billing/docs/adr",
  "services/identity/docs/adr",
]
```

The **typed** form gives each type its own roots:

```toml
[dirs]
adr = ["docs/adr"]
prd = ["docs/prd"]
```

Both parse into the same model: a type-to-roots map, plus the union of every root. Relative paths resolve against the location of the config file, so the same file works from any subdirectory. `arkouda list`, `check`, `section`, and `index` work over the union — a concept's type is a fact about its frontmatter, not about where it sits, so nothing is skipped on the strength of a directory. `arkouda new` writes into the first root configured for the type it is creating (use `--dir` to target another). A typed table is a complete declaration: a type it does not mention has no root, and `arkouda new` for that type says so rather than inventing a directory.

| Setting   | Default                    | Override (low → high precedence)                         |
| --------- | -------------------------- | -------------------------------------------------------- |
| Bundle dirs | `docs/adr`, `docs/prd`   | `.arkoudarc.toml` `dirs` → `ADR_DIR=<path>` → `--dir <path>` |

### Declaring your own concept types

Decisions and requirements are two document types out of many a repo might keep. `[[types]]` tables declare the rest, and a declared type gets everything the built-ins get: `--type`, a template, a status lifecycle, `index.md` grouping, and a `check` contract.

```toml
[[types]]
slug = "rfc"                                        # what `--type` takes
okf_type = "Request for Comments"                   # what its documents declare
statuses = ["draft", "active", "withdrawn"]         # lifecycle order; the first is `new`'s default
required_sections = ["Status", "Summary", "Motivation"]   # optional
primary_section = "Summary"                         # optional; what `section <id>` prints
default_dir = "docs/rfc"
template = "docs/templates/rfc.md"                  # optional
extensions = ["sponsors"]                           # optional frontmatter keys to scaffold
```

`slug`, `okf_type`, `statuses`, and `default_dir` are required. Status labels are derived from their names, so `in-review` displays as `In Review`.

**`required_sections` is optional, and that is the point.** A type that names none gets a lifecycle, a template, and full `list`/`index`/`section` support with no body contract — nothing to fail `check` over. Use it for concepts whose shape is not worth enforcing. `primary_section` is optional too; without it, `arkouda section <id>` needs an explicit section name.

**`template`** points at a Markdown file whose `##` headings and prose become what `arkouda new` scaffolds. It may exceed `required_sections` — that is how you prompt for a section without failing a bundle over it — but it may not fall short of them, or `new` would write documents `check` rejects on creation. `## Status` is always rendered from the status itself, so a template neither needs one nor contributes one. Without a `template`, the required sections are scaffolded with a `TODO:` line each. Paths resolve against the config file.

A declared type whose `slug` **or** `okf_type` matches a built-in **replaces it wholesale** — use this to change ADR's sections or statuses for your project. There is no partial override: a type is a contract, and one assembled from a built-in plus three patches is harder to state than one written out.

Mistakes are caught when the config loads, not when a document fails: a slug `--type` could not take, an empty lifecycle, a `primary_section` outside `required_sections`, a template short of a required section, two tables claiming one slug, or a misspelled key.

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

Arkouda records one JSON event per invocation to a local file under your OS state directory (`~/Library/Application Support/arkouda/telemetry.jsonl` on macOS, `$XDG_STATE_HOME/arkouda/telemetry.jsonl` or `~/.local/state/arkouda/telemetry.jsonl` elsewhere). The data never leaves your machine — there is no network sink. The goal is to learn how AI coding agents actually invoke arkouda so future surface decisions are informed by usage rather than guesses. Events carry the subcommand, redacted argv, exit code, duration, the diagnostic codes a `check` produced, and whether a resolved `--type` was built in or declared by the project — never a type's slug, a title, or a path.

Each event captures the subcommand, redacted argv (paths and free-text titles are replaced with `<path>` / `<title>` markers; flag names and short slugs pass through), exit code, duration, and a short agent identifier derived from a small env-var allowlist (`CLAUDECODE` → `claude-code`, `CURSOR_AGENT` → `cursor`, `AIDER` → `aider`). Concept titles, descriptions, and contents are never recorded. Write failures are silently swallowed; the log rotates at 10 MiB keeping one prior file.

Telemetry is on by default. Opt out per-session with `ARKOUDA_TELEMETRY=0` or per-project in `.arkoudarc.toml`:

```toml
telemetry = false
```

See [`docs/adr/telemetry-for-agent-command-invocations.md`](docs/adr/telemetry-for-agent-command-invocations.md) for the full design.

## Agent skill

[`skills/use-arkouda/SKILL.md`](skills/use-arkouda/SKILL.md) is a portable, [skilo](https://github.com/manuelmauro/skilo)-validated agent skill — drop it into any project that uses arkouda. It teaches an AI coding agent to:

- **Before deciding or building** — search what is already recorded (`arkouda list | xargs rg -i <topic>`) so the agent doesn't redo a debate that's already in the file, unknowingly undo a deliberate decision, or build something the requirements already rule out.
- **After deciding** — capture the outcome with `arkouda new "<title>" --description "<one-line decision summary>"` and run `arkouda check` to verify it validates.
- **Choose the right type** — an ADR for why the software is built this way, a PRD for what it is supposed to do, linked by the PRD's `decisions` key.
- **Use the subcommands correctly** — including the primary-section-by-default contract of `arkouda section`, the structured pipe-friendly output of `arkouda list`, and the `E000`–`E015` validator diagnostics with their fix hints.

The skill is repo-agnostic: it discovers paths via `arkouda list` rather than hardcoding `docs/adr/`, so it works across monorepos that use `.arkoudarc.toml` to point at multiple directories.

## CI integration

```yaml
- name: Validate decision and requirements documents
  run: |
    curl -sSfL https://raw.githubusercontent.com/manuelmauro/arkouda/main/install.sh | sh
    arkouda check
```

`arkouda check` exits 0 on a clean collection, 1 on any error. Warnings never fail the run, so a bundle holding concept types you have not declared stays green while still reporting them.

## Acknowledgements

The ADR body schema (`## Status`, `## Context`, `## Decision`, `## Consequences`) follows [Michael Nygard's template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard) — the de-facto standard for Architecture Decision Records. The PRD section set is drawn from the templates surveyed in [`docs/adr/support-product-requirements-documents.md`](docs/adr/support-product-requirements-documents.md): [GitHub's spec-kit](https://github.com/github/spec-kit/blob/main/templates/spec-template.md), the [Atlassian Product Requirements blueprint](https://confluence.atlassian.com/doc/product-requirements-blueprint-329975392.html), [Figma's approach to PRDs](https://coda.io/@yuhki/figmas-approach-to-product-requirement-docs), [Shape Up](https://basecamp.com/shapeup/1.5-chapter-06), and [Lenny Rachitsky's template](https://www.atlassian.com/software/confluence/templates/lennys-product-requirements). The frontmatter and bundle structure follow the [Open Knowledge Format][okf] v0.1, published by Google Cloud Platform under the Apache 2.0 licence. Arkouda layers its own per-type validation on top of both.

A verbatim copy of the OKF specification arkouda implements is vendored at [`docs/okf/SPEC.md`](docs/okf/SPEC.md), pinned to the upstream commit it was taken from — see [`docs/okf/README.md`](docs/okf/README.md) for provenance and checksums.

## License

MIT OR Apache-2.0
