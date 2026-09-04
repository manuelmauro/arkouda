---
name: use-arkouda
description: Find prior decisions and product requirements, and record new ones, in a repo's arkouda collection of ADRs (Architecture Decision Records) and PRDs (Product Requirements Documents). Invoke any time you're about to make a non-trivial design, architecture, library, schema, or convention decision, or about to build a feature — check what was already decided and what is already required before deciding, and capture the outcome afterwards.
license: MIT
---

# Using arkouda

In repositories that record decisions and requirements as Markdown files with YAML frontmatter (conventionally under `docs/adr/` and `docs/prd/`), `arkouda` is the CLI for finding, reading, validating, and scaffolding them. **Before you decide, check what's already been decided. Before you build, check what's already required. After you decide, capture it.**

An arkouda directory is an [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) (OKF) v0.1 *knowledge bundle*: each document is a *concept* whose id is its path within the bundle without the `.md` suffix (`security/mtls.md` → `security/mtls`). `index.md` and `log.md` are reserved by OKF and are never concepts.

If a repo has no such directory yet but the `arkouda` binary is installed, this skill is also the right one to reach for: `arkouda new` enforces the schema from the first file.

## Concept types

Arkouda has two built-in types, and picking the wrong one produces a document that fails `arkouda check`.

|                   | ADR (`--type adr`, the default)                                                 | PRD (`--type prd`)                                                                      |
|-------------------|---------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------|
| Answers           | Why is the software built this way?                                             | What is the software supposed to do?                                                    |
| Write one when    | you commit to a library, datastore, transport, layout, convention, or trade-off | you start on a feature whose scope, boundaries, or success criteria aren't written down |
| Statuses          | `proposed`, `accepted`, `superseded`, `deprecated`, `rejected`                  | `draft`, `in-review`, `approved`, `shipped`, `abandoned`, `superseded`                  |
| Required sections | `Status`, `Context`, `Decision`, `Consequences`                                 | `Status`, `Problem`, `Requirements`, `Non-Goals`, `Success Metrics`                     |
| Primary section   | `Decision`                                                                      | `Requirements`                                                                          |
| Default directory | `docs/adr`                                                                      | `docs/prd`                                                                              |

The two are linked from the PRD side: a PRD's `decisions` frontmatter key lists the concept ids of the ADRs that shaped it. Traverse from a requirement to its rationale by reading that key — never by restating the decision inside the PRD.

**Apart from `Status`, the two share no section headings.** A PRD has no `## Context`; its motivation is `## Problem`. Asking for a section the concept's type doesn't have is an error, not a near-miss. If you don't know which type a concept is, run `arkouda list -l` and read the type column, or just omit the section name and let arkouda pick the primary one.

### The project may define more

**A project can declare its own types with `[[types]]` in `.arkoudarc.toml`, and it can replace the built-in ADR or PRD contract with its own.** So the two tables above describe arkouda's defaults, not necessarily *this* repo. Before you scaffold anything in an unfamiliar repo:

```sh
arkouda --help                    # nothing about types here — it is per project
cat .arkoudarc.toml 2>/dev/null   # the authoritative list of this repo's types
arkouda list -l                   # the type column shows what is actually in use
```

A `[[types]]` table gives its type a `slug` (what `--type` takes), an `okf_type` (what its documents declare), a status lifecycle, optionally `required_sections` and a `primary_section`, a `default_dir`, and optionally a `template`. Read the table and follow it exactly as you would the built-in contracts — `arkouda new --type <slug>` scaffolds from it, and `arkouda check` enforces it.

`arkouda new --type <slug>` with an unknown slug lists the slugs that do exist, so that error is the fastest way to see this repo's types if there is no config file to read.

## When to use

Reach for this skill any time you're about to make a non-trivial decision or start non-trivial work. Concretely:

- Before writing code that picks a library, framework, datastore, encoding, transport, or other "we now depend on X" commitment. (**ADR**)
- Before changing a public interface, file layout, schema, naming convention, or directory structure. (**ADR**)
- Before refactoring away from a pattern you didn't introduce — you may be about to undo a deliberate decision. (**ADR**)
- Before building a feature of any size: check whether a PRD already defines its scope, its non-goals, and what success means. (**PRD**)
- When the user asks "did we ever decide on X?", "why is it done this way?", "what are we building?", or "is X in scope?".
- When the user asks for a new ADR or PRD, or to mark one superseded.
- Whenever you land in an unfamiliar repo with an arkouda collection.

