---
name: use-arkouda
description: Find prior decisions and record new ones in a repo's ADR (Architecture Decision Record) collection using the arkouda CLI. Invoke any time you're about to make a non-trivial design, architecture, library, schema, or convention decision — check what was already decided before deciding, and capture the outcome afterwards.
license: MIT
---

# Using arkouda

In repositories that record decisions as ADRs (Architecture Decision Records — Markdown files with YAML frontmatter, conventionally under `docs/adr/`), `arkouda` is the CLI for finding, reading, validating, and scaffolding them. **Before you decide, check what's already been decided. After you decide, capture it.**

An ADR directory is an [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) (OKF) v0.1 *knowledge bundle*: each ADR is a *concept* whose id is its path within the bundle without the `.md` suffix (`security/mtls.md` → `security/mtls`). `index.md` and `log.md` are reserved by OKF and are never ADRs.

If a repo has no ADR directory yet but the `arkouda` binary is installed, this skill is also the right one to reach for: `arkouda new` enforces the schema from the first file.

## When to use

Reach for this skill any time you're about to make a non-trivial decision. Concretely:

- Before writing code that picks a library, framework, datastore, encoding, transport, or other "we now depend on X" commitment.
- Before changing a public interface, file layout, schema, naming convention, or directory structure.
- Before refactoring away from a pattern you didn't introduce — you may be about to undo a deliberate decision.
- When the user asks "did we ever decide on X?", "why is it done this way?", or otherwise touches motivation.
- When the user asks for a new ADR or to mark one superseded.
- Whenever you land in an unfamiliar repo with an ADR directory (commonly `docs/adr/`, but not always — see below).

A 5-second `arkouda list | xargs rg -i <topic>` is cheaper than redoing a debate that's already in the file. If a relevant ADR exists, build on it, propose superseding it, or notice you don't need to decide at all.

## Philosophy

Two principles shape arkouda's behaviour, and explain why some defaults look minimal:

- **Defer to Unix tools.** Arkouda earns subcommands only where standard shell tools (`rg`, `grep`, `cat`, `awk`, `xargs`) cannot. Content search, full-file printing, counting, and slicing are left to the shell — the CLI emits structured output you compose with the rest of your toolbox. Hence: no `search`, no full-file `show`.
- **Decision-centric defaults.** `arkouda list` prints one ADR path per line (no header, no padding) so it pipes cleanly. The body of an ADR, for arkouda's purposes, is its `## Decision` section, so `arkouda decision <id>` defaults to that section's body — supporting sections (`context`, `consequences`, `status`, custom) are opt-in via `--section`.
- **Standard format over bespoke.** ADRs are stored as OKF concepts so any OKF-aware consumer can read them without special-casing arkouda.

The source rationale lives in arkouda's own repo, in the ADRs [`defer-to-unix-tools`](https://github.com/manuelmauro/arkouda/blob/main/docs/adr/defer-to-unix-tools.md), [`ls-style-list-and-decision`](https://github.com/manuelmauro/arkouda/blob/main/docs/adr/ls-style-list-and-decision.md), and [`adopt-okf`](https://github.com/manuelmauro/arkouda/blob/main/docs/adr/adopt-okf.md).

## Where ADRs live

The location varies between repos. Don't hardcode `docs/adr/` in pipelines — ask arkouda. Run **`arkouda list`** to get the actual ADR paths for the repo you're in.

Resolution order, in case you need to set or override the location:

1. `--dir <path>` flag (one-shot override, single directory).
2. `ADR_DIR=<path>` environment variable (session override, single directory).
3. `.arkoudarc.toml` at the repo root (or any ancestor of the cwd) with a `dirs = [...]` list — supports multiple directories, useful for monorepos:
   ```toml
   dirs = ["docs/adr", "services/billing/docs/adr"]
   ```
   Relative paths resolve against the config file's directory. `arkouda list`, `check`, and `decision` aggregate across all listed dirs; `arkouda new` writes into the first one.
4. Default: `docs/adr/`.

A concept id is the ADR's path *within its bundle*, minus the `.md` suffix — not just the filename. A top-level `use-postgres.md` has the id `use-postgres`; a nested `security/mtls.md` has the id `security/mtls`. The bundle directory itself varies between repos, so let `arkouda list` tell you where it is. `arkouda decision` accepts the full concept id (`security/mtls`), the bare stem (`mtls`), or the filename.

## Commands

Five subcommands, each doing something the shell can't:

- **`arkouda list [--sort id|timestamp|status] [-l]`** — one ADR path per line. Pipe straight into `xargs`/`rg`/`cat`/`wc`. With `-l`, headerless `ID STATUS TIMESTAMP PATH TITLE — DESCRIPTION` table for human skimming.
- **`arkouda decision <id> [--section <name>]`** — body of that ADR's `## Decision` section. `--section <name>` picks any other heading (`context`, `consequences`, `status`, or custom). Errors if the section is missing. For the full file, resolve the path through `arkouda list` and `cat` it.
- **`arkouda check`** — validates OKF conformance, frontmatter, concept ids, and required Markdown sections across the collection. Exit 0 clean, 1 on any error. Each diagnostic carries a code (E000–E014) and a fix hint. Warnings never fail the run.
- **`arkouda new "<title>" [--id <slug>] [--status proposed|accepted|superseded|deprecated|rejected] [--description "<one-line summary of the decision>"]`** — scaffold a new ADR with today's date. Default id is a slug from the title. The description should summarize *what was decided*, not just the topic. Refreshes `index.md` if the bundle has one.
- **`arkouda index`** — regenerate each bundle's `index.md`, an OKF §6 listing of every concept grouped by status. Read it to see the whole collection at a glance without opening any file.

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`. Run `arkouda --help` or `arkouda <subcommand> --help` for the authoritative surface.

There is intentionally no `search` subcommand and no full-file `show` — `rg`/`grep` and `cat` already do those.

## One-liners

`arkouda list` is the path source — it's where the ADRs *actually* are in this repo.

```sh
# Orient in an unfamiliar repo — the index is the cheapest overview
arkouda list -l && arkouda check

