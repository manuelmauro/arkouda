---
name: arkouda
description: Navigate and validate Architecture Decision Records with arkouda
license: MIT
---

# Arkouda

Use this skill when working in a repository that stores Architecture Decision Records (ADRs) as Markdown files with YAML frontmatter, and the `arkouda` CLI is available. The tool reads, validates, and scaffolds those records, and pulls a named `## Section` out of one. For content search, full-file display, or any other shell-friendly operation, lean on `rg`/`grep`/`cat` — arkouda does not reinvent those.

## When to use

- The user asks to list, find, show, or summarise ADRs in a repo.
- The user asks to add a new decision or change an existing one's status.
- The user asks why something was done a certain way and the answer is likely captured in an ADR.
- You see a `docs/adr/` directory (or any directory of Markdown files with `id`/`status`/`date` frontmatter) and the user is asking architectural questions.

If the repository has no ADRs yet, suggest creating one with `arkouda new` rather than freelancing a Markdown file — the tool enforces the schema.

## Where ADRs live

Default directory: `docs/adr/`. Override with `--dir <path>` on any command, or set `ADR_DIR=<path>` once for the session. Filenames are the ADR id with a `.md` suffix (e.g. `use-postgres.md`).

## Commands

Run `arkouda --help` and `arkouda <subcommand> --help` to see the authoritative CLI surface. The four subcommands all have a clear reason to exist that `ls`/`cat`/`grep` cannot replicate:

- **`arkouda list [--sort id|date|status] [-l]`** — print one ADR file path per line. No header, no padding — pipe straight into `xargs`/`rg`/`cat`/`wc`. With `-l`, print a headerless table of `ID STATUS DATE PATH TITLE` for human skimming (still pipe-friendly: `arkouda list -l | awk '$2=="accepted" {print $4}'`).
- **`arkouda decision <id> [--section <name>]`** — print the body of that ADR's `## Decision` section (case-insensitive H2, blank lines trimmed). `<id>` accepts the frontmatter id, the filename stem, or the filename. Use `--section <name>` to print a different section (e.g. `context`, `consequences`, `status`, or any custom heading); errors if the section is missing. For the full file, just `cat docs/adr/<id>.md`.
- **`arkouda check`** — validate every ADR's frontmatter, filename, and required Markdown sections. Exit code 0 if clean, 1 if any errors. Each error has a code (E000–E010) and a fix hint.
- **`arkouda new "<title>" [--id <slug>] [--status proposed|accepted|superseded|deprecated|rejected] [--abstract "<one-line summary of the decision>"]`** — scaffold a new ADR with the standard template and today's date. The id defaults to a slug derived from the title. The abstract should summarize *what was decided*, not just the topic.

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`.

Arkouda intentionally has no `search` subcommand and no full-file `show`. Use `rg <query> docs/adr/` (or `grep -ri`) for content search, `cat docs/adr/<id>.md` for the full file, and pipe arkouda's structured output through `awk`/`xargs`/etc. when you need to combine queries. See ADRs `defer-to-unix-tools` and `ls-style-list-and-decision` for the rationale.

## ADR shape (what `check` enforces)

Every file must start with YAML frontmatter delimited by `---`:

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

Required keys: `id`, `title`, `abstract`, `status`, `date`. Required body sections (case-insensitive H2): `Status`, `Context`, `Decision`, `Consequences` — these come from [Michael Nygard's ADR template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard). Filename stem must equal the frontmatter `id`.

## Common workflows

**Get oriented in a new repo**

```sh
arkouda list -l        # headerless table — id, status, date, path, title
arkouda check          # surface any drift before reading further
```

**Answer "did we ever decide on X?"**

```sh
rg -i X docs/adr/                  # find files containing X
arkouda decision <id>              # read what was decided
cat docs/adr/<id>.md               # read the full ADR if you need context
```

**Skim every decision (or context, consequences, …) at once**

```sh
# Single ADR, single section — direct CLI support:
arkouda decision use-postgres
arkouda decision use-postgres --section context

# Across the collection, compose with the shell. arkouda list emits paths,
# so the section extractor is a small awk/while loop.
arkouda list | while read f; do
  id=$(basename "$f" .md)
  echo "## $id"
  arkouda decision "$id"
  echo
done
```

**Propose a new decision**

```sh
arkouda new "Adopt Tracing"
# edit docs/adr/adopt-tracing.md to fill in Context, Decision, Consequences
arkouda check          # confirm it validates before committing
```

**Mark a decision superseded**

1. Run `cat docs/adr/<old-id>.md` to see the current frontmatter.
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
