---
type: Architecture Decision Record
title: Adopt OKF 0.2
description: 'Move arkouda from OKF v0.1 to v0.2: generated.at supersedes timestamp, the provenance families are parsed, status adopts OKF''s vocabulary, and arkouda''s per-type values move to a new lifecycle key.'
tags: []
generated:
  by: arkouda/0.6.0
  at: 2026-09-04T11:09:40Z
status: stable
lifecycle: accepted
deciders: []
---

# Adopt OKF 0.2

## Status

Accepted

## Context

Arkouda [adopted OKF](adopt-okf.md) at v0.1 and vendors the spec text it was built against at [`docs/okf/SPEC.md`](../okf/SPEC.md). Upstream released **v0.2** in July 2026 and tightened it again in August; v0.1 is superseded. Two ADRs already carry a note that upstream has moved on and arkouda has not.

v0.2 is a minor bump under its own §12 — the bundle structure, reserved filenames, the required `type`, the recommended `title`/`description`/`resource`/`tags`, cross-linking, index files, log files, and permissive conformance all carry forward unchanged. The conformance floor (§11) is word-for-word the v0.1 floor plus rules that bind only consumers of the new families. So this is not a rewrite. It is four decisions.

### What actually changed

§13 of the spec enumerates it. Two breaking changes:

- **`timestamp` is superseded by `generated: { by, at }`** (§5.2, §13.1). A concept's last meaningful change now lives in a provenance mapping alongside the actor that produced it. Consumers MAY fall back to a legacy `timestamp`.
- **The body `# Citations` list is superseded by `sources`** (§5.1, §13.1). Provenance moves into frontmatter, with per-claim attribution by markdown footnote keyed to a `sources[].id`.

And the additive families: `sources` with credibility signals, `generated`, `verified`, `status`, `stale_after` (§5); the actor convention for `generated.by` and `verified[].by` (§7); a new `Attested Computation` concept type with its own contract fields (§10); and a new conventional `# Computation` heading.

