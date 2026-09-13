---
title: ADR Static Hosting on GitHub Pages
nav_order: 5
parent: Technical Docs
---

Date: 2026-05-18
Status: Accepted
Decision Maker: ez-vend-rs maintainers

## Context

ez-booth supports three deployment scenarios: local-only (launcher or browser on the organizer's machine), static hosting (WASM bundles on a public host), and server-backed (optional Coolify instance for live sync). This ADR covers the second scenario.

Static hosting on GitHub Pages is the most accessible entry point for organizers. It requires no server, no Docker installation, and no custom domain — only a GitHub account. It is also the natural home for the Label-App: vendors must be able to open a stable URL on their own device before the event to configure and print their QR stickers. That URL is embedded in printed QR codes and must remain constant across events and device types. A locally served Label-App cannot satisfy this requirement for most organizers.

All three WASM crates — Kassen-App (point-of-sale register), Label-App (vendor label printing), and Mobile-App (helper scanning) — are published to GitHub Pages as static assets. No server-side logic is deployed in this scenario. Features that require a backend remain available when the organizer additionally operates a Coolify instance; this ADR does not cover that path.

---

## Deployment URLs

All three apps are published as subdirectories of the same GitHub Pages site that hosts the project documentation:

| App | Crate | Public URL |
| --- | --- | --- |
| Kassen-App | `crates/ez-vend-app` | `https://tschuba.github.io/ez-vend-rs/pos/` |
| Label-App | `crates/ez-vend-labels` | `https://tschuba.github.io/ez-vend-rs/labels/` |
| Mobile-App | `crates/ez-vend-mobile` | `https://tschuba.github.io/ez-vend-rs/mobile/` |

The root path (`https://tschuba.github.io/ez-vend-rs/`) continues to serve the Jekyll documentation site (unchanged). The Jekyll output and the three WASM builds are merged into the `gh-pages` branch under separate subdirectories with no conflicts.

Each app is a Single Page Application. GitHub Pages serves a static `404.html` redirect to `index.html` for client-side routing; `trunk` generates this automatically when `--public-url` is set.

The Organizer-App (Phase 5, `crates/ez-vend-organizer`) is not included in this ADR. Its hosting path will be decided when Phase 5 is scoped.

---

## Build Pipeline

### Trigger

The GitHub Actions deploy workflow fires on **published releases** (tag pattern `v*.*.*`). Each release tag produces a complete, versioned deploy of all three apps. No continuous deployment on `main` — only explicit version tags go live.

### Per-app build

Each WASM crate is built with `trunk build --release --public-url /ez-vend-rs/{path}/`:

```text
trunk build --release --public-url /ez-vend-rs/pos/      # Kassen-App
trunk build --release --public-url /ez-vend-rs/labels/   # Label-App
trunk build --release --public-url /ez-vend-rs/mobile/   # Mobile-App
```

The `--public-url` flag is passed by the CI workflow and overrides the default `public_url = "./"` in each crate's `Trunk.toml`. Local development continues to use `"./"` unchanged — no `Trunk.toml` modifications are required.

Build outputs go to `dist/pos/`, `dist/labels/`, and `dist/mobile/` respectively.

### Label-App URL — baked at build time

The Label-App URL is injected as a Cargo environment variable at build time:

```text
LABELS_PUBLIC_URL=https://tschuba.github.io/ez-vend-rs/labels/
```

The Kassen-App reads this at compile time via `env!("LABELS_PUBLIC_URL")`, defined in `crates/ez-vend-app/src/config.rs`. No runtime configuration or settings input is required for the standard GitHub Pages deployment. Local operators who cannot use the GitHub Pages Label-App URL must build and deploy the Label-App themselves and pass a different value for this variable at build time.

### Publishing

The three `dist/` outputs are merged with the Jekyll documentation build and pushed to the `gh-pages` branch via `JamesIves/github-pages-deploy-action` (or equivalent). The workflow:

1. Builds the Jekyll docs site → `_site/`
2. Builds all three WASM crates → `dist/pos/`, `dist/labels/`, `dist/mobile/`
3. Copies WASM outputs into `_site/pos/`, `_site/labels/`, `_site/mobile/`
4. Pushes `_site/` to `gh-pages`

### Service Worker cache versioning

The Mobile-App Service Worker uses the release tag as its cache name:

```js
const CACHE_NAME = 'ez-vend-v__VERSION__';
```

`__VERSION__` is replaced by a `trunk` pre-build hook using `sed` with the `RELEASE_TAG` environment variable set by the GitHub Actions workflow (e.g. `sed -i "s/__VERSION__/$RELEASE_TAG/g" public/sw.js`). Deploying a new tag immediately invalidates old SW caches for helpers who open the Mobile-App, ensuring they receive the updated WASM bundle on next visit.

---

## Feature Availability

### What works in static hosting

| Feature | Notes |
| --- | --- |
| Vendor label printing (Label-App) | Core use case for this deployment mode |
| QR code scan at register via webcam (Phase 2) | Client-side only (`rxing` / `ZXing-wasm`) |
| Handwritten label OCR — client-side Tesseract.js (Phase 4) | Runs entirely in WASM, no server needed |
| Mobile-App: scan items, file export | Offline-first; export via `navigator.share` or `<a download>` |
| Kassen-App: file import from mobile | JSON file import with UUID deduplication |
| Kassen-App: purchase settlement and reports | All data lives in IndexedDB locally |

### What requires a Coolify server

| Feature | Notes |
| --- | --- |
| `POST /api/sync` — mobile upload | Server endpoint, Coolify only |
| `GET /api/sync` — register download | Server endpoint, Coolify only |
| Handwritten label OCR — server-side offload (Phase 4) | Optional; client-side path remains available |

### Mode detection

There is no compile-time flag or separate build for "static hosting mode." The same WASM bundle runs in all deployment contexts. The Kassen-App and Mobile-App detect the available feature set at runtime based on the presence of a configured server URL in settings:

- **No server URL configured:** Sync button is disabled with tooltip *"Server-Sync nicht verfügbar — Dateiexport verwenden."* File export/import path is always visible.
- **Server URL configured:** Sync button is enabled. Server sync and file sync coexist; both use the same UUID deduplication.

This approach keeps the WASM bundle identical across deployment scenarios, simplifies testing, and makes it trivial for an organizer to add a Coolify server to an existing static deployment without changing the frontend.

---

## Consequences

### Positive

- Zero hosting cost — GitHub Pages is free for public repositories.
- The Label-App URL is stable across events; vendors receive a single link that does not change between flea markets.
- One pipeline publishes all three apps atomically — a single release tag produces a consistent, versioned deploy.
- WASM bundles are served via GitHub Pages CDN and cached by Service Workers — subsequent loads are near-instant for returning users.
- No infrastructure to maintain between events.

### Negative / Trade-offs

- `LABELS_PUBLIC_URL` is baked at build time. Local operators running the app on `localhost` cannot use the GitHub Pages Label-App URL for label link generation; they must build and deploy the Label-App separately and override the env var. This is a deliberate trade-off: the majority of organizers use the hosted deployment, and a configurable URL would require a settings field, UI copy, and documentation for a case that applies only to self-hosted-from-source operators.
- GitHub Pages requires a public repository (or GitHub Pro/Team for private repos with Pages). Organizers with private forks must use an alternative hosting platform or configure a custom Pages domain.
- SPA routing on GitHub Pages requires a `404.html` redirect to `index.html`. Trunk generates this correctly with `--public-url`, but direct URL access (e.g., a deep link to a specific route) triggers one extra redirect before the app loads.
- The `LABELS_PUBLIC_URL` constant in the Kassen-App is the only hardcoded external URL in the codebase. Future hostname changes (repository rename, GitHub account transfer) require a rebuild and redeploy.

---

## Affected Files

| File | Change |
| --- | --- |
| `.github/workflows/deploy.yml` | New — build all three WASM crates (trunk), merge with Jekyll docs output, publish to `gh-pages` on release tag |
| `crates/ez-vend-app/Trunk.toml` | `public_url = "/ez-vend-rs/pos/"` |
| `crates/ez-vend-labels/Trunk.toml` | `public_url = "/ez-vend-rs/labels/"` |
| `crates/ez-vend-mobile/Trunk.toml` | `public_url = "/ez-vend-rs/mobile/"` |
| `crates/ez-vend-app/src/config.rs` | New — `pub const LABELS_PUBLIC_URL: &str = env!("LABELS_PUBLIC_URL");` read by Kassen-App when generating label links |
| `crates/ez-vend-mobile/public/sw.js` | Cache name uses release tag: `const CACHE_NAME = 'ez-vend-v__VERSION__'` — substituted at build time |
