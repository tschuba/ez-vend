---
title: Release Process
nav_order: 7
---

# Release Process

This repository publishes stable releases from semantic version tags like `v0.1.0`. The full process is automated through GitHub Actions — no local scripts are required, and no manual version bump is needed either.

## Overview

1. Every pull request title must follow [Conventional Commits](https://www.conventionalcommits.org/): `type(scope)?: description`
2. If the PR changes anything that affects the built artifact (`crates/**`, `Cargo.toml`, `Cargo.lock`, `Trunk.toml`, `crates/app/package*.json`), the **PR Title & Version Bump** workflow computes the version bump from the title's type and pushes a version commit onto the PR branch automatically
3. The PR cannot merge until branch protection's required checks — including the version bump — pass, and the branch is up to date with `main`
4. On merge, **tag-release** detects the version change on `main` and pushes the annotated tag `vX.Y.Z`
5. **release** builds platform launchers and the WASM bundle, and creates the GitHub Release
6. **deploy-pages** publishes the Kassen-App WASM bundle to GitHub Pages

## PR Title Convention

Every PR title must match `type(scope)?: description`, with an optional `!` before the colon for breaking changes.

| Type | Effect |
| --- | --- |
| `fix`, `perf` | patch bump |
| `feat` | minor bump |
| `type!` or body containing `BREAKING CHANGE` | major bump |
| `chore`, `docs`, `test`, `ci`, `style`, `build`, `refactor` | no bump |

If a PR changes `crates/**`, `Cargo.toml`, `Cargo.lock`, `Trunk.toml`, or `crates/app/package*.json` but its title type does not warrant a bump (e.g. `chore:` or `docs:`), the **Bump Version** check fails and blocks the merge — the title must be corrected to `fix`/`feat`/`perf` or marked breaking.

Editing the PR title after the bump already ran re-triggers the check and recomputes the version.

## Versioning

Use stable semantic versioning.

- bump `MAJOR` for intentional breaking changes
- bump `MINOR` for backward-compatible features and operator-visible improvements
- bump `PATCH` for backward-compatible fixes and release-only corrections

Pre-release tags like `v1.0.0-beta.1` are rejected by the release workflow.

## Prerequisites (one-time setup)

Branch protection on `main` requires the **PR Title** and **Bump Version** status checks, in addition to **CI Gate**, and requires the branch to be up to date before merging (`strict_required_status_checks_policy`). These are applied via `.github/repo-settings.yml` / `apply-repo-settings.yml`. See [`docs/GITHUB_SETUP.md`](GITHUB_SETUP.md) for one-time manual repository setup steps.

## GitHub Actions Workflows

### `pr-version-bump.yml`

Triggered on `pull_request` (`opened`, `edited`, `synchronize`, `reopened`) against `main`. Two jobs:

- **PR Title**: validates the Conventional Commits title format. Required status check.
- **Bump Version**: depends on **PR Title**. If the diff touches a build-affecting path, computes the new version from the title's type and pushes a commit updating `Cargo.toml`/`Cargo.lock` onto the PR branch. Fails if a build-affecting diff lacks a bump-worthy title. Required status check.

### `tag-release.yml`

Triggered on push to `main` when `Cargo.toml` changes. Compares the workspace version before and after the push; if the version changed and the tag does not yet exist, it creates and pushes the annotated tag.

### `release.yml`

Triggered on tags matching `v*.*.*`, and on manual `workflow_dispatch` for an existing tag. Validates the tag against `Cargo.toml`, builds platform launchers and the WASM bundle, packages archives, generates checksums, and creates the GitHub Release.

### `deploy-pages.yml`

Triggered on push to `main` (docs changes), when the Release workflow completes successfully (via `workflow_run`), and directly on tags matching `v*.*.*`. On release triggers it additionally builds the Kassen-App WASM bundle with `LABELS_PUBLIC_URL` baked in and publishes it to GitHub Pages at `/pos/`.

## Static Deployment

Every release automatically publishes the Kassen-App to GitHub Pages:

| App | URL |
| --- | --- |
| Kassen-App | `https://tschuba.github.io/ez-vend/pos/` |
| Label-App *(Phase 1)* | `https://tschuba.github.io/ez-vend/labels/` |
| Mobile-App *(Phase 3)* | `https://tschuba.github.io/ez-vend/mobile/` |

The Kassen-App is live after each release with no additional action required. The Label-App and Mobile-App URLs will be active once those crates are implemented.

The `LABELS_PUBLIC_URL` constant is baked into the Kassen-App WASM bundle at build time. Organizers who self-host must set this environment variable when building.

## Release Assets

Each GitHub Release contains:

- `ez-vend-windows-vX.Y.Z.zip`
- `ez-vend-macos-vX.Y.Z.tar.gz`
- `ez-vend-linux-vX.Y.Z.tar.gz`
- `checksums.txt`

Each platform archive includes:

- the platform launcher binary
- `index.html`
- built `.js`, `.css`, and `.wasm` files
- `README.txt` with operator instructions

## Verify Downloads

On macOS or Linux:

```bash
shasum -a 256 -c checksums.txt
```

On Windows PowerShell:

```powershell
Get-FileHash .\ez-vend-windows-v0.1.0.zip -Algorithm SHA256
```

Compare the reported hash with the matching line in `checksums.txt`.

## Troubleshooting

### Bump Version check fails

The PR title's type doesn't warrant a bump, but the diff touches `crates/**`/`Cargo.toml`/`Cargo.lock`/`Trunk.toml`/`crates/app/package*.json`. Fix the PR title to `fix:`/`feat:`/`perf:` (or mark it breaking) and push again.

### PR Title check fails

The PR title doesn't match `type(scope)?: description`. Rename the PR.

### Tag rejected by release workflow

The release workflow rejects non-semver tags, pre-release tags, and tags whose version does not match `Cargo.toml`.

### Release build fails

Inspect the failed GitHub Actions job.

- launcher build failures are usually platform-specific toolchain issues
- WASM build failures are usually frontend dependency or `trunk` build issues
- packaging failures usually mean an expected artifact name changed

### Emergency follow-up release

Fix the issue on a new `fix/...` branch with a `fix:` PR title, and merge through a pull request. The version bump and tag happen automatically.
