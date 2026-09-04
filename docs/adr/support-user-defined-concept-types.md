---
type: Architecture Decision Record
title: Support user-defined concept types
description: Let a project declare its own concept types in .arkoudarc.toml, and tier check's rules so template conformance applies only where a template defines it.
tags:
  - types
  - schema
  - config
  - validation
  - okf
timestamp: 2026-09-04
status: proposed
deciders: []
---

# Support user-defined concept types

## Status

Proposed

## Context

[Supporting Product Requirements Documents](support-product-requirements-documents.md) generalized arkouda from an ADR-only tool into a concept-type-aware one, and closed with `ALL: &[ADR, PRD]` — two built-in types, and a note that types are not user-definable. That ADR deferred the general version rather than rejecting it, and named the four questions it would have to answer: where templates live, whether they are shareable across repos, what happens to a concept whose type is no longer declared, and whether `check` should fail or skip an unknown type. Its argument for deferring was that those questions are answered better against two real built-in types than against zero.

Two things have changed.

The first is scope. Arkouda is positioned as an OKF tool, and OKF's whole point is that `type` is a per-concept frontmatter field so that a bundle may hold any mix. Arkouda currently implements the opposite: a bundle may hold any mix drawn from a set of two. A valid OKF bundle containing a third type — a runbook, an RFC, an incident review, a data-product spec — fails `arkouda check` outright with `E005`, because `src/concept/validator.rs` treats an unrecognized `type` as an error rather than as a document it has no contract for. The tool rejects bundles that are conformant to the format it advertises support for.

The second is that the deferral's precondition has been met. ADR and PRD exist, they differ in every field of the descriptor, and the descriptor has held up: `ConceptType` already carries `okf_type`, `statuses`, `required_sections`, `primary_section`, `default_dir`, `template_sections`, and `template_extensions`, and it already separates _scaffolded_ from _enforced_ — a PRD's `Approach` and `Open Questions` are written by `arkouda new` and never checked by `arkouda check`. That separation is the hard part of a bring-your-own-template design, and it is already built and already exercised by two types.

### What the telemetry says, and what it does not

Local invocation telemetry ([`telemetry-for-agent-command-invocations`](telemetry-for-agent-command-invocations.md)) has accumulated 59 events over 10 active days, 2026-05-21 to 2026-08-31:

| command                    | invocations | non-zero exit |
| -------------------------- | ----------- | ------------- |
| `check`                    | 30          | 6             |
| `list`                     | 13          | 0             |
| `index`                    | 10          | 0             |
| `new`                      | 5           | 0             |
| `decision` (now `section`) | 1           | 0             |

`check` is the most-used command by a wide margin, and the failures are not noise: they form two repair loops on 2026-07-12 — four consecutive failures over 84 seconds converging to a pass, then a single failure resolved 21 seconds later — which is exactly the write/reject/fix/pass cycle validation exists to produce. Every event has `tty: false` and 57 of 59 carry `agent: claude-code`; in practice this is an entirely agent-driven tool.

**None of that adjudicates template linting specifically, and it must not be cited as if it did.** Every recorded event came from arkouda 0.3.0–0.5.0, all of which were ADR-only: there was exactly one template and no `--type` flag, so no agent ever chose a type or failed to. The event schema records `exit_code` but not which diagnostics fired, so the six failures cannot be attributed to the template tier (`E003`, `E009`) rather than to the OKF and frontmatter tiers (`E001`, `E004`, `E006`, `E007`). The data supports "validation is the most-used feature and it catches real errors". It says nothing about whether the _per-type contract_ is well calibrated, and it comes from one operator on one machine across roughly two projects.

So this decision is made on the design argument, not on usage evidence — and it comes with an instrumentation change so that the next such decision does not have to be.

### What blocks it in code

The descriptor's own doc comment claims that user-definable types are "a config parser over this struct rather than a redesign". That is true of the model and false of the plumbing. Four obstacles are real work:

