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

Arkouda's pitch is that an AI coding agent should check what was already decided before deciding, and capture the outcome afterwards. Decisions are only half of what an agent needs. The other half is what the software is _supposed to do_ — the requirements the decisions serve. Today an agent that wants that context has to read source, tests, and commit messages and infer it.

Product Requirements Documents are the established artefact for that, and they have the same properties that made ADRs worth tooling: Markdown, YAML frontmatter, one file per topic, versioned in the repo, written by humans and agents alike, read far more often than written.

[Adopting OKF](adopt-okf.md) already did most of the work of making a second document type possible. An arkouda ADR directory is an OKF v0.1 knowledge bundle, and OKF's central abstraction is the _concept_: a Markdown document whose only required frontmatter key is `type`. OKF does not say a bundle holds one type — `type` is per-concept precisely so a bundle can hold many. Everything arkouda does with a bundle today (recursive discovery, concept ids from paths, reserved `index.md`/`log.md`, frontmatter parsing that tolerates unknown keys, `index.md` generation) is already type-agnostic.

What is not type-agnostic is arkouda's own layer on top, and it is hardcoded in four places:

- **`ADR_TYPE`** is a `const` (`src/adr/frontmatter.rs`), and the validator rejects any other `type` with `E005` — so a PRD dropped into a bundle fails `arkouda check` today.
- **`AdrStatus`** is a five-variant enum (`src/adr/status.rs`) baked into `--status` as a clap `ValueEnum`, into `list --sort status`, and into `index.md` grouping. A PRD is never `superseded` by an argument; it is drafted, approved, shipped, or abandoned.
- **Required sections** are a literal array — `["status", "context", "decision", "consequences"]` (`src/adr/validator.rs`) — Michael Nygard's template, which is the right contract for a decision and the wrong one for a requirement.
- **`arkouda new`'s template** hardcodes the same four sections and `ADR_TYPE`.

So the question is not whether OKF permits a second type. It is whether arkouda grows a general notion of concept type, or whether PRDs get a parallel tool.

A parallel tool is unattractive: discovery, config resolution, frontmatter parsing, `index.md` generation, id validation, the `E000`–`E014` diagnostic vocabulary, and the agent skill would all be duplicated, and an agent would have to learn two CLIs to answer "what are we building and why did we build it that way". Those are the same question.

Arkouda is pre-1.0. It has made exactly one breaking schema change so far, and this is the moment to decide whether the tool is _an ADR tool_ or _an OKF tool with opinionated built-in types_.

## Decision

Generalize arkouda into a concept-type-aware OKF tool, and ship **Product Requirements Document** as its second built-in type.

### A concept-type descriptor replaces the hardcoded constants

Introduce a `ConceptType` descriptor in `src/adr/` (renamed to `src/concept/`) that carries everything currently hardcoded:

| Field             | ADR                                                            | PRD                                                                    |
| ----------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `slug` (CLI name) | `adr`                                                          | `prd`                                                                  |
| OKF `type`        | `Architecture Decision Record`                                 | `Product Requirements Document`                                        |
| statuses          | `proposed`, `accepted`, `superseded`, `deprecated`, `rejected` | `draft`, `in-review`, `approved`, `shipped`, `abandoned`, `superseded` |
| required sections | `Status`, `Context`, `Decision`, `Consequences`                | `Status`, `Problem`, `Requirements`, `Non-Goals`, `Success Metrics`    |
| primary section   | `Decision`                                                     | `Requirements`                                                         |
| default directory | `docs/adr`                                                     | `docs/prd`                                                             |

The two descriptors are the only instances, and types are **not** user-definable in this change — but the descriptor is deliberately the shape a user-supplied template would deserialize into, so bringing your own type later is a config parser rather than a redesign (see Alternatives). Status order within a descriptor is lifecycle order, which is what `index.md` grouping and `--sort status` use.

Required frontmatter keys are the same for both types — `type`, `title`, `description`, `status`, `timestamp` — because they are OKF's own recommended set, not an ADR invention. A PRD's `description` summarizes what is being built, as an ADR's summarizes what was decided.

### The PRD schema

