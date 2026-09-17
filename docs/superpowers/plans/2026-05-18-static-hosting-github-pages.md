# Static Hosting on GitHub Pages — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deploy ez-vend-app (Kassen-App) as a static WASM bundle to GitHub Pages on every release tag, served at `/ez-vend/pos/`, with the Label-App URL baked in at build time.

**Architecture:** A new `deploy-pages.yml` workflow replaces `docs.yml`. On pushes to `main` it builds and deploys Jekyll only; on release tags it additionally builds the WASM bundle with `trunk --public-url /ez-vend/pos/` and merges the output into the Jekyll site before publishing. A compile-time constant `LABELS_PUBLIC_URL` in `ez-vend-ui` makes the Label-App URL available to the app's UI layer, where label link generation lives.

**Tech Stack:** GitHub Actions, trunk (WASM bundler), Rust/wasm32-unknown-unknown, Jekyll (docs), `actions/deploy-pages@v4`, `dtolnay/rust-toolchain`, `taiki-e/install-action` (trunk)

---

## File Map

| File | Action | Purpose |
|------|--------|---------|
| `crates/ez-vend-ui/src/config.rs` | **Create** | `LABELS_PUBLIC_URL` compile-time constant |
| `crates/ez-vend-ui/src/lib.rs` | **Modify** | Declare `pub mod config` |
| `.github/workflows/deploy-pages.yml` | **Create** | Unified docs + WASM Pages deploy |
| `.github/workflows/docs.yml` | **Delete** | Replaced by deploy-pages.yml |

> **Why `ez-vend-ui` not `ez-vend-app`:** `ez-vend-app` is the WASM entry point crate; `ez-vend-ui` is where label link generation will live (the UI component layer). Rust dependencies flow `ez-vend-app → ez-vend-ui`, so the constant must live in `ez-vend-ui` to be reachable from the code that uses it.

---

## Task 1: Add LABELS_PUBLIC_URL config constant

**Files:**
- Create: `crates/ez-vend-ui/src/config.rs`
- Modify: `crates/ez-vend-ui/src/lib.rs`

- [ ] **Step 1: Create config.rs**

```rust
// crates/ez-vend-ui/src/config.rs
pub const LABELS_PUBLIC_URL: &str =
    option_env!("LABELS_PUBLIC_URL").unwrap_or("http://localhost:8080/");
```

`option_env!` returns `None` at compile time when the variable is unset, so local `trunk serve` continues to work without any env configuration.

- [ ] **Step 2: Declare the module in lib.rs**

In `crates/ez-vend-ui/src/lib.rs`, add after the existing `mod utils;` line:

```rust
pub mod config;
```

- [ ] **Step 3: Verify compilation without env var (local dev path)**

```bash
cargo check -p ez-vend-ui
```

Expected: compiles cleanly with no warnings. `LABELS_PUBLIC_URL` resolves to `"http://localhost:8080/"`.

- [ ] **Step 4: Verify compilation with env var set (CI path)**

```bash
LABELS_PUBLIC_URL=https://tschuba.github.io/ez-vend/labels/ cargo check -p ez-vend-ui
```

Expected: compiles cleanly. `LABELS_PUBLIC_URL` resolves to the GitHub Pages URL.

- [ ] **Step 5: Commit**

```bash
git add crates/ez-vend-ui/src/config.rs crates/ez-vend-ui/src/lib.rs
git commit -m "feat: add LABELS_PUBLIC_URL compile-time config constant to ez-vend-ui"
```

---

## Task 2: Create deploy-pages.yml

**Files:**
- Create: `.github/workflows/deploy-pages.yml`
- Delete: `.github/workflows/docs.yml`

- [ ] **Step 1: Delete docs.yml**

```bash
git rm .github/workflows/docs.yml
```

- [ ] **Step 2: Create deploy-pages.yml**

