## Why

The mobile sync and multi-device coordination features planned in `qr-labels-webcam-mobile-sync` require a server-side component, but burying infrastructure inside a scanning feature creates the wrong ownership boundary. `ez-vend-server` is foundational — it will be reused by handwriting OCR (Phase 4), the Organiser-App (Phase 5), and any future capability that needs persistence beyond a single browser — and it carries a distinct business model dimension (free self-hosted vs. paid managed instances) that deserves its own design scope.

## What Changes

- **New crate `ez-vend-server`** — standalone native Rust server built on `axum`; provides the sync API, auth layer, and CLI key management tooling
- **New workspace entry** — `crates/ez-vend-server` added to `Cargo.toml`; compiled separately from WASM crates; not included in any browser bundle
- **Sync API** — `POST /api/sync` (batch purchase upload) and `GET /api/sync?since=` (paginated download); same wire contracts as defined in `qr-labels-webcam-mobile-sync` §11
- **API key management** — CLI-driven key lifecycle (`key create`, `key pair`, `key revoke`, `key list`); pairing-code flow embeds server URL in QR so devices self-configure on scan
- **Auth layer** — env-var-activated, layered: disabled → API key (DB-backed) → OIDC (Authentik reference); route handlers are auth-agnostic via `AuthContext` middleware
- **Optional multi-tenancy** — compile-time feature flag `--features multi-tenant`; schema-per-tenant isolation on Postgres; single-tenant mode works with SQLite or Postgres, no schema concept
- **Docker distribution** — primary distribution as a Docker image (`ghcr.io`); Docker Compose templates for SQLite-only and Postgres variants
- **Deployment model** — API-only (no static file serving); frontend stays on GitHub Pages; `CORS_ALLOWED_ORIGINS` env var; `DATABASE_URL` selects DB; offline-first guarantee preserved (server is optional, sync features hidden when no server URL is configured)

## Capabilities

### New Capabilities

- `sync-api`: `POST /api/sync` and `GET /api/sync?since=` endpoints — batch purchase upload from mobile, paginated download to Kassen-App; idempotent via UUID dedup index; server-side `event_code` collision warning
- `server-auth`: Layered auth resolved by middleware to `AuthContext { NoAuth | ApiKey | OidcClaims }` with `AuthRole { Organiser | Cashier }`; OIDC activated by `OIDC_ISSUER_URL` env var (Authentik as reference implementation); no-auth mode grants implicit Organiser trust
- `api-key-management`: DB-backed API key lifecycle (create, pair, revoke, list); separated key-identity from key-distribution; pairing-code QR flow for non-technical onboarding; `--direct` opt-out for full-key QR; `last_used_at` tracking; key labels for operator identification
- `tenant-provisioning`: Multi-tenant mode (`--features multi-tenant`); schema-per-tenant isolation on Postgres; CLI commands `tenant create/list/delete`; per-tenant key management; startup validation (Postgres required, auth required)

### Modified Capabilities

<!-- No existing specs — all capabilities above are new -->

## Impact

**Code**
- New crate: `crates/ez-vend-server/` (axum, sqlx, clap, tokio)
- `Cargo.toml` workspace — new member `crates/ez-vend-server`
- `crates/ez-vend-server/migrations/0001_initial.sql` — events, api_keys, pairing_codes tables (includes `event_code` directly; no separate migration needed)
- `crates/ez-vend-server/migrations/0002_purchase_dedup_index.sql` — moved from `qr-labels-webcam-mobile-sync` tasks 2.1

**Dependencies (new, server-only)**
- `axum` — HTTP framework
- `sqlx` — async DB layer (SQLite + Postgres features)
- `clap` — CLI argument parsing
- `tower` / `tower-http` — middleware (CORS, logging, tracing)
- `jsonwebtoken` — JWT validation for OIDC
- `qrcode` — terminal QR output for `key pair` (reuses crate already planned in `qr-labels`)

**Deployment**
- Docker image: `ghcr.io/tschuba/ez-vend-server`
- New CI workflow: `server.yml` (separate from WASM build); `Server Build` required check; `server-v*` tag triggers image build and push

**Relationship to `qr-labels-webcam-mobile-sync`**
- Task 2.1 (dedup index migration) and tasks 6.1–6.4 from that change move here. Task 2.2 (`ALTER TABLE booths ADD COLUMN event_code`) is dropped, not moved — `events.event_code` already ships as part of this change's migration 0001, and there is no `booths` table in this schema
- That change retains client-side sync UI (tasks 6.5–6.7) and file import/export
- `qr-labels-webcam-mobile-sync` must list `ez-vend-server` as a prerequisite