```markdown
---
type: Product Requirements Document
title: Bulk ADR Import
description: One-line summary of what is being built and for whom.
tags: []
timestamp: 2026-08-07
status: draft # draft | in-review | approved | shipped | abandoned | superseded
resource: https://github.com/org/repo/issues/42 # optional: the tracker item
owner: # optional; PRD extension, mirrors `deciders`
  - alice
target_release: 2026-09-01 # optional; PRD extension
decisions: # optional; concept ids of the ADRs that shaped this PRD
  - adopt-okf
  - defer-to-unix-tools
---

# Bulk ADR Import

## Status

Draft

## Problem

Who has it, why it matters, and why now.

## Approach

The shape of the solution, in a paragraph.

## Requirements

What the software must do. The primary section.

## Non-Goals

What this explicitly does not cover.

## Success Metrics

How we will know it worked.

## Open Questions
```

The H1 must equal `title`, as for ADRs.

**A PRD borrows the ADR's section vocabulary only where the two documents genuinely agree.** `Status` is shared: it is a lifecycle marker rather than decision-specific language, both types carry the same fact in frontmatter, and mirroring it in the body is a convention arkouda already applies uniformly. Everything else is PRD-native. In particular there is no `## Context` — that is a decision record's word for what every product template surveyed calls the *Problem*, and borrowing it would make the ADR vocabulary look like arkouda's universal one, which is the assumption this ADR exists to remove.

Five required sections: the lifecycle (`Status`), the motivation (`Problem`), the substance (`Requirements`), the boundary (`Non-Goals`), and the falsifiable claim about the outcome (`Success Metrics`). `Approach` and `Open Questions` are scaffolded by `arkouda new` but not validated — the same treatment arkouda's own ADRs give `Alternatives Considered` and `Citations`. Requiring them would make the cheapest useful PRD expensive to write; scaffolding them makes writing them the default.

`Non-Goals` is required rather than scaffolded because it is the section most consistently skipped and most expensive to omit, and because a validator can enforce a heading but not a prompt. Shape Up gives it a named ingredient (*No-Gos*) and Atlassian's blueprint closes on *Out of Scope*; Figma's template is the dissent, folding the question into instructional prose under *The Problem* and *Key Features*. Prose is exactly what `arkouda check` cannot see.

### Linking a PRD to the decisions that shaped it

`decisions` is a frontmatter list of ADR concept ids, not a body section. It is the edge that makes both types in one tool pay off: the requirement and the decision serving it are one lookup apart, and neither restates the other.

Frontmatter rather than a `## Decisions` section, for three reasons:

- **Concept ids survive reconfiguration.** A body section links by relative path (`../adr/adopt-okf.md`), which bakes the directory layout into every PRD — in the same change that makes the layout configurable per type. Point `prd` at `docs/product/prd` and every such link breaks. A concept id is bundle-relative and layout-independent.
- **It is checkable.** A list of ids can be resolved against the loaded collection; Markdown prose cannot. Dangling cross-references are the characteristic rot of linked documents — an ADR gets superseded and the PRD keeps pointing at it, silently.
- **It matches an existing pattern.** `superseded_by` is already a frontmatter concept-id reference. `decisions` is the same idea with a cardinality of many, so the schema grows a second instance of a shape it has rather than a new shape.

Losing the section loses the prose about _why_ a given decision matters here, which is what Figma's *Open Issues & Key Decisions* section is for. In arkouda that prose has a home already: it is the ADR. Coda has no document type for "the discussion happened and here are the tradeoffs", so Figma's template has to carry it inline. Arkouda's whole premise is that it does have one, and a PRD only needs to point at it.

Backlinks — which PRDs depend on this ADR — are derivable from the same list and are not stored on the ADR side. `decisions` is deliberately one-directional, so there is one place to update when a link changes.