```yaml
# .github/workflows/deploy-pages.yml
name: Deploy Pages

on:
  push:
    branches:
      - main
    paths:
      - docs/**
      - .github/workflows/deploy-pages.yml
      - README.md
    tags:
      - 'v*.*.*'

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  # Cancel in-progress docs deploys, never cancel release deploys
  cancel-in-progress: ${{ github.ref_type != 'tag' }}

jobs:
  build:
    name: Build Pages Site
    runs-on: ubuntu-latest

    steps:
      - name: Check out repository
        uses: actions/checkout@v4

      - name: Set up Pages
        uses: actions/configure-pages@v5

      - name: Build with Jekyll
        uses: actions/jekyll-build-pages@v1
        with:
          source: docs
          destination: ./_site

      # ── WASM steps run only on release tags ──────────────────────────

      - name: Install Rust toolchain
        if: github.ref_type == 'tag'
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Set up Node.js
        if: github.ref_type == 'tag'
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: crates/ez-vend-app/package-lock.json

      - name: Restore Rust cache
        if: github.ref_type == 'tag'
        uses: Swatinem/rust-cache@v2

      - name: Install Trunk
        if: github.ref_type == 'tag'
        uses: taiki-e/install-action@v2
        with:
          tool: trunk

      - name: Install frontend dependencies
        if: github.ref_type == 'tag'
        run: npm ci
        working-directory: crates/ez-vend-app

      - name: Build Kassen-App WASM
        if: github.ref_type == 'tag'
        env:
          LABELS_PUBLIC_URL: https://tschuba.github.io/ez-vend/labels/
        run: trunk build --release --public-url /ez-vend/pos/
        working-directory: crates/ez-vend-app

      - name: Merge WASM bundle into site
        if: github.ref_type == 'tag'
        run: |
          mkdir -p _site/pos
          cp -R crates/ez-vend-app/dist/. _site/pos/

      - name: Upload Pages artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: ./_site

  deploy:
    name: Deploy Pages Site
    runs-on: ubuntu-latest
    needs: build
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}

    steps:
      - name: Deploy to GitHub Pages
        id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 3: Validate YAML syntax**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/deploy-pages.yml'))" && echo "YAML valid"
```

Expected: `YAML valid`

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/deploy-pages.yml
git commit -m "ci: add deploy-pages workflow replacing docs.yml — deploys Jekyll on main push, adds WASM on release tags"
```

---

## Task 3: Verify WASM build with public-url override

This task verifies the trunk CLI flag works locally before relying on CI.

- [ ] **Step 1: Run trunk build with public-url override**

```bash
cd crates/ez-vend-app
LABELS_PUBLIC_URL=https://tschuba.github.io/ez-vend/labels/ \
  trunk build --release --public-url /ez-vend/pos/
```

Expected: builds successfully. Output in `crates/ez-vend-app/dist/`.

- [ ] **Step 2: Verify asset paths in generated index.html**

```bash
grep -o 'src="[^"]*"' crates/ez-vend-app/dist/index.html | head -5
```

Expected: all asset `src` attributes begin with `/ez-vend/pos/`, e.g.:

```
src="/ez-vend/pos/ez-vend-app-abc123.js"
```

- [ ] **Step 3: Verify LABELS_PUBLIC_URL is embedded**

```bash
strings crates/ez-vend-app/dist/*.wasm | grep -c 'tschuba.github.io'
```

Expected: `1` (the URL is compiled into the WASM binary).

- [ ] **Step 4: Clean dist to avoid committing build artifacts**

```bash
rm -rf crates/ez-vend-app/dist
```

- [ ] **Step 5: Commit verification note**

No code changes in this task. If all checks passed, continue to Task 4.

---

## Task 4: Confirm GitHub Pages environment configuration

This task is a manual pre-flight check — no code changes.

- [ ] **Step 1: Verify GitHub Pages is enabled in the repository**

In the GitHub repository → Settings → Pages:
- Source: **GitHub Actions** (not "Deploy from a branch")
- If it shows "Deploy from a branch", switch to "GitHub Actions" — this enables the `actions/deploy-pages` workflow approach.

- [ ] **Step 2: Verify the `pages` concurrency group is not used by any other active workflow**

```bash
grep -r 'group: pages' .github/workflows/
```

Expected: only `deploy-pages.yml` appears. If `docs.yml` was not fully removed, this would show two matches — a conflict.

- [ ] **Step 3: Push branch and open PR**

```bash
git push -u origin feat/static-hosting-github-pages
```

Then open a PR from `feat/static-hosting-github-pages` → `main` in GitHub. CI will run the build job but NOT the deploy job (deploy only runs on the `github-pages` environment, which requires the concurrency group — in a PR context this is safe to skip).

---

## Known Gaps (out of scope for this plan)

These items are tracked for future phases and explicitly excluded here:

| Gap | Phase | Notes |
|-----|-------|-------|
| Label-App WASM (`ez-vend-labels`) | Phase 1 | Crate does not exist yet. Add build step to deploy-pages.yml when crate is created: `trunk build --release --public-url /ez-vend/labels/`, output to `_site/labels/`. |
| Mobile-App WASM (`ez-vend-mobile`) | Phase 3 | Crate does not exist yet. Add build step: `--public-url /ez-vend/mobile/`. Also add sw.js `__VERSION__` → `$GITHUB_REF_NAME` substitution via `sed` pre-build hook before the trunk build step. |
| Organizer-App (`ez-vend-organizer`) | Phase 5 | Hosting path not yet decided. Not included. |
| Release workflow WASM builds | — | `release.yml` builds WASM for the downloadable artifact (no `--public-url` override needed — runs at `"./"` for local launcher use). Do NOT add `--public-url` there; that build is separate from the Pages deploy. |
