# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING.** **Bundles are discovered, not configured.** A bundle is the topmost directory that directly contains an OKF concept — a non-reserved `.md` whose frontmatter declares a `type` — and everything beneath it belongs to that bundle, so a nested concept keeps its path in its id. `list`, `check`, `section`, and `index` read every bundle found. Adding `docs/rfc` is covered on the next run, with nothing to remember. See [`docs/adr/discover-bundles.md`](docs/adr/discover-bundles.md).
- **BREAKING.** **`dirs` and `[dirs]` are deleted, and a config file carrying either is an error** naming the replacement. Not deprecated: a key that no longer decides anything but still looks authoritative is worse than one that is gone, because the scope would quietly be the whole repository while the file said otherwise.
- **BREAKING.** The `docs/adr` and `docs/prd` search defaults stop existing, because there is nothing left to default. A conformant bundle in `knowledge/` is found for the same reason one in `docs/adr` is.
- **`--dir` and `ADR_DIR` narrow the walk rather than declaring a root.** `--dir docs/adr` discovers bundles within `docs/adr`, which for every real layout is that one bundle, so checking a single bundle in CI keeps working.
- **`arkouda new` picks its target in three steps**: the first bundle that already holds a concept of the type being created, then the `--dir` given, then that type's `default_dir` resolved against where discovery started. The first step stops a project whose ADRs live in `knowledge/decisions` from having `new` start a second bundle in `docs/adr`. The third matters more than it looks: a relative `default_dir` resolved against the process working directory writes into whatever repository the shell happens to be sitting in, which is exactly what the test suite did to this repository before it was fixed.
- The walk skips `.git`, `node_modules`, `target`, `vendor`, `dist`, `build`, `out`, `.next`, `.venv`, `venv`, `__pycache__`, `coverage`, and hidden directories. Needed only now that it starts at a repository rather than at a bundle, and about correctness as much as speed: a vendored dependency's own OKF documents must not be adopted as this project's concepts.

### Removed

- `ArkoudaError::NoDirForType`. Unreachable: `new` always has a target now.

### Upgrade impact

Delete the `dirs` key from `.arkoudarc.toml`; if that was the file's only content, delete the file. This repository's own config was exactly that and is deleted here.

Every layout these repositories use keeps its concept ids unchanged, and that was verified rather than assumed — the rule finds exactly the roots the configuration used to name.

**One layout does change its ids.** A bundle whose concepts live *only* in subdirectories: with `dirs = ["docs/adr"]` and nothing but `docs/adr/security/mtls.md`, the root was `docs/adr` and the id `security/mtls`; under discovery the root is `docs/adr/security` and the id `mtls`. Ids are references, so `decisions` and `superseded_by` entries pointing at the old spelling break, and nothing detects that automatically. Putting any concept directly in the parent restores the old root.

## [0.7.0] - 2026-09-04

### Added

- **OKF v0.2.** Arkouda now implements [Open Knowledge Format v0.2](docs/okf/SPEC.md), vendored at upstream commit [`62432a0`](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/62432a095456/okf/SPEC.md). Generated `index.md` files declare `okf_version: "0.2"`. See [`docs/adr/adopt-okf-0-2.md`](docs/adr/adopt-okf-0-2.md).
- The v0.2 provenance, trust, and lifecycle families are parsed rather than merely tolerated: `sources` (with `id`, `resource`, `title`, and the credibility signals `author`, `usage_count`, `last_modified`), the `usage_window` sibling, `generated`, `verified`, and `stale_after`. `verified` accepts either a list or a single bare `{ by, at }` mapping, which OKF §11 makes a MUST for consumers.
- **`lifecycle`, a new frontmatter key** carrying the per-type vocabulary that `status` used to hold — `accepted` for an ADR, `shipped` for a PRD, whatever a project declares. `status` now holds OKF §5.4's `draft | stable | deprecated`, so an arkouda bundle is legible to any OKF consumer without knowing its types.
  ```yaml
  status: stable          # OKF §5.4 — what any consumer reads
  lifecycle: accepted     # arkouda — what a reader of decisions wants
  ```
  The two are not independent: `status` is the **projection** of `lifecycle`. Every type declares where its values land — ADR `proposed`→`draft`, `accepted`→`stable`, `superseded`/`deprecated`/`rejected`→`deprecated`; PRD `draft`/`in-review`→`draft`, `approved`/`shipped`→`stable`, `abandoned`/`superseded`→`deprecated` — `arkouda new` writes both from one choice, and `check` reports a pair that disagrees. Declared types map their own with an optional `okf_status` table; an unmapped status projects onto `stable`, or onto itself when spelled like one of OKF's three.