A later upstream commit — [`62432a0`](https://github.com/GoogleCloudPlatform/knowledge-catalog/commit/62432a095456), *"make every timestamp an ISO 8601 datetime with an explicit offset"* — tightened every instant in the spec. v0.2's examples are uniformly `2026-06-20T22:53:05Z`; a bare date is a v0.1 shape.

### The one real collision

**OKF v0.2 §5.4 defines `status`.** It was a producer extension in v0.1, which is how arkouda uses it — a per-type lifecycle, `proposed`/`accepted`/`superseded`/`deprecated`/`rejected` for an ADR, `draft`/`in-review`/`approved`/`shipped`/`abandoned`/`superseded` for a PRD, and whatever a project declares for its own types. v0.2 gives the same key a fixed three-value vocabulary — `draft | stable | deprecated` — with `stable` as the default when absent.

The key is the same, the meanings overlap but do not match, and `status` is load-bearing in arkouda: it is a column in `list -l`, the second level of every `index.md`, a `--sort` field, the thing `E003` validates, and a required field in arkouda's profile. Every document in every existing bundle carries one. Whatever is decided here, it has to keep those bundles readable.

## Decision

Move to OKF v0.2, implementing the two breaking changes and parsing the new families, and keep arkouda's `status` as a refinement of OKF's rather than adopting OKF's vocabulary.

### `generated.at` supersedes `timestamp`, and the legacy key keeps working

`arkouda new` scaffolds `generated: { by: arkouda/<version>, at: <now> }` in the §7 actor form, because arkouda is what wrote those bytes; whoever fills the template in should replace it with their own `human:<id>`.

Reading is a fallback chain: `generated.at` when present, then a legacy `timestamp`. The spec permits exactly this, and it means **every v0.1 document keeps validating, sorting, and displaying unchanged**. Arkouda's profile requires *a* content timestamp, not a particular spelling of one — `E001` fires only when neither is there, and `E002` when `generated` exists but carries no `at`.

A document dated only the v0.1 way gets **`E017`**, a new warning. It never fails a bundle, and it is the only prompt to migrate a key that still works, so it is worth the noise on an upgrade. If that noise turns out to outweigh the prompt, an `arkouda migrate` that rewrites the key is the obvious follow-up; this ADR does not add one, because a command that rewrites every document in a bundle deserves its own decision.

### v0.2 instants carry an explicit offset; the retired key stays lenient

Every instant v0.2 defines on a concept must be an ISO 8601 datetime with an offset (`E006`): `generated.at`, each `verified[].at`, `stale_after`, each `sources[].last_modified`, and both endpoints of a `usage_window`, shared or per-source. The list is exhaustive on purpose — upstream tightened all of them in one commit, and validating some but not others would let a date-only `last_modified` through while rejecting the same value in `generated.at`.

A legacy `timestamp` continues to accept a plain date, a local datetime, or an offset datetime. Tightening a key the spec has already retired would break documents that are still perfectly readable, and would punish exactly the users who have not migrated yet.

### The provenance, trust, and lifecycle families are parsed, not merely tolerated

`sources` (with `id`, `resource`, `title`, `author`, `usage_count`, `last_modified`), the `usage_window` sibling, `generated`, `verified`, and `stale_after` all become typed frontmatter fields. Arkouda already tolerated unknown keys — §11 requires that — but tolerating is not understanding, and a tool that claims v0.2 should be able to read what v0.2 says.

`verified` accepts either a list or a single bare `{ by, at }` mapping, which §11 makes a **MUST** for consumers.

`stale_after` earns the second new warning, **`E016`**: a concept past its instant is stale (§5.5). This is the one v0.2 lifecycle field with an obvious meaning for a decisions-and-requirements tool — a decision with a review date that has passed is exactly the thing an agent should be told about before relying on it.

Trust tiers (§5.3) are derivable from `verified` but are not surfaced anywhere yet; there is no command whose output they would change. The data is parsed and available when there is.

### `status` adopts OKF's vocabulary; the per-type values move to `lifecycle`

`status` means what §5.4 says it means: `draft | stable | deprecated`, absent reading as `stable`. Arkouda's per-type vocabularies move to a new key, **`lifecycle`**:

```yaml
status: stable          # OKF §5.4 — what any consumer reads
lifecycle: accepted     # arkouda — what a reader of decisions wants
```

The two are not independent axes. **`status` is the projection of `lifecycle`**, and every concept type declares which of OKF's three values each of its lifecycle values lands on. `arkouda new` writes both from one choice, and `check` reports a pair that disagrees (`E019`) — a `rejected` ADR advertised as `stable` is precisely the fidelity loss adopting the vocabulary was meant to fix.

The built-in projections:

| | `draft` | `stable` | `deprecated` |
| --- | --- | --- | --- |
| ADR | `proposed` | `accepted` | `superseded`, `deprecated`, `rejected` |
| PRD | `draft`, `in-review` | `approved`, `shipped` | `abandoned`, `superseded` |

Both collapse the same way: a not-yet-final state, a live one, and the several ways a document stops being current. That the fine distinctions collapse is the reason `lifecycle` exists — `superseded` and `rejected` are the same to a generic consumer and are not remotely the same to someone reading decisions.

A declared type maps its own vocabulary with an optional `okf_status` table. An unmapped status projects onto `stable`, or onto itself when it is spelled exactly like one of OKF's three. Arkouda does not guess that an unfamiliar status means `deprecated`: writing `status: deprecated` into a document nobody retired would be a worse lie than the coarseness it is trying to avoid.

Nothing user-visible moves. `list -l`'s status column, `--sort status`, and `index.md`'s status headings all read the per-type value, exactly as before — they simply read it from `lifecycle` now. What changes is that the file finally tells a generic OKF consumer something true.

### The pre-0.7 spelling keeps working

A document whose `status` holds a value from its type's vocabulary, with no `lifecycle` key, is the old spelling of the same thing and is read as one. It sorts, groups, and displays unchanged, and earns **`E020`**, a warning naming both keys to write instead.

This is the same shape as the `timestamp` fallback above, for the same reason: a schema move that fails every existing bundle on upgrade is a schema move people route around. The ambiguity the fallback might seem to introduce — a PRD's `draft` and an ADR's `deprecated` are also OKF statuses — is harmless, because those values project onto themselves. Either reading gives the same answer.

### `# Citations` is left to the author

Arkouda has never validated a citations section — `Citations` is not in either built-in type's `required_sections`, and this repo's own ADRs carry one by convention. §13.1 supersedes the body list with `sources`, and `sources` is now parsed, so a project may move its citations into frontmatter and get per-claim footnote attribution. Nothing in arkouda enforces either shape, and nothing in this change rewrites existing citation sections.

### `Attested Computation` is not a built-in type

§10 is the largest addition in v0.2: a concept type with `runtime`, `parameters`, `computation`, `executor`, and `attester` fields, a verification-versus-attestation distinction, and a consumer protocol. §12 explicitly defers the runtime protocol, the attester ABI, and attestation caching to a future revision.

Implementing an incomplete subsystem as a built-in type would be arkouda taking a position on a design upstream has not finished. A project that wants attested computations in its bundle can declare the type in `.arkoudarc.toml` today and get scaffolding and validation for the fields it cares about. That is the right amount of support for a spec section still under construction.

### `E007` moves from the OKF tier to arkouda's profile

Re-reading §4.2 to check what v0.2 changed turned up a mistake this ADR corrects: **"There are no required body sections."** That is true in v0.1 too. Arkouda's [tiered `check`](support-user-defined-concept-types.md) placed `E007` — missing `#` heading — in the OKF tier, which meant a concept of an unconfigured type could fail a bundle for lacking a heading the spec never asked for. That is arkouda rejecting a conformant bundle, the exact failure the tiering was built to prevent.

Requiring a `#` heading, and requiring it to match `title`, is arkouda's contract. Both `E007` and `E008` now sit in the profile tier and apply only to concepts whose type the project configures.

## Consequences

### Positive

- Arkouda reads current OKF. A v0.2 bundle produced by any other tool — provenance, trust, staleness and all — is understood rather than passed over as unknown keys.
- Every v0.1 document keeps working, with one warning telling its author what to change and no deadline for changing it.
- `E016` turns `stale_after` into something a decisions tool can act on: an agent checking prior art now learns that a decision is past its review date.
- The `E007` conformance bug is fixed, and the tier table now matches what the spec actually requires.
- `status` means what the spec says it means, so an arkouda bundle is legible to any OKF consumer without special-casing: a rejected decision reads as `deprecated`, not as `stable`.
- The finer per-type vocabulary is not lost to the coarsening — it moves to `lifecycle`, where `check` still enforces it and every command still reads it.
- A project that wants an `Attested Computation` type can declare it without arkouda shipping an opinion about it.

### Negative

- **Every existing bundle warns until it is migrated.** `E017` fires once per document dated with `timestamp`, and `E013` fires once per bundle whose `index.md` still declares `okf_version: "0.1"`. Both are warnings and neither fails a build, but a large bundle will light up on first run after upgrading. `arkouda index` clears the second; the first is a hand edit per file, or the `arkouda migrate` this ADR declines to design.
- **A second schema move in two releases.** `status` changed meaning and `lifecycle` is new, one release after types became user-definable. The fallback keeps every existing bundle working, but a user upgrading through both releases sees two migration warnings on the same documents (`E017` and `E020`), and the fix for each is a hand edit.
- **Two keys now say overlapping things**, and one of them is derived from the other. That redundancy is deliberate — it is what lets a generic consumer read the file without knowing arkouda's types — but it is a thing to keep in step, and `E019` exists precisely because it can fall out of step.
- **A declared type that does not map its vocabulary gets `stable` for every non-obvious status.** A project with a `withdrawn` state will advertise it as `stable` until it writes an `okf_status` table. The default is the safe direction rather than the right one, and it is documented, but it is still a default that can be wrong.
- `arkouda new` writes a longer frontmatter block, and its `generated.by` is `arkouda/<version>` rather than a person. Left as-is, a bundle's provenance says a tool wrote everything.
- The instant format tightened: a `generated.at` of `2026-05-06` is now an `E006` error where a `timestamp` of `2026-05-06` was fine, and the same applies to every other v0.2 instant. Only documents that migrate wrongly hit this.

### Neutral

- Five diagnostic codes are added: `E016`, `E017`, and `E020` are warnings; `E018` (a `status` outside OKF's vocabulary) and `E019` (a `status` contradicting its `lifecycle`) are errors. No existing code is renumbered, though `E003` now judges `lifecycle` rather than `status`.
- Section references throughout the code, README, and skill are renumbered to v0.2: cross-linking §5→§6, index §6→§8, log §7→§9, conformance §9→§11, versioning §11→§12.
- The vendored spec is re-pinned; `LICENSE.md` is unchanged upstream and its checksum still matches.

## Alternatives Considered

### Keep arkouda's `status` and diverge from §5.4

Leave the key holding the per-type vocabulary and accept that a generic consumer reads every arkouda concept as `stable`, since §5.4 defaults an unrecognized value the same way it defaults a missing one and §11 forbids rejecting the concept.

Rejected. It costs nothing today and is wrong in the one direction that matters: a `rejected` or `superseded` decision advertises itself as current to every consumer that is not arkouda. A tool whose README claims OKF conformance should not have its most-read field mean something private. The fallback in this ADR is what makes the alternative unnecessary — the migration is a warning, not a wall.

### Put the per-type value in a per-type key

`decision_status` on an ADR, `requirement_status` on a PRD, matching how `deciders` and `owner` are already per-type extensions.

Rejected because every generic operation — the `list -l` column, `--sort status`, `index.md` grouping, `E003` — would have to ask a concept's type which key to read before reading it, and a concept whose type is not configured would have no key at all. One shared `lifecycle` keeps all of that type-agnostic.

### Derive `status` without storing it

Compute the OKF projection on demand and never write it into the file, keeping one key on disk.

Rejected because the file is the interface. A generic OKF consumer reads bytes, not arkouda's projection table; a `status` that exists only inside arkouda is a `status` that does not exist. Storing it is the entire point, and `E019` is what keeps the stored copy honest.

### Implement `Attested Computation` as a built-in type

Rejected as premature: §12 defers the runtime protocol, the attester ABI, and caching that the type depends on. Built-in support means committing to an interface upstream is still designing. User-defined types cover it in the meantime.

### Warn on a legacy `timestamp` only once per bundle rather than per concept

Would cut the upgrade noise considerably.

Rejected because a per-bundle warning cannot say *which* files to fix, and the fix is per file. A diagnostic that reports a problem without locating it is the kind arkouda's error codes exist to avoid.

### Stay on v0.1

Rejected. Upstream has superseded it, arkouda's own README advertises OKF conformance, and the gap only grows. v0.2 is a minor bump with a documented migration path and an explicit fallback for exactly this case.

## Citations

[1] [Open Knowledge Format v0.2 specification](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/62432a095456/okf/SPEC.md) — §4.2 no required body sections, §5 provenance/trust/lifecycle, §7 actor convention, §11 conformance, §12 versioning, §13 changes from v0.1. Pinned to the commit arkouda vendors at [`docs/okf/SPEC.md`](../okf/SPEC.md).
[2] [Adopt the Open Knowledge Format](adopt-okf.md) — the v0.1 adoption this supersedes, and the frontmatter mapping it established.
[3] [Support user-defined concept types](support-user-defined-concept-types.md) — the tiered `check` whose `E007` placement this corrects, and the mechanism that makes `Attested Computation` and OKF's own `status` vocabulary expressible without built-in support.
[4] [Support Product Requirements Documents](support-product-requirements-documents.md) — the source of the per-type status vocabularies this ADR declines to replace.
[5] Upstream commit [`62432a0`](https://github.com/GoogleCloudPlatform/knowledge-catalog/commit/62432a095456) — "make every timestamp an ISO 8601 datetime with an explicit offset", the reason v0.2 instants are validated more strictly than v0.1's.