- **`&'static` runs through everything.** Every `ConceptType` field is `&'static str` or `&'static [T]`, `ALL` is a `static`, and `by_slug`/`by_okf_type` return `&'static ConceptType`. Config-loaded types are runtime values.
- **`Dirs` depends on the registry.** `Dirs::shared` and `Dirs::defaults` iterate `types::ALL`, and `Dirs` keys its map by `&'static ConceptType`. Today config depends on a compile-time registry; afterwards the registry comes from config, inverting the load order.
- **clap validates `--type` before any config is read.** `ListArgs` and `NewArgs` use `PossibleValuesParser::new(types::slugs())`, and `main.rs` calls `Cli::parse()` before touching the filesystem. The valid slug set will depend on the working directory.
- **`E005` is a hard error by deliberate argument.** The existing rationale — "silently ignoring a document it cannot check would hide exactly the drift `check` exists to catch" — is sound for a closed registry and unsound for an open one, where an unknown type is the ordinary case rather than a mistake.

## Decision

Make concept types user-definable, declared in `.arkoudarc.toml`, and split `arkouda check` into three tiers so that template conformance is enforced only where a template defines it.

### Types are declared in `.arkoudarc.toml`

A project adds `[[types]]` tables to the config file it already has:

```toml
[[types]]
slug = "rfc"
okf_type = "Request for Comments"
statuses = ["draft", "active", "withdrawn"]
required_sections = ["Status", "Summary", "Motivation"]
primary_section = "Summary"
default_dir = "docs/rfc"
template = "docs/templates/rfc.md"   # optional
extensions = ["sponsors"]            # optional
```

The keys are the `ConceptType` fields, so the table deserializes into the descriptor with no intermediate model. This answers _where templates live_: in the repo, next to the bundles they describe, discovered by the same walk-up that already finds `.arkoudarc.toml`, with relative paths resolved against the config file's directory exactly as `dirs` entries are.

Sharing type definitions across repositories — a package, a registry, an `extends` key — is **out of scope**. It is the same deferral for the same reason: the questions it raises (versioning, trust, offline resolution) are answerable against real in-repo types and not before. Copying a `[[types]]` block between two repos is an acceptable interim answer.

Config-level mistakes are `ArkoudaError::Config`, reported with the config file's path, not concept diagnostics — they are errors in the ruleset rather than in a document. Duplicate `slug` or `okf_type`, an empty `statuses`, a `primary_section` absent from `required_sections`, and a `template` that omits a section the type requires are all rejected at load. The last two are already enforced against the built-ins by unit tests in `src/concept/types.rs`; they become runtime checks for declared types.

Statuses are declared as a plain list in lifecycle order. The title-case label used for `index.md` headings and the body's `## Status` section is derived by splitting on `-` and capitalizing (`in-review` → `In Review`), which reproduces every built-in label without a longer TOML form.

### Built-in types remain, and may be shadowed

ADR and PRD stay registered by default. A `[[types]]` block whose `slug` or `okf_type` matches a built-in **replaces it wholesale** rather than merging with it. That keeps the common case — "I want my own types _as well_" — zero-configuration, while giving a project that disagrees with Nygard's four sections a way to say so without a merge semantics nobody wants to reason about.

Partial override is deliberately not offered. A type is a contract; a contract assembled from a built-in plus three patches is harder for an agent to state than one written out.

### A template file supplies the scaffold body

`template` points at a Markdown file that `arkouda new` renders. Frontmatter in that file is ignored — `new` generates frontmatter from the descriptor, the title, and the date, as it does today. Its `##` headings and their prose become the scaffolded body, playing the role `template_sections` plays for the built-ins.

The scaffold and the contract stay separate keys. `required_sections` is what `check` enforces; the template is what `new` writes; the template must be a superset. This preserves the property PRD already relies on, where `Approach` and `Open Questions` are prompted for without being worth failing a bundle over. A type with no `template` scaffolds its `required_sections` with a `TODO:` line each, which is a usable default.

### `check` validates in three tiers

| Tier              | Codes                                                                          | Applies to                                          |
| ----------------- | ------------------------------------------------------------------------------ | --------------------------------------------------- |
| OKF conformance   | `E000`, `E004`, `E007`, `E010`, `E011`, `E012`                                 | every concept, always                               |
| Arkouda profile   | `E001`, `E002` (`title`, `description`, `status`, `timestamp`), `E006`, `E008` | concepts whose `type` resolves to a declared type   |
| Template contract | `E003` (status vocabulary), `E009` (required sections)                         | types that declare `statuses` / `required_sections` |

No diagnostic codes are added or renumbered; the change is which tier each already belongs to, made explicit. The tiers are what let arkouda be strict about the format it implements and permissive about the contracts a project has chosen not to write down.

