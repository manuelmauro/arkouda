---
id: telemetry-for-agent-command-invocations
title: Telemetry for agent command invocations
abstract: Record arkouda command invocations to a local JSONL file, on by default, attributing each event to the agent (or human) that ran it.
status: accepted
date: 2026-05-20
deciders: []
tags:
  - telemetry
  - observability
  - agents
  - cli
---

# Telemetry for agent command invocations

## Status

Accepted

## Context

Arkouda is increasingly invoked by coding agents — through the `use-arkouda` skill and equivalents in other harnesses — not just by humans typing at a terminal. We have no visibility into how that invocation actually plays out: which subcommands agents reach for, which flags they combine, how often `check` fires in CI versus locally, how often a `list | xargs rg` pipeline is the entry point, or whether `decision --section` is being used the way the skill recommends.

That gap matters for two reasons:

- **Skill and CLI evolution.** Decisions like ADR [`ls-style-list-and-decision`](./ls-style-list-and-decision.md) were partly informed by guesses about how agents use the tool. Future surface changes (new subcommands, flag deprecations, default behaviour) deserve evidence rather than guesses.
- **Agent ergonomics.** If agents systematically misuse a flag or fall back to a workaround, that's a signal the skill prompt or the CLI default is wrong. Without invocation data, the signal is invisible.

Several forces push back on adding telemetry:

- Arkouda is a small, local-first OSS CLI. Anything resembling "phone home" is hostile to the user base and to the project's tone.
- The project philosophy ([`defer-to-unix-tools`](./defer-to-unix-tools.md)) is to keep the surface tight and avoid behaviour the shell already provides. Telemetry is a new responsibility, not a removal.
- ADRs themselves can be sensitive — they contain decisions, names of products, internal projects. Their content must never leave the user's machine via arkouda.

The combination — wanting visibility into agent usage, but unwilling to ship a network client or exfiltrate any ADR content — points at a narrower design: record structured invocation events locally, where the operator can inspect, aggregate, or wipe them at will.

## Decision

Arkouda will emit one structured telemetry event per CLI invocation to a local JSONL file. There is no network sink and no remote collector; the events stay on the user's machine.

### Event shape

Each invocation appends one line of JSON to the telemetry log. The schema is intentionally small:

```json
{
  "ts": "2026-05-20T14:32:11Z",
  "version": "0.2.1",
  "command": "decision",
  "args": ["use-postgres", "--section", "consequences"],
  "exit_code": 0,
  "duration_ms": 38,
  "agent": "claude-code",
  "agent_source": "env:CLAUDECODE",
  "tty": false
}
```

Fields:

- `ts` — UTC ISO-8601 timestamp.
- `version` — arkouda's own version, so events from different builds can be told apart.
- `command` — the subcommand (`list`, `decision`, `check`, `new`); `null` for `--help`/`--version`.
- `args` — the argv tail after the subcommand, with values that look like file paths or free-text titles redacted to their kind (`<path>`, `<title>`). Flag names and enum values (`--sort`, `id`, `accepted`) are kept verbatim because that's the whole point.
- `exit_code`, `duration_ms` — outcome and rough cost.
- `agent` — short identifier of the detected agent, or `null` if none.
- `agent_source` — which signal produced the agent identification (e.g. `env:CLAUDECODE`, `env:CURSOR_AGENT`), so we can audit the detection.
- `tty` — whether stdout is a TTY, to roughly distinguish interactive use from scripted/CI use.

The schema is additive — new optional fields may be appended; existing fields will not change meaning.

### Agent detection

Detection is by env-var sniffing only, against a small, explicit allowlist maintained in code. The first match wins:

| Signal              | `agent`          |
|---------------------|------------------|
| `CLAUDECODE=1`      | `claude-code`    |
| `CURSOR_AGENT=1`    | `cursor`         |
| `AIDER=…`           | `aider`          |
| (none)              | `null`           |

The allowlist is short on purpose. Unknown agents show up as `agent: null`, which is itself a signal worth tracking. Users and other agents can land in the data without any identifying information being inferred about them.

### Default and override

Telemetry is **on by default**. Users opt out by setting `ARKOUDA_TELEMETRY=0` (or `false`/`off`), or by adding `telemetry = false` to `.arkoudarc.toml`. The opt-out is honoured before any event is written.

The first time arkouda writes to the telemetry log on a given machine, it prints a one-line notice to stderr pointing at this ADR and at the opt-out. The notice is suppressed on subsequent runs by a sentinel file alongside the log.