- **`E016`** *(warning)* — the concept is past its `stale_after` instant (OKF §5.5). A decision whose review date has passed is exactly what an agent should know before relying on it.
- **`E018`** — `status` is not one of OKF's `draft | stable | deprecated`.
- **`E019`** — `status` contradicts the OKF projection of `lifecycle`. A `rejected` ADR advertised as `stable` is the fidelity loss adopting OKF's vocabulary was meant to fix. Absent `status` counts as `stable` (§5.4), so it disagrees too when the lifecycle projects elsewhere.
- **`E020`** *(warning)* — the per-type value is still in `status`, arkouda's pre-0.7 spelling of `lifecycle`.
- **`E017`** *(warning)* — the concept dates itself with v0.1's `timestamp` rather than `generated.at`.

### Changed

- **BREAKING.** **`generated: { by, at }` supersedes `timestamp`** (OKF §5.2, §13.1). `arkouda new` scaffolds `generated: { by: arkouda/<version>, at: <now> }` in the OKF §7 actor form — replace the `by` with your own `human:<id>` when you fill the template in. Reading falls back to a legacy `timestamp`, which the spec permits, so **every v0.1 document keeps validating, sorting, and displaying unchanged**; it earns an `E017` warning and nothing more. Arkouda's profile requires *a* content timestamp, not a particular spelling: `E001` fires only when neither key is present, `E002` when `generated` carries no `at`.
- **BREAKING.** Every instant v0.2 defines — `generated.at`, each `verified[].at`, `stale_after`, each `sources[].last_modified`, and both endpoints of a `usage_window` (shared or per-source) — must be an ISO 8601 datetime with an explicit offset (`E006`), following the upstream tightening in [`62432a0`](https://github.com/GoogleCloudPlatform/knowledge-catalog/commit/62432a095456). A legacy `timestamp` stays lenient and still accepts a plain date: tightening a key the spec has retired would break documents that are still perfectly readable.
- **Behaviour change on upgrade.** An existing bundle warns until migrated — `E017` once per document dated with `timestamp`, and `E013` once per bundle whose `index.md` still declares `okf_version: "0.1"`. Both are warnings and neither fails a build. `arkouda index` clears the second; the first is a hand edit per file.
- **BREAKING.** **`status` adopts OKF §5.4's vocabulary and the per-type values move to `lifecycle`** (see Added). `E003` now judges `lifecycle` rather than `status`. **A document written before 0.7 keeps working**: a `status` holding a value from its type's vocabulary, with no `lifecycle` key, is read as the lifecycle it always was, so it sorts, groups, and displays unchanged and earns an `E020` warning naming the two keys to write instead. Nothing user-visible moved — `list -l`'s status column, `--sort status`, and `index.md`'s status headings all still show the per-type value.
- `Attested Computation` (OKF §10) is not a built-in type. The runtime protocol, attester ABI, and attestation caching it depends on are explicitly deferred by OKF §12, and a project that wants the type can declare it in `.arkoudarc.toml` today.
- OKF section references throughout the code, README, and skill are renumbered to v0.2: cross-linking §5→§6, index §6→§8, log §7→§9, conformance §9→§11, versioning §11→§12.
- The `use-arkouda` skill declares `version: 0.7.0`.

### Fixed

- **`E007` moved from the OKF conformance tier to arkouda's profile.** OKF §4.2 states that there are no required body sections, so a concept with no `#` heading is conformant. Failing one meant arkouda could reject a bundle the spec accepts whenever the concept's type was not configured — the exact failure the tiering introduced in 0.6.0 was built to prevent. `E007` and `E008` now apply only to concepts whose type the project configures.

## [0.6.0] - 2026-09-04

### Added

- **User-defined concept types.** A project declares its own with `[[types]]` tables in `.arkoudarc.toml`, and a declared type gets everything the built-ins get: `--type`, a template, a status lifecycle, `index.md` grouping, and an `arkouda check` contract.
  ```toml
  [[types]]
  slug = "rfc"
  okf_type = "Request for Comments"
  statuses = ["draft", "active", "withdrawn"]
  required_sections = ["Status", "Summary", "Motivation"]   # optional
  primary_section = "Summary"                               # optional
  default_dir = "docs/rfc"
  template = "docs/templates/rfc.md"                        # optional
  extensions = ["sponsors"]                                 # optional
  ```
  `slug`, `okf_type`, `statuses`, and `default_dir` are required; status labels are derived from their names (`in-review` → `In Review`). `required_sections` is optional, so a type may take a lifecycle and a template without a body contract — nothing for `check` to fail on. `template` points at a Markdown file whose `##` headings and prose become the scaffold; it may exceed `required_sections` but not fall short of them. A declared type whose `slug` or `okf_type` matches a built-in replaces it wholesale, which is how a project changes ADR's own sections or statuses. Config mistakes — a slug `--type` could not take, an empty lifecycle, a `primary_section` outside `required_sections`, a template short of a required section, two tables claiming one slug, a misspelled key — are reported when the config loads. See [`docs/adr/support-user-defined-concept-types.md`](docs/adr/support-user-defined-concept-types.md).
- **`arkouda check` validates in three tiers.** OKF conformance (`E000`, `E004`, `E007`, `E010`, `E011`, `E012`) for every concept; arkouda's frontmatter profile (`E001`, `E002`, `E006`, `E008`) for concepts whose `type` resolves to a configured type; and that type's contract (`E003`, `E009`) for its vocabulary and sections. No codes are added or renumbered — the change is which tier each belongs to. The tiers are what let arkouda be strict about the format it implements and permissive about the contracts a project has chosen not to write down.
- The `use-arkouda` skill declares `version: 0.6.0` in its frontmatter, tracking the arkouda release whose CLI surface it documents. The skill's contract changed materially this release: an agent must now read the discovered `.arkoudarc.toml` before scaffolding, because `--type adr` is no longer guaranteed to exist or to mean the built-in ADR.
- Telemetry events gain two fields, both omitted when empty: `codes`, the sorted diagnostic codes a `check` produced, and `type_kind`, whether a resolved `--type` was `builtin` or `custom`. The type's slug is project-specific free text and is deliberately not recorded — a `--type` value in the recorded argv is kept only when it names a built-in, and becomes `<type>` otherwise. An exit code alone cannot say which rules fire, which is the evidence a future change to the tiers or to a built-in contract has to be made on.

- **Product Requirements Documents as a second built-in concept type.** Arkouda is now a concept-type-aware OKF tool rather than an ADR-only one: a `ConceptType` descriptor carries each type's OKF `type` string, status vocabulary, required sections, primary section, default directory, and template, and the two built-in types are Architecture Decision Record and Product Requirements Document. A PRD's required sections are `Status`, `Problem`, `Requirements`, `Non-Goals`, and `Success Metrics`; `Approach` and `Open Questions` are scaffolded but not validated. Its statuses are `draft`, `in-review`, `approved`, `shipped`, `abandoned`, `superseded`, and its default directory is `docs/prd`. Types are not user-definable. See [`docs/adr/support-product-requirements-documents.md`](docs/adr/support-product-requirements-documents.md).
- `arkouda new --type adr|prd` (defaults to `adr`) selects the template, the `type` string, the status vocabulary, and the target directory.
- `arkouda list --type adr|prd` filters by the type a concept declares in its frontmatter — never by the directory it sits in, so a bundle may hold both.
- Three optional frontmatter producer extensions: `owner` (a PRD's counterpart to `deciders`), `target_release`, and `decisions` — a list of the concept ids of the ADRs that shaped a PRD. `decisions` is frontmatter rather than a body section so that it survives reconfiguring where a type's documents live, and so that `check` can resolve it.
- **`E015`** *(warning)* — a frontmatter concept reference does not resolve to a loaded concept. It covers `decisions` and, for the first time, `superseded_by`, which had been parsed but never validated since it was introduced. A warning rather than an error because arkouda cannot distinguish a broken reference from one pointing into a bundle the invocation did not load.
- `.arkoudarc.toml` accepts a per-type `dirs` table alongside the existing flat list:
  ```toml
  [dirs]
  adr = ["docs/adr"]
  prd = ["docs/prd"]
  ```
  Both forms parse into one model: a type-to-roots map plus the union that `list`, `check`, `section`, and `index` search. `new` writes into the first root configured for the type it is creating. With nothing configured the defaults are `adr → docs/adr` and `prd → docs/prd`.

### Changed

- **BREAKING.** **`E005` is now a warning.** A concept whose `type` no configured type declares is validated for OKF conformance, reported, and left to pass, where it used to fail the run. It is not skipped: its concept id, heading, and bundle placement are still checked, and it still appears in `list` and `index`. This reverses the position taken in [`support-product-requirements-documents`](docs/adr/support-product-requirements-documents.md), which was sound under a closed registry of two types and unsound under an open one — an unrecognized `type` is now ordinarily one the operator has not declared, and failing on it made `arkouda check` reject conformant OKF bundles. Deleting a `[[types]]` table therefore degrades its documents rather than breaking a build. A CI run that was red because of an unknown type goes green; nothing that was green turns red.
- **BREAKING.** `--type` is no longer a clap enum on `list` and `new`. Which slugs are valid depends on a `.arkoudarc.toml` that has not been read when argv is parsed, so the value is resolved after the config loads and an unresolvable one names the slugs that are configured. `list --type <unknown>` used to silently list everything and now errors. Shell completions offer nothing for `--type`, as they already do not for `--status`.
- `arkouda section <id>` with no section name errors when the concept's type names no `primary_section`, alongside the existing error for a type arkouda does not know.
- **BREAKING.** `arkouda decision <id> [--section <name>]` is now `arkouda section <id> [<name>]`, with the section name as an optional positional and **no `decision` alias**. With no name it prints the concept type's primary section — `## Decision` for an ADR, `## Requirements` for a PRD. A command that announces its output as a decision invites a requirements section to be recorded as one, and arkouda's primary consumer is an agent. This revises the naming half of [`ls-style-list-and-decision`](docs/adr/ls-style-list-and-decision.md); its list decision stands.
- **BREAKING.** `arkouda list -l` gains a type column: the table is now `ID TYPE STATUS TIMESTAMP PATH TITLE — DESCRIPTION`. `awk '$2=="accepted"'` becomes `$3`, and `{print $4}` becomes `{print $5}`. The type is the one fact about a mixed collection that nothing else reveals — it cannot be inferred from the path — and `list -l` is the first command run in an unfamiliar repo. `arkouda list` without `-l`, the actual pipeline surface, is untouched.
- **BREAKING.** Every `index.md` regenerates with a new heading structure: `# <Type>` at H1 and `## <Status>` at H2, where before status was the H1. Nesting is uniform, so a single-type bundle pays one extra heading level and consumers parse one shape instead of two. Existing bundles report `E014` (stale index, a warning) until `arkouda index` is run; no concept document needs editing.
- **BREAKING.** `arkouda new --status` is no longer a clap `ValueEnum`, because which values are valid now depends on `--type`. It is validated against the resolved type's vocabulary and defaults to the first of its lifecycle (`proposed` for an ADR, `draft` for a PRD). Shell completions offer nothing for it until they learn to be type-aware.
- `E005`, `E003`, and `E009` are now type-relative: `E005` fires when `type` is not one of the types arkouda knows (rather than when it is not `Architecture Decision Record`), `E003` checks `status` against *that type's* vocabulary, and `E009` checks *that type's* required sections. A concept whose `type` arkouda does not know is an `E005` error rather than a skipped file, and its sections are not checked against another type's contract.
- **Behaviour change on upgrade.** A concept whose required headings exist only inside a code fence validated before and now fails with `E009`. That is the fix working as intended, but it can turn a passing CI run red without the document having changed.
- The `adr` module is now `concept`, and `AdrStatus` and `ADR_TYPE` are gone, replaced by the per-type descriptor. The `ADR_DIR` environment variable keeps its name for compatibility.
- `arkouda check` counts "concept(s)" rather than "ADR(s)", and the error text for a missing collection, an ambiguous lookup, and an existing file says "concept" rather than "ADR".

### Fixed

- Markdown structure is determined by parsing the document rather than scanning lines for a `## ` prefix, fixing two defects. A heading inside a fenced or indented code block counted as a real heading, so an ADR whose only `## Decision` and `## Consequences` headings sat inside a fenced template passed `arkouda check` while missing both sections. And `arkouda decision` truncated its output at the first heading inside a fence. Setext headings (`Title` over `=====`) are now recognized. See [`docs/adr/parse-markdown-instead-of-scanning-lines.md`](docs/adr/parse-markdown-instead-of-scanning-lines.md).
- Only document-level headings count as sections. A heading nested in a block quote or list item (`> ## Decision`) belongs to that container, so quoting a template no longer satisfies the requirement to have written one.
- `arkouda decision` ends a section at the next heading of the same or higher level rather than at the next `##`, so an intervening `#` no longer lands in the preceding section's body and a nested `###` subsection is no longer excluded from it.

## [0.5.0] - 2026-07-14

### Changed

- **BREAKING.** ADRs are now stored as an [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) (OKF) v0.1 knowledge bundle. Frontmatter adopts OKF's vocabulary: a new required `type: Architecture Decision Record`, `abstract` → `description`, `date` → `timestamp` (ISO 8601 date *or* datetime). `status`, `deciders`, and `superseded_by` are retained as OKF producer extensions. No compatibility shim is provided — `arkouda check` fails on the old schema, which is how you find the files to migrate. See [`docs/adr/adopt-okf.md`](docs/adr/adopt-okf.md).
- **BREAKING.** The `id` frontmatter key is removed. A concept's id is its path within the bundle without the `.md` suffix (OKF §2), so `security/mtls.md` has the id `security/mtls`. `arkouda decision <id>` accepts the concept id, the filename stem, or the filename, as before.
- **BREAKING.** `arkouda new --abstract` is now `--description`.
- **BREAKING.** `arkouda list --sort date` is now `--sort timestamp`; the `-l` table's third column is the timestamp.
- ADR discovery recurses into subdirectories. `index.md` and `log.md` are reserved by OKF §3.1 and are never treated as ADRs.
- `arkouda check` prints `[E001]: message` rather than `[E001] : message`, and `[E012] 3: message` for diagnostics that carry a line number.
- `arkouda new` no longer fails when it cannot refresh the bundle's `index.md`. Refreshing re-parses every concept, so an unrelated malformed ADR used to turn a successful creation into exit code 1. The refresh failure is now a warning on stderr and the command exits 0; `arkouda check` reports the resulting stale index as `E014`.

### Added

- `arkouda index` — regenerate each bundle's `index.md` (OKF §6): every concept grouped under its status heading with its one-line description, for progressive disclosure. The bundle-root index declares `okf_version: "0.1"`, the one place OKF §11 permits frontmatter in an index. `arkouda new` refreshes an existing index but never creates one, since OKF §9 makes indexes optional.
- Validation of OKF reserved files: `E011` (an `index.md` carries frontmatter where OKF does not permit it) and `E012` (a `log.md` heading is not an ISO 8601 `YYYY-MM-DD` date).
- Two warnings, which report but never fail the run, per OKF's permissive-consumption rule (§9): `E013` (the bundle declares an OKF version arkouda does not implement) and `E014` (`index.md` is stale — run `arkouda index`).
- `E005` now flags a `type` that is not `Architecture Decision Record`, replacing the old "filename stem does not match id" check that the removal of `id` made vacuous.
- Vendored a verbatim copy of the OKF v0.1 specification at [`docs/okf/SPEC.md`](docs/okf/SPEC.md), together with its Apache 2.0 licence and a provenance note pinning the upstream commit and SHA-256 checksums. The exact text arkouda implements is now in-tree, readable offline, and diffable when upstream moves.

## [0.4.0] - 2026-06-12

### Added

- `arkouda self completions <shell>` — print a shell completion script to stdout for `bash`, `zsh`, `fish`, `powershell`, or `elvish`, generated with `clap_complete`. Add `eval "$(arkouda self completions bash)"` to your shell profile (or `arkouda self completions fish | source`) for tab completion of subcommands, flags, and enum values.
- Pinned Rust toolchain via `rust-toolchain.toml` (1.92.0 with `rustfmt` and `clippy`) for reproducible builds across contributors.

## [0.3.0] - 2026-05-20

### Added

- Local invocation telemetry. Each arkouda invocation appends one JSON event to `telemetry.jsonl` under the OS state directory (`~/Library/Application Support/arkouda` on macOS, `$XDG_STATE_HOME/arkouda` or `~/.local/state/arkouda` elsewhere) — no network, no remote collector. Events capture the subcommand, redacted argv (paths and free-text titles become `<path>` / `<title>` markers; flag names and short slugs pass through), exit code, duration, TTY presence, and an agent identifier derived from a small env-var allowlist (`CLAUDECODE` → `claude-code`, `CURSOR_AGENT` → `cursor`, `AIDER` → `aider`). The log rotates at 10 MiB keeping one prior file. Write failures are silently swallowed so telemetry never affects the command's outcome. See [`docs/adr/telemetry-for-agent-command-invocations.md`](docs/adr/telemetry-for-agent-command-invocations.md) for the full design.
- `telemetry` key in `.arkoudarc.toml` (`telemetry = false` to opt out per-project).

### Changed

- Telemetry is on by default. Opt out with `ARKOUDA_TELEMETRY=0` (also accepts `false`/`off`/`no`) or `telemetry = false` in `.arkoudarc.toml`. The first eligible invocation prints a one-line notice to stderr pointing at the log path and the opt-out; the notice is suppressed under `--quiet` and on subsequent runs via a sentinel file.

## [0.2.1] - 2026-05-07

### Docs

- Reframe arkouda as an AI-native CLI built for AI coding agents. Updated README lede, GitHub About description and topics, and `Cargo.toml` description and keywords. The portable agent skill, structured pipe-friendly output, and `E000`–`E010` validator diagnostics were always there — the messaging now leads with them. README's Agent skill section expanded with the before/after-deciding workflow.

## [0.2.0] - 2026-05-07

### Added

- `.arkoudarc.toml` config file with a `dirs = [...]` list of ADR directories, discovered by walking up from the working directory. Useful for monorepos that keep ADRs per service or area. Relative paths resolve against the config file's location, so the same file works from any subdirectory. `arkouda list`, `check`, and `decision` aggregate across all listed dirs; `arkouda new` writes into the first one (override with `--dir`). Precedence: `--dir` > `ADR_DIR` > `.arkoudarc.toml` > default `docs/adr`.

### Changed

- `arkouda list` now prints one ADR file path per line by default — no header, no padded columns. Pipe it straight into `xargs`/`rg`/`cat`/`wc`. Pass `-l` for the long-form `ID STATUS DATE PATH TITLE` table (still headerless).
- Replaced `arkouda show <id>` with `arkouda decision <id>`. The new command prints the body of the `## Decision` section by default; pass `--section <name>` to pick another. Full-file display moves to the shell (`cat docs/adr/<id>.md`). See [`docs/adr/ls-style-list-and-decision.md`](docs/adr/ls-style-list-and-decision.md) for rationale.
- Renamed the agent skill `skills/arkouda` → `skills/use-arkouda` and rewrote it to be repo-agnostic: it now triggers any time a non-trivial decision is being made, not just when the user explicitly mentions ADRs, and tells agents to discover ADR paths via `arkouda list` instead of hardcoding `docs/adr/`. Drop it into any project that uses arkouda.

### Removed

- `arkouda show` — `show <id>` without `--section` was just `cat docs/adr/<id>.md` with id resolution; with `--section` it has been folded into `arkouda decision`.
- The header row from `arkouda list -l`.

## [0.1.1] - 2026-05-06

### Changed

- `arkouda list` now includes a `PATH` column so the table composes directly with shell tools (e.g. `arkouda list | awk 'NR>1 && $2=="accepted" {print $4}' | xargs cat`).

### Removed

- `arkouda list --section <name>` — the flag silently switched the command between a metadata table and a content digest. For a single section of a single ADR, `arkouda show <id> --section <name>` is unchanged. For collection-wide section extraction, compose with `awk`/`xargs`/`rg`.

### Docs

- Credit Michael Nygard's ADR template (the source of the `Status` / `Context` / `Decision` / `Consequences` body schema) in the README, the `basic-adr-cli` ADR, and the agent skill.

## [0.1.0] - 2026-05-06

Initial release.

### Added

- `arkouda list` — table of every ADR in the directory, with `--sort id|date|status` and a `--section <name>` flag that prints a Markdown digest of that section across all ADRs.
- `arkouda show <id>` — print one ADR by frontmatter id, filename stem, or filename. With `--section <name>`, print only that section's body.
- `arkouda check` — validate frontmatter, filename, and required Markdown structure across the collection. Reports diagnostics with codes `E000`–`E010` and fix hints; exits 1 on any error.
- `arkouda new "<title>"` — scaffold a new ADR from the standard template, with optional `--id`, `--status`, and `--abstract` flags.
- Frontmatter schema: required `id`, `title`, `abstract`, `status`, `date`; optional `deciders`, `tags`, `superseded_by`. Status is one of `proposed | accepted | superseded | deprecated | rejected`.
- Body schema: `# <title>` H1 plus required `## Status`, `## Context`, `## Decision`, `## Consequences` sections.
- Configurable ADR directory via `--dir <path>` or `ADR_DIR=<path>` (default `docs/adr`).
- GitHub Actions CI (fmt, clippy `-D warnings`, tests, build) and tagged-release workflow that ships Linux x86_64, macOS aarch64, and Windows x86_64 binaries with sha256 checksums.
- `install.sh` quick installer that prefers a pre-built release binary and falls back to `cargo install arkouda`.
- Dual MIT/Apache-2.0 license.
- Agent skills: `skills/arkouda` (how to use the CLI) and `skills/prepare-release` (how to cut a release).

[0.7.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.7.0
[0.6.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.6.0
[0.5.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.5.0
[0.4.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.4.0
[0.3.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.3.0
[0.2.1]: https://github.com/manuelmauro/arkouda/releases/tag/v0.2.1
[0.2.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.2.0
[0.1.1]: https://github.com/manuelmauro/arkouda/releases/tag/v0.1.1
[0.1.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.1.0
