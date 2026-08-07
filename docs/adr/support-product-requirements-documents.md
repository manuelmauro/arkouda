---
type: Architecture Decision Record
title: Support Product Requirements Documents
description: Generalize arkouda from an ADR-only tool into a concept-type-aware OKF tool, adding Product Requirements Documents as a second built-in concept type.
tags:
  - prd
  - okf
  - schema
  - cli
timestamp: 2026-08-07
status: proposed
deciders: []
---

# Support Product Requirements Documents

## Status

Proposed

## Context

Arkouda's pitch is that an AI coding agent should check what was already decided before deciding, and capture the outcome afterwards. Decisions are only half of what an agent needs. The other half is what the software is *supposed to do* — the requirements the decisions serve. Today an agent that wants that context has to read source, tests, and commit messages and infer it.

Product Requirements Documents are the established artefact for that, and they have the same properties that made ADRs worth tooling: Markdown, YAML frontmatter, one file per topic, versioned in the repo, written by humans and agents alike, read far more often than written.

[Adopting OKF](adopt-okf.md) already did most of the work of making a second document type possible. An arkouda ADR directory is an OKF v0.1 knowledge bundle, and OKF's central abstraction is the *concept*: a Markdown document whose only required frontmatter key is `type`. OKF does not say a bundle holds one type — `type` is per-concept precisely so a bundle can hold many. Everything arkouda does with a bundle today (recursive discovery, concept ids from paths, reserved `index.md`/`log.md`, frontmatter parsing that tolerates unknown keys, `index.md` generation) is already type-agnostic.

What is not type-agnostic is arkouda's own layer on top, and it is hardcoded in four places:

- **`ADR_TYPE`** is a `const` (`src/adr/frontmatter.rs`), and the validator rejects any other `type` with `E005` — so a PRD dropped into a bundle fails `arkouda check` today.
- **`AdrStatus`** is a five-variant enum (`src/adr/status.rs`) baked into `--status` as a clap `ValueEnum`, into `list --sort status`, and into `index.md` grouping. A PRD is never `superseded` by an argument; it is drafted, approved, shipped, or abandoned.
- **Required sections** are a literal array — `["status", "context", "decision", "consequences"]` (`src/adr/validator.rs`) — Michael Nygard's template, which is the right contract for a decision and the wrong one for a requirement.
- **`arkouda new`'s template** hardcodes the same four sections and `ADR_TYPE`.

So the question is not whether OKF permits a second type. It is whether arkouda grows a general notion of concept type, or whether PRDs get a parallel tool.

A parallel tool is unattractive: discovery, config resolution, frontmatter parsing, `index.md` generation, id validation, the `E000`–`E014` diagnostic vocabulary, and the agent skill would all be duplicated, and an agent would have to learn two CLIs to answer "what are we building and why did we build it that way". Those are the same question.

Arkouda is pre-1.0. It has made exactly one breaking schema change so far, and this is the moment to decide whether the tool is *an ADR tool* or *an OKF tool with opinionated built-in types*.

## Decision

Generalize arkouda into a concept-type-aware OKF tool, and ship **Product Requirements Document** as its second built-in type.

### A concept-type descriptor replaces the hardcoded constants

Introduce a `ConceptType` descriptor in `src/adr/` (renamed to `src/concept/`) that carries everything currently hardcoded:

| Field              | ADR                                                  | PRD                                                                 |
| ------------------ | ---------------------------------------------------- | ------------------------------------------------------------------- |
| `slug` (CLI name)  | `adr`                                                | `prd`                                                                |
| OKF `type`         | `Architecture Decision Record`                       | `Product Requirements Document`                                      |
| statuses           | `proposed`, `accepted`, `superseded`, `deprecated`, `rejected` | `draft`, `in-review`, `approved`, `shipped`, `abandoned`, `superseded` |
| required sections  | `Status`, `Context`, `Decision`, `Consequences`      | `Status`, `Context`, `Requirements`, `Success Metrics`               |
| primary section    | `Decision`                                           | `Requirements`                                                       |
| default directory  | `docs/adr`                                           | `docs/prd`                                                           |

The two descriptors are the only instances; types are **not** user-definable in this change (see Alternatives). Status order within a descriptor is lifecycle order, which is what `index.md` grouping and `--sort status` use.