### Storage

Events are appended to `$XDG_STATE_HOME/arkouda/telemetry.jsonl` (defaulting to `~/.local/state/arkouda/telemetry.jsonl` on Linux, `~/Library/Application Support/arkouda/telemetry.jsonl` on macOS). The directory is created lazily on first write.

The file is rotated when it exceeds 10 MiB: the current file is renamed to `telemetry.jsonl.1` (overwriting any previous rotation) and a fresh file is started. Only one rotation is kept; older data is dropped. This keeps disk usage bounded without inventing a retention policy.

Writes use `O_APPEND` on a single line per event; partial writes from a killed process may leave a malformed final line, which downstream tools should tolerate.

### What is *not* recorded

- No `self completions` invocations. Shells run `eval "$(arkouda self completions zsh)"` from their startup file, so the command fires once per shell rather than once per deliberate use; recording it buries real invocations under startup noise.
- No ADR content, no titles, no abstracts, no file paths beyond the redacted kind.
- No environment variables other than the agent-detection allowlist.
- No hostname, username, IP address, or any other host identifier.
- No machine-stable id is generated; there is no concept of a "user" across invocations.

### Failure mode

If telemetry cannot be written (disk full, permission denied, opt-out parsing error), arkouda **silently continues with the command**. Telemetry never affects exit code, stderr (except the one-time notice), or behaviour. A `--quiet` invocation stays quiet.

## Consequences

### Positive

- Surface decisions about flags, defaults, and the skill prompt can be informed by what agents actually do, not by speculation. The data is right there on the developer's own machine; they can grep it.
- Agent authors can use the same log to verify their skill prompts produce the intended invocations.
- Local-only storage sidesteps the privacy, hosting, and dependency burden of a remote collector.
- The schema is small enough that `jq` over the JSONL file is the analysis tool, consistent with [`defer-to-unix-tools`](./defer-to-unix-tools.md).

### Negative

- On-by-default telemetry, even strictly local, is a posture some users will dislike. The first-run notice and trivial env-var opt-out are the mitigations, but the choice still carries a reputational cost.
- Arkouda gains responsibility for a file format, a rotation policy, and an argv redactor. All three are now things `check` and tests must cover.
- Env-var sniffing is a moving target. New agents will be invisible until the allowlist catches up; renamed env vars in existing agents will silently start producing `agent: null`.
- The aggregate "how is arkouda being used across the ecosystem?" question is still unanswered — local-only data lives on each developer's machine and is not collated anywhere.

### Neutral

- The schema is versioned implicitly via the `version` field rather than a `schema_version`. If a breaking change becomes necessary, a separate `schema_version` field can be added; downstream consumers should already tolerate unknown fields.
- Rotation keeping only one prior file is deliberately crude. If real users hit the limit and want more history, a follow-up ADR can revisit retention.
- A future ADR may add an opt-in remote sink for users or organisations that want centralised analytics. This ADR explicitly does not ship one.

## Alternatives Considered

### Opt-in by default

Standard for OSS CLIs and the safest reputational choice. Rejected for this iteration because the data we actually need — agent invocation patterns — only exists if telemetry runs by default in agent contexts, and we don't want to require every skill author to flip a flag. The first-run notice plus trivial opt-out is the compromise.

### Remote collector

A hosted endpoint would let us see usage across machines and agents in aggregate. Rejected: it forces arkouda to grow a network client, an auth story, a hosting bill, and a privacy policy, none of which match the project's scope. Local JSONL keeps the data where the user already trusts it to be.

### Explicit `--agent` flag instead of env sniffing

Cleaner and more honest than sniffing, but requires every skill and harness to pass the flag. In practice that means most invocations would land as `agent: null` until adoption catches up, defeating the purpose. Env sniffing is started with; an explicit override (`ARKOUDA_AGENT=<name>`) can be added later if sniffing proves brittle.

## Open Questions

- Should the first-run notice also be printed when telemetry is enabled but the user has previously opted out and then re-enabled? Probably yes, but worth confirming after first use.
- Is one rotated file enough, or should `telemetry.jsonl.1..N` be supported behind a config knob?
- Should `arkouda` grow a small `arkouda telemetry` subcommand (`status`, `path`, `clear`) for managing the log, or is that better left to `cat`, `rm`, and `jq`? The [`defer-to-unix-tools`](./defer-to-unix-tools.md) bias says leave it.