Arkouda validates document *structure*, not requirement *content*. It will not enforce `FR-###`/`SC-###` numbering, `P1`/`P2` priorities, or [EARS](https://alistairmavin.com/ears/) acceptance-criteria syntax (`WHEN <event> THE SYSTEM SHALL <behavior>`) the way spec-driven tools like GitHub's spec-kit and Kiro do. Those are worth adopting inside a `## Requirements` section and are out of scope for a validator whose contract is headings and frontmatter.

`owner`, `target_release`, and `decisions` are new optional producer extensions (OKF §4.1); `owner` parallels `deciders`, and `target_release` is the one scheduling field Atlassian's blueprint treats as first-class. `resource` already exists in the frontmatter struct and is where a PRD's tracker link goes. `superseded_by` works unchanged for both types, and starts being validated (see below).

The status vocabulary deliberately blends document state (`draft`, `in-review`, `approved`) with delivery state (`shipped`, `abandoned`), which Atlassian's blueprint splits across two fields. One field is the right call here: arkouda groups `index.md` by status and sorts by it, and two lifecycle axes would need two groupings. The ADR vocabulary already blends the same way — `proposed` and `accepted` describe the document, `deprecated` describes the world.

### CLI surface

Three additions, no new subcommands:

- **`arkouda new "<title>" [--type adr|prd]`** — defaults to `adr`. Selects the template, the type string, the status vocabulary, and the target directory.
- **`arkouda list [--type adr|prd]`** — filters. The `-l` columns stay exactly as they are: `ID STATUS TIMESTAMP PATH TITLE — DESCRIPTION`. Inserting a type column would break every documented `awk '$2=="accepted"'` pipeline, and the path column already discriminates under the default per-type directories.
- **`arkouda decision <id>`** — prints the concept's _primary section_: `## Decision` for an ADR, `## Requirements` for a PRD. `--section <name>` is unchanged and already type-neutral.

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

`arkouda check` validates every concept against the descriptor its `type` names. Three existing codes change meaning to be type-relative:

- `E005` — `type` is not one of the types arkouda knows (was: `type` is not `Architecture Decision Record`).
- `E003` — `status` is not in _this type's_ vocabulary. The hint lists that type's values.
- `E009` — a section required by _this type_ is missing.

A concept with an unknown `type` is an `E005` error rather than a skipped file. Arkouda manages the bundles it is pointed at; silently ignoring a document it cannot check would hide exactly the drift `check` exists to catch.

One code is added. **`E015` — a frontmatter concept reference does not resolve to a loaded concept.** It covers `decisions` and, for the first time, `superseded_by`: that key has been parsed but never validated since it was introduced, so a supersede pointing at a renamed or deleted ADR is silent today.

`E015` is a **warning**, not an error, because arkouda cannot distinguish a broken reference from an out-of-scope one. `arkouda check --dir docs/prd` loads one bundle, and every `decisions` entry pointing into `docs/adr` is then legitimately unresolvable. Failing there would make `check` depend on which directories the invocation happened to cover. This is the same reasoning that made `E013` and `E014` warnings — OKF §9 permissive consumption — and it keeps the property that `check` never fails a bundle for something outside it.

### `index.md`

Group by type, then by status: `# <Type>` at H1, `## <Status>` at H2, entries beneath. Today's index groups by status at H1, so every existing bundle's index changes shape once and must be regenerated with `arkouda index`. Until then `E014` (stale index) fires, which is a warning and never fails a bundle — the OKF §9 permissive-consumption behaviour arkouda already implements.

Uniform nesting beats conditional nesting. A single-type bundle pays one extra heading level; consumers parse one shape instead of two.

## Consequences

### Positive

- An agent can answer "what are we building" and "why is it built this way" from one CLI, one config file, one diagnostic vocabulary, and one skill — and `arkouda list | xargs rg -i <topic>` searches both at once.
- The hardcoded type, status list, section list, and template collapse into one descriptor, so a third type (RFC, runbook, postmortem) becomes a data change rather than a refactor — and user-supplied templates become a config parser over a shape that already exists.
- PRDs get the property that made ADRs worth tooling: a strict schema with machine-readable diagnostics, so CI can gate on a PRD being well-formed and an agent gets an actionable error instead of a shrug.
- `decisions` makes the PRD-to-ADR edge typed data rather than prose, so traversing from a requirement to its rationale is a frontmatter read rather than a Markdown-link parse — and `superseded_by` finally gets validated, closing a hole that predates this change.
- Arkouda becomes closer to what OKF describes — a consumer of typed concepts — rather than a tool that reads OKF bundles but only believes in one type.

### Negative

- `--status` loses its static completion values, because valid statuses now depend on `--type`. Shell completions will offer nothing for it until completions learn to be type-aware.
- `arkouda decision` is a misleading name when the concept is a PRD. The alternative was a subcommand per type, which does not scale.
- Every existing `index.md` regenerates with a new heading structure — a one-line diff of churn per bundle, and a stale-index warning until someone runs `arkouda index`.
- Two vocabularies means `arkouda list --sort status` orders within a type but interleaves across types. Sorting a mixed collection by status is now only meaningful with `--type`.
- Apart from `Status`, the two types share no section headings, so `arkouda decision <id> --section <name>` takes a different set of names depending on what the concept is. An agent that guesses `--section context` on a PRD gets a `SectionNotFound` error rather than a near-miss. That is the intended failure — the alternative is a shared vocabulary that implies the documents answer the same question — but it does mean `--section` cannot be scripted across a mixed collection without branching on type, `--section status` excepted.
- The module named `adr` becomes the module named `concept`, and `AdrStatus`, `ADR_TYPE`, and `ADR_DIR` are all named after one of two types. The env var stays `ADR_DIR` for compatibility; the internals get renamed.
- **Prerequisite:** section handling must learn about fenced code blocks first. `check_required_sections` and `Manifest::section` both scan raw lines for a `## ` prefix, so a heading inside a ` ```markdown ` fence counts as a real section — an ADR carrying a template in a fence passes `check` without having the sections it appears to declare, and `decision` truncates its output at the fence. This ADR's own body demonstrates both: `arkouda decision support-product-requirements-documents` stops at the PRD template's first heading. Two built-in types make this acute, because documenting a type means showing its headings.

### Neutral

- One new diagnostic code (`E015`), and it is a warning, so the `E000`–`E014` contract that CI and agents key on keeps its shape and nothing that passes today starts failing.
- Discovery, concept ids, reserved filenames, id slug rules, and duplicate-id detection are untouched — they were already type-agnostic.
- Arkouda's own `docs/adr/` bundle is unaffected until it grows a PRD. Existing ADRs validate as-is.
- `skills/use-arkouda/SKILL.md` and the README both widen: the skill must teach when to write a PRD versus an ADR, or an agent will keep reaching for `arkouda new` with the default type.

## Alternatives Considered

### Ship a separate `prd` binary

Cleanest separation, zero risk to the ADR path. It also duplicates discovery, config, frontmatter parsing, index generation, and the diagnostic vocabulary — and forces an agent to learn two tools to answer one question. The shared machinery is most of the code; the type-specific part is a template and two lists.

### Make concept types user-definable now

Let a project declare any `type` in `.arkoudarc.toml` with its own sections, statuses, and template. **Deferred rather than rejected** — this is the likely next step, and `ConceptType` is deliberately shaped to be what a user-supplied template deserializes into, so adding it later is a parser and a config key rather than a redesign.

Not now, for two reasons. Arkouda's value is that the schema is _known_: an agent that has read the skill knows what a PRD looks like without reading a project's config, and that property is worth keeping until there is a concrete second consumer. And the questions a bring-your-own-template feature has to answer — where templates live, whether they are shareable across repos, what happens to a concept whose type is no longer declared, whether `check` should fail or skip an unknown type — are answered better against two real built-in types than against zero. Shipping ADR and PRD first is what makes the general version designable.

### Treat a PRD as an ADR with `tags: [prd]`

Zero code change. It also means `arkouda check` validates a requirements document against Nygard's sections, `arkouda new` scaffolds the wrong template, `status: accepted` has to stand in for `shipped`, and `type` — the one field OKF requires — lies to every other OKF consumer. The whole point of a strict schema is lost when one type is used to smuggle another.

### Add a type column to `list -l`

More informative in a mixed bundle, and it would break `awk '$2=="accepted"'` — a pipeline shape the README, the skill, and any agent that has read either depend on. `--type` filtering gives the same information without moving a column.

### Keep `index.md` flat when a bundle holds one type

Byte-identical output for every existing bundle and no regeneration churn. It also gives consumers two shapes to parse for the same file, and the churn is one command run once.

### Give the PRD the whole ADR section vocabulary

Reusing `Context` alongside `Status` would let more `--section` names work across both types and would keep the templates visibly related. It would also assert something false: a PRD's problem statement is not a decision record's context, and the templates surveyed agree — Figma, Lenny's, and Shape Up all call it *Problem*. `Status` is the one word the two documents genuinely share, because it names a lifecycle rather than a kind of content.

### Adopt Figma's phase grouping

[Figma's PRD template](https://coda.io/@yuhki/figmas-approach-to-product-requirement-docs) groups sections under three H1s — *Problem Alignment*, *Solution Alignment*, *Launch Readiness* — so the document's shape is a sequence of agreements rather than a bag of sections. It is the best structural idea in any template surveyed: you align on the problem before a solution exists.

Two things rule it out. Mechanically, `Manifest::section` stops only at the next `## ` heading, so an intervening H1 is swallowed into the preceding section's body — `--section "goals & success"` would return `# Solution Alignment` as part of its answer. Adopting the shape means changing section extraction for every type.

More importantly, the phase sequence is what `status` already encodes. Figma expresses the lifecycle structurally because a Coda document has no typed status field; arkouda has one. Storing draft-review-approved in both the frontmatter and the heading tree is the redundancy [adopting OKF](adopt-okf.md) removed when it deleted `id`.

Figma's *Launch Checklist* is likewise left out: its rows are Figma's team topology (Support, Growth, PMM, Enterprise, Platform, Security), not anything general about products.

## Citations

[1] [Open Knowledge Format v0.1 specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) — §4.1 producer extensions, §6 index, §9 permissive consumption
[2] [Adopt the Open Knowledge Format](adopt-okf.md) — the migration that made a second concept type possible
[3] [ls-style list and a decision subcommand](ls-style-list-and-decision.md) — the primary-section CLI contract this generalizes
[4] [Defer to Unix tools](defer-to-unix-tools.md) — why `--type` is a filter rather than a new subcommand
[5] [Michael Nygard's ADR template](https://github.com/joelparkerhenderson/architecture-decision-record/tree/main/locales/en/templates/decision-record-template-by-michael-nygard)

### PRD templates surveyed

The PRD section set above is drawn from these. `Problem`, `Requirements`, and `Success Metrics` appear in essentially all of them; `Non-Goals` appears in all but one.

[6] [GitHub spec-kit feature specification template](https://github.com/github/spec-kit/blob/main/templates/spec-template.md) — the closest comparable: agent-native, plain Markdown, versioned in-repo. Mandates User Scenarios, Requirements, and Success Criteria; marks unresolved points `[NEEDS CLARIFICATION]`
[7] [Kiro spec-driven development](https://kiro.dev/docs/specs/) — `requirements.md` / `design.md` / `tasks.md`, with acceptance criteria in [EARS](https://alistairmavin.com/ears/) notation
[8] [Atlassian Product Requirements blueprint](https://confluence.atlassian.com/doc/product-requirements-blueprint-329975392.html) and [the accompanying guide](https://www.atlassian.com/blog/development/write-product-requirements-confluence) — source of `target_release` and `owner` as first-class properties, and of the document-status/delivery-status split this ADR deliberately collapses
[9] [Figma's approach to PRDs](https://coda.io/@yuhki/figmas-approach-to-product-requirement-docs) — Problem Alignment / Solution Alignment / Launch Readiness; source of `Approach` and of `Decisions`, and the dissenting view on `Non-Goals`
[10] [Shape Up, ch. 6: Write the Pitch](https://basecamp.com/shapeup/1.5-chapter-06) — Problem, Appetite, Solution, Rabbit Holes, No-Gos
[11] [Lenny Rachitsky's product requirements template](https://www.atlassian.com/software/confluence/templates/lennys-product-requirements) — Description, Problem, Why, Success, Audience, What
[12] [Amazon's Working Backwards PR/FAQ](https://workingbackwards.com/resources/working-backwards-pr-faq/) — the press-release-first alternative shape, not adopted
