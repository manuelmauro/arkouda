---
name: arkouda
description: Navigate and validate Architecture Decision Records with arkouda
license: MIT
---

# Arkouda

Use this skill when working in a repository that stores Architecture Decision Records (ADRs) as Markdown files with YAML frontmatter, and the `arkouda` CLI is available. The tool reads, validates, and scaffolds those records, and resolves an id to a section. For content search, lean on `rg`/`grep` — arkouda does not reinvent that.

## When to use

- The user asks to list, find, show, or summarise ADRs in a repo.
- The user asks to add a new decision or change an existing one's status.
- The user asks why something was done a certain way and the answer is likely captured in an ADR.
- You see a `docs/adr/` directory (or any directory of Markdown files with `id`/`status`/`date` frontmatter) and the user is asking architectural questions.

If the repository has no ADRs yet, suggest creating one with `arkouda new` rather than freelancing a Markdown file — the tool enforces the schema.

## Where ADRs live

Default directory: `docs/adr/`. Override with `--dir <path>` on any command, or set `ADR_DIR=<path>` once for the session. Filenames are the ADR id with a `.md` suffix (e.g. `use-postgres.md`).

## Commands

Run `arkouda --help` and `arkouda <subcommand> --help` to see the authoritative CLI surface. The four subcommands:

- **`arkouda list [--sort id|date|status] [--section <name>]`** — print a table of every ADR in the directory. With `--section <name>` (e.g. `decision`, `context`, `consequences`, `status`), print a Markdown digest of that section across all ADRs instead, skipping any that lack it. Use the default to orient; use `--section decision` to scan everything that's been decided.
- **`arkouda show <id> [--section <name>]`** — print one ADR's full Markdown to stdout. `<id>` accepts the frontmatter id, the filename stem, or the filename. With `--section <name>`, print only that section's body; errors if the ADR has no such section.
- **`arkouda check`** — validate every ADR's frontmatter, filename, and required Markdown sections. Exit code 0 if clean, 1 if any errors. Each error has a code (E000–E010) and a fix hint.
- **`arkouda new "<title>" [--id <slug>] [--status proposed|accepted|superseded|deprecated|rejected] [--abstract "<one-line summary>"]`** — scaffold a new ADR with the standard template and today's date. The id defaults to a slug derived from the title.

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`.

Arkouda intentionally has no `search` subcommand. Use `rg <query> docs/adr/` (or `grep -ri`) for content search, and pipe arkouda's structured output through `awk`/`jq`/etc. when you need to combine queries. See ADR `defer-to-unix-tools` for the rationale.

## ADR shape (what `check` enforces)

Every file must start with YAML frontmatter delimited by `---`:

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

Required keys: `id`, `title`, `abstract`, `status`, `date`. Required body sections (case-insensitive H2): `Status`, `Context`, `Decision`, `Consequences`. Filename stem must equal the frontmatter `id`.

## Common workflows

**Get oriented in a new repo**

```sh
arkouda list
arkouda check          # surface any drift before reading further
```

**Answer "did we ever decide on X?"**

```sh
rg -i X docs/adr/                  # find files containing X
arkouda show <id>                  # read the hits
arkouda list --section decision \
  | rg -i X                        # search just inside Decision sections
```

**Skim every decision (or context, consequences, …) at once**

```sh
arkouda list --section decision
arkouda list --section consequences --sort status
arkouda show use-postgres --section decision    # just one ADR's section
```

**Propose a new decision**

```sh
arkouda new "Adopt Tracing"
# edit docs/adr/adopt-tracing.md to fill in Context, Decision, Consequences
arkouda check          # confirm it validates before committing
```

**Mark a decision superseded**

1. Run `arkouda show <old-id>` to see the current frontmatter.
2. Edit the file: change `status: "superseded"` and add `superseded_by: "<new-id>"`.
3. Create the replacement with `arkouda new "<New Title>"`.
4. Run `arkouda check` to confirm both files still validate.

## When `check` reports errors

Each diagnostic has a code. The hint usually tells you the exact fix.

- **E001/E002** missing or empty required field → add the field with a real value.
- **E003** invalid status → use one of the five valid values.
- **E004** id is not a lowercase slug → use letters, digits, and single hyphens.
- **E005** filename does not match id → rename the file to `<id>.md`.
- **E006** invalid date → use ISO `YYYY-MM-DD` for a real calendar day.
- **E007/E008** missing or wrong H1 → ensure the body's first heading is `# <title>`.
- **E009** missing required section → add the named `## Section` heading.
- **E010** duplicate id across files → make ids unique.

## What not to do

- Don't write or edit ADR files freehand without running `arkouda check` afterwards — the schema is strict.
- Don't invent statuses outside the five valid values; downstream tooling depends on them.
- Don't change a published ADR's `id` after creation; create a new ADR and mark the old one `superseded` instead.
- Don't commit ADRs whose `arkouda check` fails — the project's CI is likely to enforce it.
