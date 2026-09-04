---
type: Architecture Decision Record
title: Discover Bundles Instead of Configuring Them
description: Find every OKF bundle in a repository by walking for concepts, and delete the dirs configuration that used to name them.
tags:
  - config
  - discovery
  - okf
generated: { by: human:manuelmauro, at: 2026-09-04T14:11:45Z }
status: stable
lifecycle: accepted
deciders: []
---

# Discover Bundles Instead of Configuring Them

## Status

Accepted

## Context

Arkouda has never found a bundle. It has been *told* where one is.

`dirs` in `.arkoudarc.toml` names the roots, in a flat form every type shares
or a typed form per type, and `support-product-requirements-documents` recorded
what that key actually does:

> Both parse into one internal model — a type-to-roots map, plus the union used
> for search. [...] the typed `dirs` table is a default write target and a
> search scope, not a schema.

Two jobs in one key. As a **search scope** it is a configuration line that
restates what is already true on disk: a directory holds OKF concepts or it
does not, and the filesystem knows which. As a **default write target** it
answers a different question — where `arkouda new` should put a file that does
not exist yet — and that question survives.

Three things make the scope half a liability rather than a convenience.

**It is a second source of truth about the same fact.** Add a bundle and
arkouda ignores it until the config catches up. The failure is silent: `check`
exits 0 having validated nothing in the new directory, which is the worst
possible way for a gate to be wrong. A project that adds `docs/rfc` and forgets
the config line has a green build over unvalidated documents.

**It forces the defaults to be a guess.** With nothing configured, arkouda falls
back to `docs/adr` and `docs/prd`, which are conventions this tool invented. A
conformant OKF bundle in `knowledge/` or `decisions/` is invisible to a tool
that advertises OKF support until its owner discovers a config key.

**It makes the CLI's own philosophy inconsistent.** `defer-to-unix-tools` says
arkouda earns surface only where a shell pipeline cannot do the job. Finding
Markdown files with a particular frontmatter key is a walk — the one thing a
tool sitting on a repository is unambiguously better placed to do than its
user is to write down.

The constraint that makes this delicate is that **a concept id is its path
within its bundle**. `decisions` and `superseded_by` hold ids, and Arkouda Web
puts them in URLs. Any rule that moves a bundle root silently rewrites every id
inside it, and a rewritten id is a broken reference with no diagnostic — `E015`
fires on the *pointing* document, in a different bundle, if anyone runs `check`
at all. So the rule cannot be chosen on elegance. It has to be chosen on
whether it leaves today's ids alone.

## Decision

**A bundle is the topmost directory that directly contains at least one OKF
concept.** Everything beneath it belongs to that bundle, however deep. Walk the
repository, and stop descending the moment a directory holds a concept of its
own.

A concept, for the purpose of the walk, is a non-reserved `.md` file whose
frontmatter parses and declares a `type`. Reserved names (`index.md`, `log.md`)
never make a directory a bundle root: a bundle listing is not a bundle.

`dirs` and `[dirs]` are **deleted**. Not deprecated — a key that no longer
decides anything is worse than a key that is gone, because it goes on looking
authoritative. A config file carrying one is an error naming the replacement,
so nobody is left wondering why their scope is being ignored.

### Why this rule and not a simpler one

Two rules are more obvious and both are wrong:

- **Every directory containing a concept is a bundle.** `docs/adr/security/mtls.md`
  would make `docs/adr/security` its own bundle and `mtls` its id, where it is
  `security/mtls` today. That deletes nested ids as a feature.
- **The shallowest directory whose *subtree* contains concepts.** `docs/` becomes
  one bundle and every id gains a prefix: `adopt-okf` turns into `adr/adopt-okf`.
  Every reference in every repository breaks at once.

Topmost-with-a-direct-concept was chosen because it is the rule that leaves ids
alone, and that was verified rather than assumed. Run against both
repositories, it finds exactly the roots their configuration names —
`docs/adr` here; `docs/adr`, `docs/prd`, and `docs/threads` in Arkouda Web —
and reproduces all seventeen concept ids unchanged.

### What the walk must skip

Pointing a recursive walk at a whole repository is not the same as pointing it
at `docs/adr`, and the walk needs ignores it never needed before: `.git`,
`node_modules`, `target`, `vendor`, `dist`, `build`, `out`, `.next`, `.venv`,
`__pycache__`, and hidden entries. This is as much about correctness as speed —
a vendored dependency carrying its own OKF documents must not have them adopted
as this project's concepts.

### What survives

- **`default_dir`** on a type, as the *last* resort for `arkouda new`. The
  write target is now the first discovered bundle that already holds a concept
  of the requested type, and `default_dir` only when none does. Without that
  order a project whose ADRs live in `knowledge/decisions` would have them
  found by `check` and then have `new` start a second bundle in `docs/adr`,
  which is a worse failure than the config line this replaces. It is a
  creation default and never a search scope.
- **`--dir` and `ADR_DIR`**, with their meaning narrowed: they scope discovery
  to a subtree rather than declaring a root. `--dir docs/adr` discovers bundles
  within `docs/adr`, which for every real layout is that one bundle. Checking a
  single bundle in CI keeps working.
- **`[[types]]`**. Type declarations are a different question and are untouched.

## Consequences

Adding a bundle is adding a directory. No config change, and no window in which
`check` is silently validating less than the repository contains.

The `docs/adr` and `docs/prd` defaults stop existing as defaults, because there
is nothing left to default. A conformant bundle in `knowledge/` is found for the
same reason one in `docs/adr` is.

**One layout does change its ids**, and it is the honest cost of the rule: a
bundle whose concepts live *only* in subdirectories. With
`dirs = ["docs/adr"]` and nothing but `docs/adr/security/mtls.md`, the root was
`docs/adr` and the id `security/mtls`; under discovery the root is
`docs/adr/security` and the id `mtls`. Neither of these repositories has that
shape, and a project that does can restore its ids by putting any concept
directly in the parent. It is a real break and it is not detectable
automatically, which is why it is written down here rather than only in a
changelog.

**A concept at the repository root makes the repository one bundle**, and every
id gains its directory prefix. That is the rule working as stated rather than an
edge case to patch: a file at the top level says the top level is where the
knowledge lives.

**Deleting `dirs` deletes the only way to say "not that directory."** Nothing
now excludes a vendored or archived bundle beyond the fixed ignore list. That is
a genuine loss and the successor is an ignore mechanism — a list in
`.arkoudarc.toml`, or an `.arkoudaignore` — which is deliberately not designed
here. Building the exclusion before anyone has a bundle they need excluded would
be inventing the requirement.

Discovery costs one walk per invocation instead of a config read plus a walk per
configured root. On a repository with a large ignored subtree the ignore list is
what keeps that honest.

Arkouda Web mirrors this contract and must adopt the same rule, including the
ignore list, or the two disagree about what a repository contains. It reads its
tree from GitHub in a single request and so pays nothing for the walk; its
`typescript-okf-contract` corpus is where the agreement gets checked.