### An unknown type degrades rather than failing or skipping

This answers the two remaining deferred questions together, and answers them with neither of the offered options.

A concept whose `type` matches no declared type is validated at the **OKF tier only**, and `E005` fires as a **warning** rather than an error. It is not skipped — its id, headings, and bundle placement are still checked, and it still appears in `list` and `index` — and it does not fail the run.

Demoting `E005` reverses the position taken in [`support-product-requirements-documents`](support-product-requirements-documents.md). That position was correct under a closed registry, where an unrecognized `type` could only be a typo or an unmigrated file. Under an open registry it is ordinarily a type the operator simply has not declared, and failing on it makes `arkouda check` reject conformant OKF bundles — the precise behaviour that motivates this ADR. The drift argument is preserved by the warning: arkouda still says, on every run, that it has no contract for this document.

This also gives _a concept whose type is no longer declared_ a sane answer. Deleting a `[[types]]` block turns its documents into unknown-type concepts: warned about, still listed, still OKF-checked, never silently dropped and never a broken build.

`E013` and `E014` are already warnings on the same OKF §9 permissive-consumption reasoning, so this is the established shape rather than a new one.

### The registry is resolved before `--type` is validated

`--type` loses its `PossibleValuesParser` and is parsed as a free `String`. After the config is loaded and the registry built, the value is resolved against it, and an unresolvable one is an error naming the slugs actually configured for this project. Startup order becomes: parse argv → discover config → build registry → resolve `dirs` → validate `--type` → run.

`new --type` keeps `adr` as its default when the built-in is present, and requires an explicit `--type` when a project has shadowed or omitted it.

The parsed registry is `Box::leak`ed once at startup so the `&'static ConceptType` signatures threaded through `Dirs`, `list`, `check`, `new`, `index`, and `section` survive unchanged. A single process-lifetime allocation in a short-lived CLI is the cheaper trade against converting the descriptor to owned data and plumbing a `&Registry` through every command. Shell completions for `--status` and `--type` remain unable to offer values, as they already are since `--status` stopped being a `ValueEnum`.

### Telemetry records which diagnostics fired

Add a `codes` field to the invocation event: the sorted set of diagnostic codes a `check` run produced, e.g. `"codes": ["E003", "E009"]`. The event schema is declared additive, so this is a compatible change.

This is the field that would have answered the question that prompted this ADR. Without it, "is the template contract too strict" is unanswerable from six recorded failures that could have been anything.

Also record whether the resolved type was built-in or user-declared — `"type_kind": "builtin" | "custom"` — and **not** the slug. A user-declared slug is project-specific free text of exactly the kind the telemetry ADR redacts to a marker; the distribution of built-in versus custom is the signal, and the name is not.

## Consequences

### Positive

- Arkouda accepts any conformant OKF bundle. The tool's behaviour matches its positioning, and a project can adopt it for a knowledge base that has nothing to do with decisions or requirements.
- The strict-schema property survives where it earns its keep. A declared type is checked exactly as strictly as ADR and PRD are today; only undeclared types relax, and they relax to the OKF floor rather than to nothing.
- Deleting or renaming a type degrades gracefully instead of breaking a build.
- A project that disagrees with the built-in ADR or PRD contract can change it in config rather than forking or abandoning `check`.
- The next iteration of this design will have evidence behind it, because `codes` will say which rules actually fire.

### Negative

- **The schema is no longer knowable from the skill alone.** This is the real cost, and it is the objection [`support-product-requirements-documents`](support-product-requirements-documents.md) raised when deferring. An agent that has read `use-arkouda` currently knows what an ADR and a PRD look like without opening a project's config; afterwards it must read `.arkoudarc.toml` to know what a project's types are. The skill will need a step that reads the configured types before scaffolding, and `arkouda new` for an unknown slug must error with the configured list rather than with `adr|prd`.
- A misconfigured `[[types]]` block is a new failure mode, and it fails at load, so it takes down every command rather than just `check`.
- Shadowing means two projects can both have a type called `adr` that means different things. That is inherent to the feature and is why shadowing is wholesale rather than partial.
- `Box::leak` is a deliberate leak. It is bounded and process-lifetime, but it will look wrong to a reader who does not find this paragraph.
- Demoting `E005` means a genuine typo in `type` — `Architecture Decision Recrod` — now warns instead of failing. A project that wants the old strictness has no way to ask for it. If that turns out to matter, a `strict_types = true` config key is the obvious follow-up, and the `codes` telemetry will show whether `E005` warnings are being ignored in practice.

