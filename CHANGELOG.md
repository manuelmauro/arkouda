# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[0.2.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.2.0
[0.1.1]: https://github.com/manuelmauro/arkouda/releases/tag/v0.1.1
[0.1.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.1.0
