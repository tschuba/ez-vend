---
title: GitHub Repository Setup
nav_order: 8
---

# GitHub Repository Setup

One-time manual steps required in the GitHub web UI. None of these can be
applied via `gh api` from a script running without interactive auth, so they
are not part of `.github/repo-settings.yml` or any workflow. Do these once
per repository (e.g. after a fork or a fresh clone of the infra).

## 1. Enable GitHub Pages via Actions

**Settings → Pages → Build and deployment → Source: GitHub Actions**

`deploy-pages.yml` uses `actions/configure-pages@v6` and
`actions/deploy-pages@v5` — the Actions-native deployment flow, not a
`gh-pages` branch push. A fresh repository defaults to "Deploy from a
branch", which this workflow does not use. Until the source is switched to
"GitHub Actions", `deploy-pages.yml` runs will fail or no-op.

## 2. Allow auto-merge

**Settings → General → Pull Requests → Allow auto-merge**

Required for the version-bump PR created by `bump-version.yml` to merge
automatically once the WASM Build check passes. See
[`docs/RELEASE_PROCESS.md`](RELEASE_PROCESS.md) for the full release flow.

## 3. Create the `REPO_SETTINGS_TOKEN` secret

`apply-repo-settings.yml` applies the branch rulesets defined in
[`.github/repo-settings.yml`](../.github/repo-settings.yml) (PR review
requirements, required status checks, linear history). It runs on push to
`main` touching that file, nightly (drift correction), and on manual
dispatch — and fails without this secret.

1. **Settings → Developer settings → Personal access tokens → Fine-grained
   tokens → Generate new token**, scoped to this repository only.
2. Permissions:
   - **Administration**: Read and write
   - **Contents**: Read-only
3. **Settings → Secrets and variables → Actions → New repository secret**,
   name it `REPO_SETTINGS_TOKEN`, paste the token value.

Fine-grained PAT creation is not exposed via `gh api` or `gh auth`, so this
step requires a human with repo admin access in the GitHub UI.

Until this secret exists, any changes to `.github/repo-settings.yml` must be
applied manually via an authenticated local `gh api` session instead of the
workflow — see the `gh api .../rulesets` upsert logic in
`.github/workflows/apply-repo-settings.yml` for the exact calls to replay.

## Verifying setup

- Pages: push a docs change to `main`, confirm `Deploy Pages` succeeds and
  `https://tschuba.github.io/ez-vend/` serves the updated page.
- Auto-merge: run `bump-version.yml`, confirm the opened PR shows auto-merge
  enabled and merges once CI passes.
- Rulesets: `gh api repos/tschuba/ez-vend/rulesets` should list `pr-review`
  and `ci-and-history`; or wait for the nightly `apply-repo-settings` run to
  succeed.
