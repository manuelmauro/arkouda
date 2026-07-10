# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **BREAKING.** ADRs are now stored as an [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) (OKF) v0.1 knowledge bundle. Frontmatter adopts OKF's vocabulary: a new required `type: Architecture Decision Record`, `abstract` → `description`, `date` → `timestamp` (ISO 8601 date *or* datetime). `status`, `deciders`, and `superseded_by` are retained as OKF producer extensions. No compatibility shim is provided — `arkouda check` fails on the old schema, which is how you find the files to migrate. See [`docs/adr/adopt-okf.md`](docs/adr/adopt-okf.md).
- **BREAKING.** The `id` frontmatter key is removed. A concept's id is its path within the bundle without the `.md` suffix (OKF §2), so `security/mtls.md` has the id `security/mtls`. `arkouda decision <id>` accepts the concept id, the filename stem, or the filename, as before.
- **BREAKING.** `arkouda new --abstract` is now `--description`.
- **BREAKING.** `arkouda list --sort date` is now `--sort timestamp`; the `-l` table's third column is the timestamp.
- ADR discovery recurses into subdirectories. `index.md` and `log.md` are reserved by OKF §3.1 and are never treated as ADRs.
- `arkouda check` prints `[E001]: message` rather than `[E001] : message`, and `[E012] 3: message` for diagnostics that carry a line number.

### Added

- `arkouda index` — regenerate each bundle's `index.md` (OKF §6): every concept grouped under its status heading with its one-line description, for progressive disclosure. The bundle-root index declares `okf_version: "0.1"`, the one place OKF §11 permits frontmatter in an index. `arkouda new` refreshes an existing index but never creates one, since OKF §9 makes indexes optional.
- Validation of OKF reserved files: `E011` (an `index.md` carries frontmatter where OKF does not permit it) and `E012` (a `log.md` heading is not an ISO 8601 `YYYY-MM-DD` date).
- Two warnings, which report but never fail the run, per OKF's permissive-consumption rule (§9): `E013` (the bundle declares an OKF version arkouda does not implement) and `E014` (`index.md` is stale — run `arkouda index`).
- `E005` now flags a `type` that is not `Architecture Decision Record`, replacing the old "filename stem does not match id" check that the removal of `id` made vacuous.

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

[0.4.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.4.0
[0.3.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.3.0
[0.2.1]: https://github.com/manuelmauro/arkouda/releases/tag/v0.2.1
[0.2.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.2.0
[0.1.1]: https://github.com/manuelmauro/arkouda/releases/tag/v0.1.1
[0.1.0]: https://github.com/manuelmauro/arkouda/releases/tag/v0.1.0