Required frontmatter keys are the same for both types — `type`, `title`, `description`, `status`, `timestamp` — because they are OKF's own recommended set, not an ADR invention. A PRD's `description` summarizes what is being built, as an ADR's summarizes what was decided.

### The PRD schema

```markdown
---
type: Product Requirements Document
title: Bulk ADR Import
description: One-line summary of what is being built and for whom.
tags: []
timestamp: 2026-08-07
status: draft                        # draft | in-review | approved | shipped | abandoned | superseded
resource: https://github.com/org/repo/issues/42   # optional: the tracker item
owner:                               # optional; PRD extension, mirrors `deciders`
  - alice
---

# Bulk ADR Import                    # H1 must equal title, as for ADRs

## Status

Draft

## Context

The problem, who has it, and why now.

## Goals                             # scaffolded, not required

## Non-Goals                         # scaffolded, not required

## Requirements

What the software must do. The primary section.

## Success Metrics

How we will know it worked.

## Open Questions                    # scaffolded, not required
```

Four required sections, mirroring Nygard's four in count and intent: the situation (`Context`), the substance (`Requirements`), and the falsifiable claim about the outcome (`Success Metrics`), plus `Status` so lifecycle lives in the body as well as the frontmatter. `Goals`, `Non-Goals`, and `Open Questions` are scaffolded by `arkouda new` but not validated — the same treatment arkouda's own ADRs give `Alternatives Considered` and `Citations`. Requiring them would make the cheapest useful PRD expensive to write; scaffolding them makes writing them the default.

`owner` is a new optional producer extension (OKF §4.1), parallel to `deciders`. `resource` already exists in the frontmatter struct and is where a PRD's tracker link goes. `superseded_by` works unchanged for both types.

### CLI surface

Three additions, no new subcommands:

- **`arkouda new "<title>" [--type adr|prd]`** — defaults to `adr`. Selects the template, the type string, the status vocabulary, and the target directory.
- **`arkouda list [--type adr|prd]`** — filters. The `-l` columns stay exactly as they are: `ID STATUS TIMESTAMP PATH TITLE — DESCRIPTION`. Inserting a type column would break every documented `awk '$2=="accepted"'` pipeline, and the path column already discriminates under the default per-type directories.
- **`arkouda decision <id>`** — prints the concept's *primary section*: `## Decision` for an ADR, `## Requirements` for a PRD. `--section <name>` is unchanged and already type-neutral.

`check` and `index` gain no flags. `--status` on `new` stops being a clap `ValueEnum` and becomes a string validated against the resolved type's vocabulary.

Keeping the subcommand named `decision` while it may print a PRD's requirements is a deliberate wart: it keeps the surface at five subcommands and breaks nothing, and the ADR case dominates. [The decision-centric CLI shape](ls-style-list-and-decision.md) is unchanged — the body of a concept, for arkouda's purposes, is still its one section that matters.

### Configuration

`.arkoudarc.toml` keeps the flat form and gains a per-type table:

```toml
# Flat form, unchanged: every type shares these roots.
dirs = ["docs/adr"]
```

```toml
# Typed form: roots per type.
[dirs]
adr = ["docs/adr"]
prd = ["docs/prd"]
```

Both parse into one internal model — a type-to-roots map, plus the union used for search. `list`, `check`, and `decision` aggregate over the union exactly as they do now; `new` writes into the first root for the resolved type. With nothing configured, the defaults are `adr → docs/adr` and `prd → docs/prd`. `--dir`/`ADR_DIR` still override everything for a single invocation.

A bundle may hold mixed types. Type comes from frontmatter, never from the path — the typed `dirs` table is a default write target and a search scope, not a schema.

### Validation

`arkouda check` validates every concept against the descriptor its `type` names. **No new diagnostic codes**; three change meaning to be type-relative:

- `E005` — `type` is not one of the types arkouda knows (was: `type` is not `Architecture Decision Record`).
- `E003` — `status` is not in *this type's* vocabulary. The hint lists that type's values.
- `E009` — a section required by *this type* is missing.

A concept with an unknown `type` is an `E005` error rather than a skipped file. Arkouda manages the bundles it is pointed at; silently ignoring a document it cannot check would hide exactly the drift `check` exists to catch.

### `index.md`

Group by type, then by status: `# <Type>` at H1, `## <Status>` at H2, entries beneath. Today's index groups by status at H1, so every existing bundle's index changes shape once and must be regenerated with `arkouda index`. Until then `E014` (stale index) fires, which is a warning and never fails a bundle — the OKF §9 permissive-consumption behaviour arkouda already implements.

