# CI Workflow Optimization

**Date**: 2026-05-20
**Status**: Implemented

## Problem

Every PR waits for a full `trunk build` (~4–5 min) regardless of what changed. A docs-only PR triggers the same WASM compilation as a deep Rust change. The `wasm-build` job was the bottleneck in all PR runs.

Additionally, the WASM build setup (Rust wasm32 target + Node.js + Trunk + npm ci) was copy-pasted across three workflows (`ci.yml`, `release.yml`, `deploy-pages.yml`), creating a reliability risk: any toolchain upgrade requires three edits, and a missed edit causes silent drift.

## Goals

1. Skip expensive jobs on PRs when their source files didn't change
2. Replace the full `trunk build` on PRs with `cargo check` (~30s) to catch WASM compilation failures fast
3. Add a stable `ci-gate` aggregator job as the single required branch protection check
4. Extract shared WASM setup into a composite action — one place to update

## Critical Constraint

`tag-release.yml` queries the GitHub API for a job with `.name == "WASM Build"` before creating a release tag. This name must be preserved exactly for push events on `chore/bump-version-*` branches.

## Architecture

### New composite action: `.github/actions/setup-wasm-build/action.yml`

Encapsulates the shared setup used for full WASM builds:
- `dtolnay/rust-toolchain@stable` (with `wasm32-unknown-unknown` target)
- `actions/setup-node@v4` (Node 20 + npm cache)
- `Swatinem/rust-cache@v2`
- `taiki-e/install-action@v2` (Trunk — already cached by this action)
- `npm ci` in `crates/ez-vend-app`

Used by: `wasm-build` in `ci.yml`, `build-wasm` in `release.yml`, conditional WASM block in `deploy-pages.yml`.

### Restructured `ci.yml`

| Job | Event: PR | Event: push |
|---|---|---|
| `changes` | Runs path filter, emits `rust`/`wasm` booleans | Runs but filter step skipped → all outputs default `true` |
| `lint` | Skipped if `rust=false` | Always runs |
| `test` | Skipped if `rust=false` | Always runs |
| `security-audit` | Skipped if `rust=false` | Always runs |
| `wasm-check` *(new)* | Runs if `wasm=true` — `cargo check -p ez-vend-app --target wasm32-unknown-unknown` | Skipped |
| `wasm-build` (name: "WASM Build") | Skipped | Always runs — full `trunk build` + artifact upload |
| `ci-gate` *(new)* | Always — fails on any `failure`/`cancelled` result | Always |

The `changes` job uses `dorny/paths-filter@v3` with two filter groups:
- `rust`: `**/*.rs`, `**/Cargo.toml`, `Cargo.lock`, `ci.yml`
- `wasm`: `crates/ez-vend-app/**`, `crates/ez-vend-ui/src/**`, `crates/ez-vend-core/src/**`, `crates/domain/src/**`, `crates/storage/src/**`, `Trunk.toml`

### Updated `release.yml` and `deploy-pages.yml`

The five WASM setup steps in `build-wasm` (release) and the five conditional WASM setup steps in `build` (deploy-pages) are replaced with a single composite action call. All other logic — triggers, job names, outputs, artifact names, conditionals — is unchanged.

## Workflow Chain Verification

```
bump-version (manual)
  → pushes chore/bump-version-X.Y.Z
  → creates PR with auto-merge

CI (push to chore/bump-version-*):
  └─ wasm-build (name "WASM Build") ✓ always runs on push
  └─ ci-gate passes ✓

tag-release (workflow_run: CI on chore/bump-version-*):
  └─ gh API finds job "WASM Build" → passed → creates tag ✓

release (push: tag v*.*.*):
  └─ validate → build-launcher × 3 + build-wasm → package → publish ✓

deploy-pages (workflow_run: Tag Release):
  └─ Jekyll + WASM (LABELS_PUBLIC_URL + ROUTER_BASE preserved) → GitHub Pages ✓
```

## Expected Performance

| PR type | Before | After |
|---|---|---|
| Docs only | ~5–6 min | ~30s |
| Rust, no WASM files | ~5–6 min | ~2–3 min |
| WASM UI change | ~5–6 min | ~2–3 min |
| Push to main / bump branch | ~5–6 min | ~5–6 min (unchanged) |

## Required Manual Step

Update branch protection for `main` in GitHub repo settings:
- **Add** `CI Gate` as required status check
- Remove individual job requirements (`Lint`, `Test`, `WASM Build`, etc.) if previously set

This ensures auto-merge in `bump-version.yml` gates on `ci-gate`.

## Files Changed

| File | Change |
|---|---|
| `.github/actions/setup-wasm-build/action.yml` | Created |
| `.github/workflows/ci.yml` | Restructured (changes + wasm-check + wasm-build split + ci-gate) |
| `.github/workflows/release.yml` | 5 setup steps → 1 composite action call |
| `.github/workflows/deploy-pages.yml` | 5 conditional setup steps → 1 conditional composite action call |