A 5-second `arkouda list | xargs rg -i <topic>` is cheaper than redoing a debate that's already in the file, or building something a PRD explicitly listed as a non-goal.

## Philosophy

Three principles shape arkouda's behaviour, and explain why some defaults look minimal:

- **Defer to Unix tools.** Arkouda earns subcommands only where standard shell tools (`rg`, `grep`, `cat`, `awk`, `xargs`) cannot. Content search, full-file printing, counting, and slicing are left to the shell — the CLI emits structured output you compose with the rest of your toolbox. Hence: no `search`, no full-file `show`.
- **Primary-section defaults.** `arkouda list` prints one path per line (no header, no padding) so it pipes cleanly. Each type has one section carrying its substance, so `arkouda section <id>` with no section name prints that one — `## Decision` for an ADR, `## Requirements` for a PRD. Supporting sections are named explicitly.
- **Standard format over bespoke.** Documents are stored as OKF concepts, and a concept's `type` is a frontmatter field, so any OKF-aware consumer can read them and one bundle can hold both types.

The source rationale lives in arkouda's own repo, in the ADRs [`defer-to-unix-tools`](https://github.com/manuelmauro/arkouda/blob/main/docs/adr/defer-to-unix-tools.md), [`ls-style-list-and-decision`](https://github.com/manuelmauro/arkouda/blob/main/docs/adr/ls-style-list-and-decision.md), [`adopt-okf`](https://github.com/manuelmauro/arkouda/blob/main/docs/adr/adopt-okf.md), and [`support-product-requirements-documents`](https://github.com/manuelmauro/arkouda/blob/main/docs/adr/support-product-requirements-documents.md).

## Where documents live

The location varies between repos. Don't hardcode `docs/adr/` in pipelines — ask arkouda. Run **`arkouda list`** to get the actual paths for the repo you're in.

Resolution order, in case you need to set or override the location:

1. `--dir <path>` flag (one-shot override, single directory, applies to every type).
2. `ADR_DIR=<path>` environment variable (session override, single directory).
3. `.arkoudarc.toml` at the repo root (or any ancestor of the cwd), in either of two forms:
   ```toml
   # Flat: every type shares these roots. Useful in monorepos.
   dirs = ["docs/adr", "services/billing/docs/adr"]
   ```
   ```toml
   # Typed: roots per type.
   [dirs]
   adr = ["docs/adr"]
   prd = ["docs/prd"]
   ```
   Relative paths resolve against the config file's directory. `arkouda list`, `check`, `section`, and `index` work over the union of every root — type comes from frontmatter, not from the directory — while `arkouda new` writes into the first root configured for the type it is creating. A `[dirs]` key must name a type the project has: a built-in, or one of its own `[[types]]`.
4. Default: each type's own `default_dir` — `docs/adr/` for ADRs, `docs/prd/` for PRDs.

A concept id is the document's path *within its bundle*, minus the `.md` suffix — not just the filename. A top-level `use-postgres.md` has the id `use-postgres`; a nested `security/mtls.md` has the id `security/mtls`. `arkouda section` accepts the full concept id (`security/mtls`), the bare stem (`mtls`), or the filename.

## Commands

Five subcommands, each doing something the shell can't:

- **`arkouda list [--sort id|timestamp|status] [--type <slug>] [-l]`** — one path per line. Pipe straight into `xargs`/`rg`/`cat`/`wc`. With `-l`, a headerless `ID TYPE STATUS TIMESTAMP PATH TITLE — DESCRIPTION` table for human skimming and for `awk`. `--type` filters by frontmatter type; valid slugs are this project's, not a fixed `adr|prd`.
- **`arkouda section <id> [<name>]`** — body of that concept's primary section (`Decision` for an ADR, `Requirements` for a PRD). Give a `<name>` for any other heading (`context`, `consequences`, `problem`, `non-goals`, `success metrics`, `status`, or custom). Errors if the section is missing. For the full file, resolve the path through `arkouda list` and `cat` it.
- **`arkouda check`** — validates in three tiers: OKF conformance for every concept, arkouda's frontmatter profile for concepts whose `type` the project configures, and that type's status vocabulary and required sections. Exit 0 clean, 1 on any error. Each diagnostic carries a code (E000–E015) and a fix hint. Warnings never fail the run. A concept whose `type` no `[[types]]` table configures is checked for OKF conformance only and warned about (`E005`) — it is neither skipped nor a failure.
- **`arkouda new "<title>" [--type <slug>] [--id <slug>] [--status <value>] [--description "<one-line summary>"]`** — scaffold a new concept with today's date, from that type's template. Defaults to `--type adr`, which a project that replaces or omits the built-in ADR will reject — read `.arkoudarc.toml` first. `--status` must come from the chosen type's vocabulary and defaults to the first of its lifecycle (`proposed` for an ADR, `draft` for a PRD). Default id is a slug from the title. The description should summarize *what was decided* or *what is being built*, not just the topic. Refreshes `index.md` if the bundle has one.
- **`arkouda index`** — regenerate each bundle's `index.md`, an OKF §6 listing of every concept under `# <Type>` then `## <Status>`. Read it to see the whole collection at a glance without opening any file.