Uniform nesting beats conditional nesting. A single-type bundle pays one extra heading level; consumers parse one shape instead of two.

## Consequences

### Positive

- An agent can answer "what are we building" and "why is it built this way" from one CLI, one config file, one diagnostic vocabulary, and one skill — and `arkouda list | xargs rg -i <topic>` searches both at once.
- The hardcoded type, status list, section list, and template collapse into one descriptor, so a third type (RFC, runbook, postmortem) becomes a data change rather than a refactor.
- PRDs get the property that made ADRs worth tooling: a strict schema with machine-readable diagnostics, so CI can gate on a PRD being well-formed and an agent gets an actionable error instead of a shrug.
- Arkouda becomes closer to what OKF describes — a consumer of typed concepts — rather than a tool that reads OKF bundles but only believes in one type.

### Negative

- `--status` loses its static completion values, because valid statuses now depend on `--type`. Shell completions will offer nothing for it until completions learn to be type-aware.
- `arkouda decision` is a misleading name when the concept is a PRD. The alternative was a subcommand per type, which does not scale.
- Every existing `index.md` regenerates with a new heading structure — a one-line diff of churn per bundle, and a stale-index warning until someone runs `arkouda index`.
- Two vocabularies means `arkouda list --sort status` orders within a type but interleaves across types. Sorting a mixed collection by status is now only meaningful with `--type`.
- The module named `adr` becomes the module named `concept`, and `AdrStatus`, `ADR_TYPE`, and `ADR_DIR` are all named after one of two types. The env var stays `ADR_DIR` for compatibility; the internals get renamed.

### Neutral

- No new diagnostic codes, so the `E000`–`E014` contract that CI and agents key on is unchanged in shape.
- Discovery, concept ids, reserved filenames, id slug rules, and duplicate-id detection are untouched — they were already type-agnostic.
- Arkouda's own `docs/adr/` bundle is unaffected until it grows a PRD. Existing ADRs validate as-is.
- `skills/use-arkouda/SKILL.md` and the README both widen: the skill must teach when to write a PRD versus an ADR, or an agent will keep reaching for `arkouda new` with the default type.

## Alternatives Considered

### Ship a separate `prd` binary

Cleanest separation, zero risk to the ADR path. It also duplicates discovery, config, frontmatter parsing, index generation, and the diagnostic vocabulary — and forces an agent to learn two tools to answer one question. The shared machinery is most of the code; the type-specific part is a template and two lists.

### Make concept types fully user-definable in `.arkoudarc.toml`

Let a project declare any `type` with its own sections and statuses. Strictly more powerful, and the descriptor introduced here is the data model that would enable it later. Rejected for now because arkouda's value is that the schema is *known*: an agent that has read the skill knows what a PRD looks like without reading a config file, and a user-defined type has no template, no default sections, and no shared vocabulary across repos. Two curated types beat n bespoke ones until there is demand.

### Treat a PRD as an ADR with `tags: [prd]`

Zero code change. It also means `arkouda check` validates a requirements document against Nygard's sections, `arkouda new` scaffolds the wrong template, `status: accepted` has to stand in for `shipped`, and `type` — the one field OKF requires — lies to every other OKF consumer. The whole point of a strict schema is lost when one type is used to smuggle another.

### Add a type column to `list -l`

More informative in a mixed bundle, and it would break `awk '$2=="accepted"'` — a pipeline shape the README, the skill, and any agent that has read either depend on. `--type` filtering gives the same information without moving a column.

### Keep `index.md` flat when a bundle holds one type

Byte-identical output for every existing bundle and no regeneration churn. It also gives consumers two shapes to parse for the same file, and the churn is one command run once.

## Citations

[1] [Open Knowledge Format v0.1 specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) — §4.1 producer extensions, §6 index, §9 permissive consumption
[2] [Adopt the Open Knowledge Format](adopt-okf.md) — the migration that made a second concept type possible
[3] [ls-style list and a decision subcommand](ls-style-list-and-decision.md) — the primary-section CLI contract this generalizes
[4] [Defer to Unix tools](defer-to-unix-tools.md) — why `--type` is a filter rather than a new subcommand
[5] [Michael Nygard's ADR template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard)