# Paths of all ADRs (for piping)
arkouda list

# Search ADRs for a topic — let list provide the search roots
arkouda list | xargs rg -i <topic>

# Read the decision of a specific ADR
arkouda decision use-postgres

# Read another section instead
arkouda decision use-postgres --section consequences

# Read the whole ADR — resolve the path through list
cat "$(arkouda list | grep -F /use-postgres.md)"

# Paths of accepted ADRs only
arkouda list -l | awk '$2=="accepted" {print $4}'

# Count ADRs by status
arkouda list -l | awk '{print $2}' | sort | uniq -c

# Most recent N decisions
arkouda list -l --sort timestamp | tail -10

# Stream every Decision section in the collection
arkouda list | while read f; do
  id=$(basename "$f" .md)
  printf '## %s\n\n' "$id"
  arkouda decision "$id"
  printf '\n'
done

# Scaffold a new decision and validate it
arkouda new "Adopt Tracing" --description "Use OpenTelemetry across services."
arkouda check
```

## Workflows

**Before deciding** — search what's already there:

```sh
arkouda list | xargs rg -i <topic>         # content search across all ADRs
arkouda list -l | awk '$2=="accepted"'     # accepted decisions only
arkouda decision <id>                      # read the meat of a hit
```

**After deciding** — capture it:

```sh
arkouda new "<Title>" --description "<one-line summary of what was decided>"
# arkouda new prints the path it created — open that file and fill in
# Context, Decision, Consequences
arkouda check
```

**Supersede an existing decision**

1. Resolve the path: `path=$(arkouda list | grep -F /<old-id>.md)`.
2. `cat "$path"` to see the current frontmatter, then edit: change `status: superseded` and add `superseded_by: <new-concept-id>` (the full bundle-relative id, e.g. `security/mtls`, not just the stem).
3. `arkouda new "<New Title>"` for the replacement.
4. `arkouda check` to confirm both files still validate.

## ADR shape (what `check` enforces)

The frontmatter is [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) v0.1. There is no `id` key — the concept id *is* the bundle-relative path without `.md`.

```markdown
---
type: Architecture Decision Record   # required by OKF; always this value
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

Required keys: `type`, `title`, `description`, `status`, `timestamp`. Required body sections (case-insensitive H2): `Status`, `Context`, `Decision`, `Consequences` — from [Michael Nygard's ADR template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard). `status`, `deciders`, and `superseded_by` are OKF producer extensions; `type`, `title`, `description`, `tags`, and `timestamp` are OKF's own fields.

## When `check` reports errors

Each diagnostic has a code; the hint usually tells you the exact fix.

- **E000** unparseable file → the file must start with YAML frontmatter delimited by `---`.
- **E001/E002** missing or empty required field → add the field with a real value.
- **E003** invalid status → use one of the five valid values.
- **E004** concept id is not a lowercase slug → rename the file (and any parent dirs) to letters, digits, single hyphens.
- **E005** wrong `type` → set `type: Architecture Decision Record`.
- **E006** invalid timestamp → ISO 8601, e.g. `2026-05-06` or `2026-05-06T14:30:00Z`.
- **E007/E008** missing or wrong H1 → first heading must be `# <title>`.
- **E009** missing required section → add the named `## Section` heading.
- **E010** duplicate concept id across files → make ids unique.
- **E011** `index.md` frontmatter → only a bundle-root index may have it, and only `okf_version`.
- **E012** `log.md` heading is not `## YYYY-MM-DD`.
- **E013** *(warning)* bundle declares an OKF version arkouda doesn't implement.
- **E014** *(warning)* `index.md` is stale → run `arkouda index`.

## What not to do

- Don't make a non-trivial decision without first checking existing ADRs.
- Don't hardcode `docs/adr/` in pipelines — different repos put ADRs elsewhere via `.arkoudarc.toml`. Use `arkouda list` to discover the actual paths.
- Don't write or edit ADR files freehand without running `arkouda check` afterwards — the schema is strict.
- Don't invent statuses outside the five valid values; downstream tooling depends on them.
- Don't move or rename a published ADR after creation — its path within the bundle *is* its concept id, so links and `superseded_by` values pointing at it will break. Create a new ADR and mark the old one `superseded` instead.
- Don't add an `id:` key to frontmatter; it was removed when arkouda moved to OKF. The concept id comes from the path within the bundle.
- Don't hand-edit `index.md` — it is generated by `arkouda index`, and edits are overwritten. `log.md` is yours to maintain: arkouda never writes it, only validates that its headings are `## YYYY-MM-DD`. Neither file is ever an ADR; both are reserved by OKF.
- Don't commit ADRs whose `arkouda check` fails — CI is likely to enforce it.