Global flags: `--dir <path>` (also `ADR_DIR`), `-q/--quiet`. Run `arkouda --help` or `arkouda <subcommand> --help` for the authoritative surface.

There is intentionally no `search` subcommand and no full-file `show` — `rg`/`grep` and `cat` already do those.

## One-liners

`arkouda list` is the path source — it's where the documents *actually* are in this repo.

```sh
# Orient in an unfamiliar repo — the type column tells you what's here
arkouda list -l && arkouda check

# Paths of everything (for piping)
arkouda list

# Search for a topic — let list provide the search roots
arkouda list | xargs rg -i <topic>

# Read the substance of a specific concept (Decision, or Requirements)
arkouda section use-postgres

# Read another section instead
arkouda section use-postgres consequences
arkouda section bulk-import non-goals

# Read the whole document — resolve the path through list
cat "$(arkouda list | grep -F /use-postgres.md)"

# Paths of accepted ADRs only  (note: status is $3, path is $5)
arkouda list -l --type adr | awk '$3=="accepted" {print $5}'

# Count concepts by status
arkouda list -l | awk '{print $3}' | sort | uniq -c

# Count concepts by type
arkouda list -l | awk '{print $2}' | sort | uniq -c

# Most recent N concepts
arkouda list -l --sort timestamp | tail -10

# Stream every primary section in the collection
arkouda list | while read f; do
  id=$(basename "$f" .md)
  printf '## %s\n\n' "$id"
  arkouda section "$id"
  printf '\n'
done

# Which ADRs shaped a PRD
arkouda list | grep -F /bulk-import.md | xargs sed -n '/^decisions:/,/^[a-z_]*:/p'

# Scaffold and validate
arkouda new "Adopt Tracing" --description "Use OpenTelemetry across services."
arkouda new "Bulk Import" --type prd --description "Import loose Markdown files as ADRs."
arkouda check
```

## Workflows

**Before deciding** — search what's already there:

```sh
arkouda list | xargs rg -i <topic>              # content search across everything
arkouda list -l --type adr | awk '$3=="accepted"'  # accepted decisions only
arkouda section <id>                            # read the meat of a hit
```

**Before building** — check the requirements, especially the non-goals:

```sh
arkouda list --type prd | xargs rg -i <feature>
arkouda section <prd-id>                        # the Requirements section
arkouda section <prd-id> non-goals              # what is explicitly out of scope
```

**After deciding** — capture it:

```sh
arkouda new "<Title>" --description "<one-line summary of what was decided>"
# arkouda new prints the path it created — open that file and fill in
# Context, Decision, Consequences
arkouda check
```

**Writing a PRD**

```sh
arkouda new "<Title>" --type prd --description "<what is being built, for whom>"
# Fill in Problem, Requirements, Non-Goals, Success Metrics (required), and
# Approach / Open Questions (scaffolded, optional). Then link the decisions
# that shaped it by adding their concept ids to frontmatter:
#   decisions:
#     - adopt-okf
#     - defer-to-unix-tools
arkouda check
```

**Supersede an existing document**

1. Resolve the path: `path=$(arkouda list | grep -F /<old-id>.md)`.
2. `cat "$path"` to see the current frontmatter, then edit: change `status: superseded` (valid for both types) and add `superseded_by: <new-concept-id>` (the full bundle-relative id, e.g. `security/mtls`, not just the stem).
3. `arkouda new "<New Title>" [--type prd]` for the replacement.
4. `arkouda check` to confirm both files still validate, and that the `superseded_by` reference resolves (a dangling one is `E015`).

## Shape (what `check` enforces)

