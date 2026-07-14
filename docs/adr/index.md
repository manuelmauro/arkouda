---
okf_version: "0.1"
---

# Accepted

* [Adopt the Open Knowledge Format](adopt-okf.md) - Store ADRs as an OKF v0.1 knowledge bundle, replacing arkouda's bespoke frontmatter schema with OKF's type/title/description/timestamp fields.
* [Provide a basic ADR navigation and validation CLI](basic-adr-cli.md) - Introduce a small CLI to navigate ADRs and validate their YAML metadata and Markdown structure.
* [Defer to Unix tools](defer-to-unix-tools.md) - Arkouda exposes structured access to the ADR collection but defers content search and other shell-friendly operations to standard Unix tools.
* [ls-style list and a decision subcommand](ls-style-list-and-decision.md) - Make arkouda list headerless and ls-style (paths by default, -l for the table) and replace show with arkouda decision <id>, defaulting to the decision section.
* [Telemetry for agent command invocations](telemetry-for-agent-command-invocations.md) - Record arkouda command invocations to a local JSONL file, on by default, attributing each event to the agent (or human) that ran it.
