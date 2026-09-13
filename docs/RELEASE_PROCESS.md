---
title: Release Process
nav_order: 7
---

# Release Process

This repository publishes stable releases from semantic version tags like `v0.1.0`. The full process is automated through GitHub Actions — no local scripts are required.

## Overview

1. Trigger the **Bump Version** workflow from the GitHub Actions UI
2. A version-bump PR is opened and auto-merges once CI passes
3. **tag-release** creates the annotated tag automatically
4. **release** builds platform launchers and creates the GitHub Release
5. **deploy-pages** publishes the Kassen-App WASM bundle to GitHub Pages

## Versioning

Use stable semantic versioning.

- bump `MAJOR` for intentional breaking changes
- bump `MINOR` for backward-compatible features and operator-visible improvements
- bump `PATCH` for backward-compatible fixes and release-only corrections

Pre-release tags like `v1.0.0-beta.1` are rejected by the release workflow.

## Prerequisites (one-time setup)

Enable **Allow auto-merge** in the repository:

> GitHub repository → Settings → General → Pull Requests → Allow auto-merge ✓

This allows the version-bump PR to merge automatically once the WASM Build check passes.

## Before A Release

1. Merge all intended work into `main` through pull requests.
2. Confirm the branch is clean and all CI checks are green on `main`.
3. Decide the next version number (see Versioning above).
4. Prepare any optional release notes you want included in the release.

## Create A Release

Go to **Actions → Bump Version → Run workflow** in the GitHub UI.

| Input | Required | Description |
| --- | --- | --- |
| `version` | yes | New version in `X.Y.Z` format, e.g. `0.2.0` |
| `notes` | no | Release notes prepended to the auto-generated GitHub Release notes |

What happens automatically:

1. Cargo.toml workspace version and Cargo.lock are updated on a `chore/bump-version-X.Y.Z` branch
2. A pull request is opened targeting `main`
3. The PR auto-merges once the **WASM Build** check passes
4. **tag-release** detects the version change on `main` and pushes the annotated tag `vX.Y.Z`
5. **release** builds launchers for Windows, macOS, and Linux; packages archives with checksums; creates the GitHub Release
6. **deploy-pages** builds the production WASM bundle and publishes it to GitHub Pages at `/pos/`

The full pipeline from workflow trigger to published release takes approximately 15–20 minutes.

## GitHub Actions Workflows

### `bump-version.yml`

Triggered manually. Opens the version-bump PR and enables auto-merge.

### `tag-release.yml`

Triggered on push to `main` when `Cargo.toml` changes. Compares the workspace version before and after the push; if the version changed and the tag does not yet exist, it creates and pushes the annotated tag.

### `release.yml`

Triggered when `tag-release.yml` completes successfully (via `workflow_run`), and also directly on tags matching `v*.*.*` for manually pushed tags. Validates the tag against `Cargo.toml`, builds platform launchers and the WASM bundle, packages archives, generates checksums, and creates the GitHub Release.

### `deploy-pages.yml`

Triggered on push to `main` (docs changes), when `tag-release.yml` completes successfully (via `workflow_run`), and directly on tags matching `v*.*.*`. On release triggers it additionally builds the Kassen-App WASM bundle with `LABELS_PUBLIC_URL` baked in and publishes it to GitHub Pages at `/pos/`.

## Static Deployment

Every release automatically publishes the Kassen-App to GitHub Pages:

| App | URL |
| --- | --- |
| Kassen-App | `https://tschuba.github.io/ez-vend-rs/pos/` |
| Label-App *(Phase 1)* | `https://tschuba.github.io/ez-vend-rs/labels/` |
| Mobile-App *(Phase 3)* | `https://tschuba.github.io/ez-vend-rs/mobile/` |

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

### Auto-merge does not trigger

Confirm **Allow auto-merge** is enabled in repo Settings → General → Pull Requests.

### Tag rejected by release workflow

The release workflow rejects non-semver tags, pre-release tags, and tags whose version does not match `Cargo.toml`. If `tag-release` created a tag before the merge was clean, delete the tag and re-run from the Bump Version workflow.

### Release build fails

Inspect the failed GitHub Actions job.

- launcher build failures are usually platform-specific toolchain issues
- WASM build failures are usually frontend dependency or `trunk` build issues
- packaging failures usually mean an expected artifact name changed

### Emergency follow-up release

1. Fix the issue on a new `fix/...` branch and merge through a pull request.
2. Trigger the **Bump Version** workflow with the next patch version.
