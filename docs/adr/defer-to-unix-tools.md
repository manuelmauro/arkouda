---
id: defer-to-unix-tools
title: Defer to Unix tools
abstract: Arkouda exposes structured access to the ADR collection but defers content search and other shell-friendly operations to standard Unix tools.
status: accepted
date: 2026-05-06
deciders: []
tags:
  - philosophy
  - cli
---

# Defer to Unix tools

## Status

Accepted

## Context

ADRs are plain Markdown files with YAML frontmatter, sitting in a single directory. That layout already works well with the standard Unix toolchain: `ls`, `cat`, `grep`, `rg`, `fzf`, `awk`, `sed`, and shell pipelines all operate on these files directly.

The first cut of arkouda included a `search` subcommand that loaded every ADR, concatenated id, title, abstract, status, date, tags, and body into one lowercase string per file, did a substring match against a query, and printed matches in the same table format as `list`. In effect, it reimplemented `grep -l` with worse defaults: no regex, no surrounding context, no colour, no count, no follow-on piping. It also drifted from how users actually find things in code repositories — they reach for `rg` or their editor, not a per-tool search command.

The pattern generalises. Whenever arkouda adds a feature that a unix tool already does well, two things go wrong:

- The tool inherits maintenance cost for behaviour users already know from elsewhere.
- The output stops being composable, because each subcommand invents its own format.

## Decision

Arkouda's scope is **structured access** to the ADR collection: parsing, validating, scaffolding, resolving an id to a file, and extracting a named section. Anything that is "operate on the bytes of these Markdown files" is left to the shell.

Concrete consequences for the surface:

- **Removed** `arkouda search`. Use `rg <query> docs/adr/` (or `grep -ri <query> docs/adr/`) for content search.
- **Kept** `arkouda list`, `arkouda show`, `arkouda check`, `arkouda new` — these add real value over `ls`/`cat` (id-to-file resolution, schema enforcement, template scaffolding, table view).
- **Kept** `--section <name>` on `list` and `show` — extracting a named Markdown section from frontmatter-delimited content is non-trivial in pure shell, and the case-insensitive heading match plus blank-line trimming is genuinely worth a tool.
- New features are evaluated against the question: *can a one-line shell pipeline using standard tools do this already?* If yes, we don't add it.

## Consequences

- The CLI surface stays small and focused. Each subcommand has a clear reason to exist that's hard to replicate with off-the-shelf tools.
- Users compose arkouda with their existing shell habits — `arkouda list --section decision | rg postgres`, `arkouda show <id> | bat`, `ls docs/adr/ | wc -l`.
- Discoverability of "how do I search ADRs?" shifts from `arkouda --help` to general shell knowledge. The agent skill at `skills/use-arkouda/SKILL.md` documents the recommended pipelines so agents reach for the right tool.
- We accept that anyone expecting a built-in `search` will be briefly surprised. The `--help` output and skill doc point them at `rg`/`grep`.
- If a future feature is on the boundary (e.g. "summarise all ADRs in last 30 days"), the default answer is "pipe `arkouda list` through `awk`/`jq`" before considering a new subcommand.
