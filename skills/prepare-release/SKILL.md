---
name: prepare-release
description: Prepare a new arkouda release with changelog and version bumps
license: MIT OR Apache-2.0
---

# Prepare Release

Steps to prepare a new arkouda release. Pushing a `vX.Y.Z` tag triggers `.github/workflows/release.yml`, which builds Linux x86_64, macOS aarch64, and Windows x86_64 binaries and attaches them to a GitHub release.

## Checklist

1. `make ci` is green (fmt, clippy `-D warnings`, tests, build, `arkouda check` on `docs/adr/`)
2. `arkouda check` passes on the project's own ADRs
3. Update version in `Cargo.toml`
4. Update `Cargo.lock` (`cargo check`)
5. Update `CHANGELOG.md`
6. Find and update version references in docs
7. Commit changes (including `Cargo.lock`)
8. Create annotated git tag and push

## Version Bump

Update version in `Cargo.toml`:

```toml
[package]
version = "X.Y.Z"
```

After updating `Cargo.toml`, run `cargo check` to refresh `Cargo.lock`:

```bash
cargo check
```

## Changelog Format

Follow [Keep a Changelog](https://keepachangelog.com/) format. If `CHANGELOG.md` does not yet exist, create it for this release.

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes to existing functionality

### Fixed
- Bug fixes
```

## Update Version References in Docs

Search for hardcoded version numbers in documentation and update them:

```bash
# Find version references (e.g., arkouda@0.1.0, v0.1.0)
rg "arkouda@\d+\.\d+\.\d+" --type md
rg "v\d+\.\d+\.\d+" README.md
```

Common locations:
- `README.md` — CI example (`cargo install arkouda@X.Y.Z`)
- Installation instructions
- Badge URLs

## Release Commands

```bash
# Verify everything passes (mirrors what CI and lefthook run)
make ci

# Commit release changes
git add -A
git commit -m "chore: release vX.Y.Z"

# Create annotated tag
git tag -a vX.Y.Z -m "Release vX.Y.Z"

# Push commit and tag — pushing the tag triggers the release workflow
git push && git push --tags
```

The `release.yml` workflow handles the rest: cross-platform builds, sha256 checksums, and a GitHub release with `generate_release_notes: true` (so commit subjects since the previous tag become the release body).

## Publishing to crates.io

The release workflow does not publish to crates.io — do it manually after the GitHub release is up:

```bash
# Dry run first
cargo publish --dry-run

# Publish
cargo publish
```

## After release

- Confirm the binary downloads on the GitHub release page work (`curl -sSfL .../install.sh | sh` should now pick up the new version).
- If `cargo publish` succeeded, `cargo install arkouda` and `cargo install arkouda@X.Y.Z` both reach the new version.