The frontmatter is [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) v0.1. There is no `id` key — the concept id *is* the bundle-relative path without `.md`. Required keys are the same for both types: `type`, `title`, `description`, `status`, `timestamp`. Required *sections* differ, and `type` is what selects them.

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

Sections from [Michael Nygard's ADR template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard).

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

`status`, `deciders`, `superseded_by`, `owner`, `target_release`, and `decisions` are OKF producer extensions; `type`, `title`, `description`, `resource`, `tags`, and `timestamp` are OKF's own fields.

Arkouda validates document *structure*, not requirement *content*. It does not enforce `FR-###` numbering, `P1`/`P2` priorities, or EARS acceptance-criteria syntax. Those are good conventions to use *inside* a `## Requirements` section; the validator's contract is headings and frontmatter.

## When `check` reports errors

Each diagnostic has a code; the hint usually tells you the exact fix.

- **E000** unparseable file → the file must start with YAML frontmatter delimited by `---`.
- **E001/E002** missing or empty required field → add the field with a real value.
- **E003** invalid status → use a value from *this type's* vocabulary; the hint lists them.
- **E004** concept id is not a lowercase slug → rename the file (and any parent dirs) to letters, digits, single hyphens.
- **E005** *(warning)* no configured type declares this `type` → either the value is a typo (fix it to a configured `okf_type`; the hint lists them), or the project has not declared this type yet. Until it does, the concept is checked for OKF conformance only — its status and sections are not validated. A warning rather than an error because a conformant OKF bundle may legitimately hold types this project has not described.
- **E006** invalid timestamp → ISO 8601, e.g. `2026-05-06` or `2026-05-06T14:30:00Z`.
- **E007/E008** missing or wrong H1 → first heading must be `# <title>`.
- **E009** missing required section → add the named `## Section`. Which ones are required depends on `type`, and a project's own type may require none at all.
- **E010** duplicate concept id across files → make ids unique.
- **E011** `index.md` frontmatter → only a bundle-root index may have it, and only `okf_version`.
- **E012** `log.md` heading is not `## YYYY-MM-DD`.
- **E013** *(warning)* bundle declares an OKF version arkouda doesn't implement.
- **E014** *(warning)* `index.md` is stale → run `arkouda index`.
- **E015** *(warning)* a `decisions` or `superseded_by` entry doesn't resolve to a loaded concept → fix the id, or widen `--dir`/`dirs` if it lives in a bundle this run didn't load. It's a warning precisely because arkouda can't tell a broken reference from an out-of-scope one.

## What not to do

- Don't make a non-trivial decision, or start non-trivial work, without first checking what's already recorded.
- Don't hardcode `docs/adr/` in pipelines — different repos put these documents elsewhere via `.arkoudarc.toml`. Use `arkouda list` to discover the actual paths.
- Don't reach for the default `--type adr` when you're describing what to build; that's a PRD. And don't file a technical trade-off as a PRD.
- Don't ask a PRD for its `## Context` or an ADR for its `## Requirements` — apart from `Status`, the section vocabularies don't overlap. Omit the section name to get the right one for the type.
- Don't restate a decision inside a PRD. Link it with the `decisions` frontmatter key and let the ADR carry the reasoning.
- Don't write or edit these files freehand without running `arkouda check` afterwards — the schema is strict for every type the project configures.
- Don't invent statuses. Each type has its own closed list, and `shipped` on an ADR (or `accepted` on a PRD) is an `E003`.
- Don't move or rename a published document after creation — its path within the bundle *is* its concept id, so links, `superseded_by`, and `decisions` values pointing at it will break. Create a new one and mark the old one `superseded` instead.
- Don't add an `id:` key to frontmatter; it was removed when arkouda moved to OKF. The concept id comes from the path within the bundle.
- Don't hand-edit `index.md` — it is generated by `arkouda index`, and edits are overwritten. `log.md` is yours to maintain: arkouda never writes it, only validates that its headings are `## YYYY-MM-DD`. Neither file is ever a concept; both are reserved by OKF.
- Don't commit documents whose `arkouda check` fails — CI is likely to enforce it. Do read the warnings too: an `E005` on a document you just wrote means you gave it a `type` this project doesn't configure, and nothing checked its shape.
- Don't assume `--type adr` exists. It is the default, but a project may replace or omit the built-in; read `.arkoudarc.toml`, or let `arkouda new --type` tell you what the slugs are.
