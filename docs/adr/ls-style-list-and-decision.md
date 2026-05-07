---
id: ls-style-list-and-decision
title: ls-style list and a decision subcommand
abstract: Make arkouda list headerless and ls-style (paths by default, -l for the table) and replace show with arkouda decision <id>, defaulting to the decision section.
status: accepted
date: 2026-05-07
deciders: []
tags: []
---

# ls-style list and a decision subcommand

## Status

Accepted

## Context

The [`defer-to-unix-tools`](defer-to-unix-tools.md) ADR set the rule: arkouda earns subcommands only where standard shell tools cannot. `list` and `show` predate that rule, and rereading them against it surfaces two seams.

`list` prints a header row and padded columns. The header pollutes every pipeline — `arkouda list | awk '$2=="accepted" {print $4}'` needs a redundant `NR>1`, `arkouda list --sort date | tail -10` shows the header among the results, `arkouda list | awk '{print $2}' | sort | uniq -c` counts "STATUS" as a status. The padded columns are friction too: agents piping `list` want IDs or paths, and the only consumer that wants the table is a human at a terminal.

`show <id>` does two things. With `--section`, it extracts a named Markdown section (case-insensitive H2 match, blank-line trimming) — non-trivial in shell, genuine value-add. Without it, `show <id>` equals `cat docs/adr/<id>.md` plus a tiny id-resolution convenience; once `arkouda check` is clean the filename stem equals the id, so the resolution is mechanical.

Two refinements that fall out of looking at this end-to-end:

- A `path` subcommand to resolve id → path is redundant. With a headerless `list` (below), `arkouda list | rg -F /<id>.md` is a one-liner, and `docs/adr/<id>.md` is the path by construction.
- The `Decision` section is qualitatively different from the other three required sections. The canonical question — "what did we decide about X?" — wants the Decision body; Status, Context, and Consequences are supporting. That asymmetry deserves a default, not yet another flag — and once a default exists, the command name should reflect it.

## Decision

Two changes to the CLI surface, building on `defer-to-unix-tools`:

**`list` becomes ls-style.**

- `arkouda list` prints one ADR file path per line. No header, no columns, no padding. `--sort id|date|status` still controls the order.
- `arkouda list -l` prints today's table (`ID STATUS DATE PATH TITLE`) **without a header row**.
- The default is the most composable shape possible: `arkouda list | xargs rg foo`, `arkouda list | wc -l`, `arkouda list | head -1`. The long form stays for humans skimming.

**`show` is replaced by `decision`.**

- `arkouda decision <id>` prints the body of that ADR's `## Decision` section (case-insensitive H2, blank lines trimmed) — the same extraction `show --section decision` performs today.
- `arkouda decision <id> --section <name>` overrides the section, accepting `context`, `consequences`, `status`, or any custom heading. Errors if the section is missing.
- Full-file display moves to the shell: `cat docs/adr/<id>.md`, or `bat docs/adr/<id>.md` for highlighting.

We do not add a `--decision` shortcut, a `path` subcommand, or per-section flags. The asymmetry that "the body of an ADR, for arkouda's purposes, is its decision" is encoded once, in the default.

## Consequences

- Pipelines stop working around a header and stop reaching for `awk` to slice columns. `arkouda list | xargs cat`, `arkouda list -l | awk '$2=="accepted"'`, and `arkouda decision <id>` cover the three motions an agent needs.
- Anyone running `arkouda show <id>` or relying on `list`'s header is surprised. The CLI bumps a minor version; the SKILL.md and README document the move. `--help` for `decision` states the default explicitly ("prints the Decision section; use `--section` to pick another").
- Naming `decision` is a strong opinion: it presumes Michael Nygard's four canonical sections stay canonical, and that Decision stays the headline among them. If a future convention demotes Decision (e.g. Y-statements that fold the decision into the title), this command becomes a vestige. We accept that — it is reversible by renaming.
- The earlier ADR's claim that `show` adds value over `cat` is now sharper: the value was only ever the section extractor. Naming a command after that fact is more honest than carrying a thin `cat` wrapper.
- Section extraction across the collection still composes through the shell: `arkouda list | while read f; do id=$(basename "$f" .md); echo "## $id"; arkouda decision "$id"; echo; done`.