### Neutral

- No diagnostic codes are added, removed, or renumbered. `E005` changes severity; the rest change only in which tier they are documented under.
- `.arkoudarc.toml` gains a key and keeps both existing `dirs` forms. A config file with no `[[types]]` behaves exactly as it does today.
- Cross-repo sharing of type definitions stays deferred, for the second time and for the same reason.

## Alternatives Considered

### Derive `required_sections` from the template file

Let the template be the single source of truth: whatever `##` headings it contains are what `check` requires. Tempting — one artefact, no way for the scaffold and the contract to drift apart.

Rejected because it collapses the scaffold/enforce distinction that both built-in types already use. A PRD scaffolds `Approach` and `Open Questions` precisely so an author is prompted for them, and requires neither, because a requirements document with no open questions is finished rather than broken. Deriving the contract from the template makes every prompt mandatory, which makes templates worse by making them shorter.

### Keep the registry closed and add built-in types on request

Ship RFC, runbook, and incident review as built-ins three and four and five. Preserves the knowable-schema property completely.

Rejected as unbounded. The set of document types a team keeps in a repo is not enumerable by this project, and each addition is a release, a skill update, and a schema an agent has to learn whether or not any given project uses it. The knowable-schema property is worth something, but not worth arkouda maintaining a taxonomy of everything.

### Keep `E005` a hard error and require every type to be declared

Preserves today's strictness: point arkouda at a bundle and every document in it must be one arkouda has a contract for.

Rejected because it makes adoption all-or-nothing. A project with 400 concepts across nine types would have to write nine `[[types]]` blocks before `arkouda check` exits 0 once, and would get no value from the tool until it did. Degrading to the OKF tier means arkouda is useful on day one and gets stricter as types are declared, which is the same progressive path `dirs` already offers.

### Express the contract as JSON Schema over frontmatter

A standard, expressive schema language with existing tooling, rather than a bespoke TOML table.

Rejected on fit. Half of what a concept type constrains is Markdown body structure — required `##` sections, the primary section — which is not frontmatter and which JSON Schema cannot express. The result would be a JSON Schema for part of the contract plus a bespoke config for the rest, which is worse than one bespoke config. It also imports a dependency and a second file format into a tool whose config is nine lines of TOML.

### Ship the tiering without user-defined types

Demote `E005`, split the tiers, and stop there. Strictly smaller, and it alone fixes the "rejects valid OKF bundles" problem.

Rejected as half a feature, though it is the natural fallback if the config parser proves larger than expected. It leaves a project able to _store_ other types but not to get any contract enforced on them, which turns arkouda into a linter that ignores most of the bundle. The tiering is what makes user-defined types safe; user-defined types are what make the tiering worth having.

## Citations

[1] [Open Knowledge Format v0.1 specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/ee67a5ca27044ebe7c38385f5b6cffc2305a9c1a/okf/SPEC.md) — §2 concept ids, §3.1 reserved files, §9 permissive consumption, §11 version declaration. Pinned to the commit arkouda vendors at [`docs/okf/SPEC.md`](../okf/SPEC.md).
[2] [Support Product Requirements Documents](support-product-requirements-documents.md) — the ADR that introduced `ConceptType`, deferred this decision while naming the four questions it answers, and took the `E005`-as-error position this ADR reverses.
[3] [Adopt the Open Knowledge Format](adopt-okf.md) — the migration that made concept types possible at all, and the source of the permissive-consumption reasoning the `E005` demotion follows.
[4] [Telemetry for agent command invocations](telemetry-for-agent-command-invocations.md) — the event schema this ADR extends with `codes` and `type_kind`, and its additive-schema and no-free-text commitments.
[5] [Defer to Unix tools](defer-to-unix-tools.md) — the standing constraint that arkouda adds surface only where a shell pipeline cannot do the job; a per-project schema is not something `rg` can enforce.
[6] Local telemetry log, `~/Library/Application Support/arkouda/telemetry.jsonl` — 59 events, 2026-05-21 to 2026-08-31, all from ADR-only builds 0.3.0–0.5.0. The evidence base for the usage figures in Context, and the reason those figures cannot settle the question.
