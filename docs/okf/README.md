# Open Knowledge Format — vendored copy

This directory holds a **verbatim, unmodified copy** of the Open Knowledge
Format (OKF) v0.2 specification, which arkouda implements. It is vendored so
the exact text arkouda was built against is pinned in this repository, readable
offline, and diffable when upstream changes.

This `README.md` is arkouda's provenance note. It is **not** part of the
specification and is not from upstream — upstream has its own, different
`okf/README.md`, which is not vendored here.

| File         | Origin                              |
| ------------ | ----------------------------------- |
| `SPEC.md`    | Upstream `okf/SPEC.md`, verbatim    |
| `LICENSE.md` | Upstream `okf/LICENSE.md`, verbatim |
| `README.md`  | Written for arkouda; not upstream   |

## Provenance

- **Upstream:** [GoogleCloudPlatform/knowledge-catalog](https://github.com/GoogleCloudPlatform/knowledge-catalog)
- **Source path:** `okf/SPEC.md`
- **Pinned commit:** [`62432a095456`](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/62432a095456/okf/SPEC.md) (2026-08-21) — the latest commit touching `okf/SPEC.md`
- **Retrieved:** 2026-09-04
- **Specification version:** OKF 0.2
- **Modifications:** none

`LICENSE.md` is unchanged upstream since the v0.1 pin, and its checksum is
identical; only `SPEC.md` moved.

```text
SHA-256  26aa5da029278939f914e578107242d9607d4f2dc5fe153272b82f9ed1030101  SPEC.md
SHA-256  8c6db340475136df3c1201d458fa5755698eace76e510471ecc9d857d6083dac  LICENSE.md
```

### Previous pin

OKF 0.1 (Draft) was vendored at
[`ee67a5ca2704`](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/ee67a5ca27044ebe7c38385f5b6cffc2305a9c1a/okf/SPEC.md)
(retrieved 2026-07-10, `SHA-256 b9655e607346dbbdc6de21190e9a953313eda6a7eba68d4d272a65975940ad6e`).
The move to v0.2 is recorded in [`docs/adr/adopt-okf-0-2.md`](../adr/adopt-okf-0-2.md).

## License

The specification is licensed under the Apache License, Version 2.0, by its
upstream authors. The full text is in [`LICENSE.md`](LICENSE.md); upstream
ships no `NOTICE` file. Neither file has been modified.

This is a third-party document redistributed under its own terms, and is not
covered by arkouda's own `MIT OR Apache-2.0` license.

## Refreshing this copy

Re-fetch at a chosen upstream commit and confirm the bytes before committing:

```bash
REV=<upstream-commit-sha>
BASE="https://raw.githubusercontent.com/GoogleCloudPlatform/knowledge-catalog/$REV/okf"

curl -sSfL "$BASE/SPEC.md"    -o docs/okf/SPEC.md
curl -sSfL "$BASE/LICENSE.md" -o docs/okf/LICENSE.md

git diff --stat docs/okf/          # review what upstream changed
shasum -a 256 docs/okf/SPEC.md docs/okf/LICENSE.md
```

Then update the pinned commit, retrieval date, and checksums above. If the
spec's _version_ changed (not just its text), also bump `OKF_VERSION` in
[`src/concept/mod.rs`](../../src/concept/mod.rs), re-run `arkouda index` to
refresh each bundle's declaration, and re-run `arkouda check` — a bundle
declaring an OKF version arkouda does not implement is reported as `E013`.

## How arkouda uses it

Arkouda stores concepts as an OKF knowledge bundle. The mapping from OKF's
vocabulary onto arkouda's metadata, and the decision to adopt the format, are
recorded in [`docs/adr/adopt-okf.md`](../adr/adopt-okf.md); the move from v0.1
to v0.2 is in [`docs/adr/adopt-okf-0-2.md`](../adr/adopt-okf-0-2.md).

Sections of `SPEC.md` referenced directly by the implementation:

| Section | What arkouda does with it                                                          |
| ------- | ---------------------------------------------------------------------------------- |
| §2      | A concept's id is its bundle-relative path without `.md`.                          |
| §3.1    | `index.md` and `log.md` are reserved; never treated as concepts.                   |
| §4.1    | Required `type`; `deciders`/`superseded_by`/`owner` are producer extensions.       |
| §4.2    | No body section is required, which is why `E007`/`E008` are arkouda's own profile. |
| §5.1    | `sources`, its credibility signals, and the `usage_window` sibling are parsed.     |
| §5.2    | `generated.at` is a concept's timestamp; a bare `verified` mapping is one entry.   |
| §5.4    | OKF's `status` vocabulary, which arkouda's per-type one deliberately refines.      |
| §5.5    | `stale_after` past its instant is reported as `E016`.                              |
| §7      | The actor convention `arkouda new` writes into `generated.by`.                     |
| §8      | `arkouda index` generates the bundle-root `index.md`.                              |
| §9      | `log.md` headings must be ISO 8601 dates (`E012`).                                 |
| §10     | Attested Computation — not built in; declarable as a project type.                 |
| §11     | Conformance floor, and why `E013`–`E017` are warnings rather than errors.          |
| §12     | `okf_version` in the bundle-root `index.md`.                                       |
| §13.1   | `generated.at` supersedes `timestamp` (`E017`); `sources` supersedes `# Citations`.|
