## Context

`ez-vend` is a Leptos/WASM offline-first flea-market checkout application. All data today lives in browser storage (IndexedDB, localStorage). The `qr-labels-webcam-mobile-sync` change introduces mobile helpers scanning purchases on phones — those purchases need a path onto the Kassen-App registers. A server component enables batch sync while preserving the offline-first guarantee: the server is optional, and the core checkout flow works without it at all times.

This crate is intentionally separated from the scanning feature that first motivated it. It is foundational infrastructure that will also serve handwriting OCR fallback (Phase 4), the Organiser-App (Phase 5), and any future capability requiring server-side persistence.

**Constraints that must not be violated:**
- Offline-first: core checkout works without the server at all times
- Server is additive — sync features are hidden/disabled when no server URL is configured in the app
- Single-organiser integrity: `event_code` uniqueness is not enforced server-side; organisers cross-check totals before vendor payout
- Multi-tenant mode is a compile-time opt-in, not available in self-hosted open-source builds without recompilation

---

## Goals / Non-Goals

**Goals:**
- Provide `POST /api/sync` and `GET /api/sync` endpoints for mobile purchase batch sync
- Layered, env-var-activated auth (disabled → API key → OIDC) with route handlers that are auth-agnostic
- DB-backed API key management with pairing-code distribution flow suitable for non-technical operators
- Schema-per-tenant isolation for multi-tenant Postgres deployments (compile-time feature flag)
- CLI provisioning surface extensible to HTTP without refactor
- Docker image as primary distribution; offline-first guarantee preserved

**Non-Goals:**
- Serving static WASM frontend files (stays on GitHub Pages / static host)
- Real-time relay or WebSocket push between devices
- Multi-tenant shared deployments in the open-source build
- PDF generation, email delivery, payment processing
- Mobile app client-side code (stays in `qr-labels-webcam-mobile-sync`)
- Authentication for the mobile sync endpoint (parked — resolved when adapting `qr-labels` change)

---

## Decisions

### §1 — Framework: axum over warp

**Decision:** Use `axum` (tokio-native, tower ecosystem).

The launcher already uses `warp`, but the server is a separate binary with no shared dependency. `axum` has a more active maintenance posture, first-class `tower` middleware composability (CORS, tracing, request ID, timeout all drop in cleanly), and better async ergonomics for the handler signatures this server needs. No existing code is affected.

*Alternative considered:* `warp` — rejected; less maintained, middleware story is weaker for multi-layer auth.

---

### §2 — Database: SQLite default, Postgres for multi-tenant

**Decision:** `DATABASE_URL` env var selects the DB. SQLite is the default for single-tenant self-hosted. Postgres is required for multi-tenant (enforced at startup with a clear error).

`sqlx` handles both backends transparently via feature flags. A single-tenant organiser running the server locally gets a zero-dependency setup (`sqlite:///data/ez-booth.db`). Postgres is the right choice for managed multi-tenant due to schema isolation and concurrent write semantics.

```
DATABASE_URL=sqlite:///data/ez-booth.db   ← self-hosted default
DATABASE_URL=postgres://...               ← managed / Coolify Postgres
```

*Alternative considered:* Postgres-only — rejected; adds operational overhead for self-hosters who don't need it.

---

### §3 — Tenant isolation: schema-per-tenant (Postgres, multi-tenant only)

**Decision:** Each organiser gets a dedicated Postgres schema. Middleware sets `SET search_path TO tenant_{uuid}` per request. Route handlers have no tenant awareness.

```
public.tenants           ← tenant registry (multi-tenant mode only)
tenant_{uuid}.events     ← per-organiser purchase events
tenant_{uuid}.api_keys   ← per-organiser API keys
tenant_{uuid}.pairing_codes ← per-organiser pairing codes
```

Provisioning = `CREATE SCHEMA` + `sqlx::migrate!().run()` in that schema — identical to a fresh single-tenant install. Dropping a tenant = `DROP SCHEMA CASCADE`. Route handlers query `events` with no tenant filter; isolation is structural, not conditional.

Single-tenant mode has no schema concept. Migrations run against the default schema. Middleware is a no-op.

*Alternatives considered:*
- Row-level `tenant_id` — works on SQLite too, but requires every query to filter correctly; one missing `WHERE` leaks cross-tenant data.
- Row-Level Security (RLS) — elegant but Postgres-only, adds session variable ceremony, harder to test.
- Schema-per-tenant wins: leak-proof by structure, clean tenant drop, self-hosted and managed share migration files.

---

### §4 — Multi-tenancy gate: compile-time feature flag

**Decision:** `--features multi-tenant` activates schema routing middleware, startup Postgres validation, and CLI `tenant` subcommands. Absent from the default open-source build.

```toml
[features]
default = []
multi-tenant = []
```

A feature flag means multi-tenancy cannot be accidentally activated by configuration; it requires a deliberate custom build. This is the gate for the paid managed offering.

*Alternative considered:* runtime env var (`MULTI_TENANT=true`) — weaker gate; anyone could set it on a self-hosted instance. Compile-time is stronger.

---

### §5 — Auth: layered, env-var-activated

**Decision:** Auth mode is determined by which env vars are present at startup.

```
OIDC_ISSUER_URL set   → OIDC middleware active
API_KEY mode          → keys are DB-backed, no env var needed (presence of key records implies API key mode)
Neither               → auth disabled; all requests treated as implicit Organiser
Both                  → startup error
```

Route handlers receive `AuthContext` extracted by middleware; they never inspect auth mechanism:

```rust
enum AuthContext {
    NoAuth,
    ApiKey { key_id, tenant_id, role: AuthRole },
    OidcClaims { sub, tenant_id, role: AuthRole },
}

enum AuthRole { Organiser, Cashier }
```

`NoAuth` passes all role guards — full trust for operators running without auth.

Multi-tenant mode fails at startup if neither OIDC nor any API keys exist (auth is required when tenants exist).

**OIDC (Authentik reference implementation):** Server fetches `{OIDC_ISSUER_URL}/.well-known/openid-configuration` at startup, caches JWKS, validates `Authorization: Bearer <jwt>` on each request. Claims-to-role mapping: Authentik groups → `AuthRole`. `OIDC_ISSUER_URL` is a fully qualified URL — server is topology-agnostic (Authentik can be internal or external, same config).

*Alternative considered:* Hard-coded auth modes via `AUTH_MODE=none|api-key|oidc` env var — more explicit, but redundant; presence of `OIDC_ISSUER_URL` already signals intent clearly.

---

### §6 — API key lifecycle: separated create / pair

**Decision:** Key identity (create, revoke) and key distribution (pair) are decoupled commands.

```
key create --role <r> --label <l>   → stores key hash in DB, no distribution token
key pair   --id <id> [--direct]     → generates pairing code + QR (repeatable)
key list                             → ID, role, label, status, last_used_at
key revoke --id <id>                → immediate; invalidates outstanding pairing codes
```

Rationale: an organiser may prepare keys days before an event and distribute them at the door. A cashier who loses their device needs a new pairing code for the existing key — `key pair` is repeatable for exactly this case. Pairing codes are short-lived (15 min TTL), single-use, and do not expose the key value in the QR.

Default `key pair` output:
```
ez-booth://pair?s={server_url}&c={code}
```
Server URL is embedded in the QR — scanning IS device onboarding. No prior server configuration required on the target device.

`--direct` flag emits a full-key QR (`ez-booth://setup?s=...&k=...`) with a printed warning. Opt-out for operators who accept the security trade-off.

App also accepts 6-digit numeric manual entry as a fallback — consistent with event onboarding pattern.

Keys are always DB-backed (SQLite or Postgres). Revocation is immediate without server restart.

**Rate limiting on `/api/pair`:** A 6-digit code has only 10^6 possibilities, the endpoint is unauthenticated, multiple outstanding codes per key are permitted, and a successful guess returns a full plaintext key — so `/api/pair` MUST be rate-limited (e.g. a per-IP attempt counter, returning HTTP 429 after a small number of failed attempts within a short window) to prevent brute-forcing a code inside its 15-minute TTL. This is enforced independently of the 15-minute TTL and single-use marking, which alone are not sufficient against an unthrottled guesser.

---

### §7 — CLI structure: service layer over adapters

**Decision:** Provisioning logic lives in a service module; CLI (and future HTTP) are thin adapters calling into it.

```
crates/ez-vend-server/src/
├── cli/          ← clap definitions, calls service functions
├── services/
│   ├── key.rs    ← create, pair, revoke, list — business logic here
│   └── tenant.rs ← create, delete, list (multi-tenant feature only)
├── routes/       ← axum handlers, call service functions
└── ...
```

Adding an HTTP provisioning endpoint later = new adapter in `routes/`, no service changes.

---

### §8 — Migration strategy

**Decision:** `sqlx migrate` with numbered SQL files in `crates/ez-vend-server/migrations/`.

```
0001_initial.sql          ← events, api_keys, pairing_codes (event_code ships as part of this table)
0002_purchase_dedup_index.sql  ← partial unique index for ON CONFLICT DO NOTHING
```

Migrations run at startup (`sqlx::migrate!().run(&pool)`). For multi-tenant, tenant provisioning runs the same migration set against the new schema before activating it.

Note: `event_code` is declared `NOT NULL` directly in the 0001 `events` table below — there is no separate backfill migration for it. (`qr-labels-webcam-mobile-sync` task 2.2, which alters a `booths` table to add `event_code`, does not carry over here; that table doesn't exist in this schema and the column is already covered by 0001. See proposal.md's relationship section.)

**Initial schema (0001):**
```sql
CREATE TABLE events (
    id           TEXT PRIMARY KEY,
    entity_id    TEXT NOT NULL,        -- UUID, dedup key
    event_type   TEXT NOT NULL,        -- e.g. 'purchase_upserted'
    payload      TEXT NOT NULL,        -- JSON
    sequence     INTEGER NOT NULL,     -- autoincrement via ROWID / SEQUENCE
    client_id    TEXT NOT NULL,
    event_code   TEXT NOT NULL,
    created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE api_keys (
    id           TEXT PRIMARY KEY,     -- short auto-generated, e.g. "kf7x2p9q"
    key_hash     TEXT NOT NULL,        -- bcrypt/argon2 hash; plain value shown once at create
    tenant_id    TEXT NOT NULL,
    role         TEXT NOT NULL,        -- 'organiser' | 'cashier'
    label        TEXT,
    created_at   TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP,
    revoked_at   TIMESTAMP             -- NULL = active
);

CREATE TABLE pairing_codes (
    code         TEXT PRIMARY KEY,     -- 6-digit numeric
    key_id       TEXT NOT NULL REFERENCES api_keys(id),
    expires_at   TIMESTAMP NOT NULL,
    used_at      TIMESTAMP             -- NULL = not yet used; single-use enforced
);
```

*Postgres note:* `sequence` uses a `BIGSERIAL` column; SQLite uses `INTEGER PRIMARY KEY AUTOINCREMENT` pattern via a separate `sequences` table or `ROWID`.

---

### §9 — Deployment

**Decision:** API-only server (no static file serving). Docker image is the primary distribution.

Frontend (WASM bundle) stays on GitHub Pages or any static host. Server and frontend are separate origins; `CORS_ALLOWED_ORIGINS` env var restricts allowed origins. Offline-first is preserved: app functions fully without the server; sync features are hidden when no server URL is configured in Kassen-App settings.

**Frontend coupling:** Server URL is a setting in the Kassen-App settings page (blank = offline-only mode). Mobile-App learns the URL from the onboarding QR (`ez-booth://onboard?...&s={server_url_encoded}`). No app code changes are required to deploy the server — it is additive.

**Coolify topology (managed multi-tenant):**
```
Coolify instance
├── ez-vend-server (Docker, --features multi-tenant)
│   ├── DATABASE_URL → Postgres service (internal network)
│   └── OIDC_ISSUER_URL → Authentik (internal or external URL)
└── Postgres service (Coolify managed)
```

Authentik is topology-agnostic — `OIDC_ISSUER_URL` is always a fully qualified URL regardless of whether Authentik is on the same Coolify instance or a separate server.

**CI (parked detail):** Separate `server.yml` workflow; `Server Build` required check scoped to `crates/ez-vend-server/**`; `server-v*` tags trigger Docker build + push to `ghcr.io`. Independent release cadence from WASM frontend.

---

## Risks / Trade-offs

| Risk | Mitigation |
|---|---|
| Unauthenticated single-tenant sync — anyone who can reach the server can POST purchases | Offline-first means the server is never required; organisers who need isolation keep auth enabled. Document: organisers must cross-check server-synced totals against register totals before payout. |
| `event_code` collision silently interleaves financial data across two organisers | Deployment constraint: one organiser per single-tenant instance. Server logs a warning when two different `client_id` values POST with the same `event_code` in the same calendar month. |
| Schema-per-tenant complicates connection pooling (`SET search_path` is session-level) | Set `search_path` at the start of every transaction (not connection), ensuring pool connections are safe to reuse across tenants. |
| JWKS cache invalidation if OIDC provider rotates keys | Refresh JWKS on 401 from signature validation; cap refresh rate to avoid hammering the provider. |
| `key pair --direct` leaks the key value in QR if photographed | Print a prominent warning; make pairing-code flow the default; document the security trade-off clearly. |
| Pairing codes expire during slow event setup | 15-minute TTL is generous for in-person setup; `key pair` can be rerun immediately to generate a fresh code. |

---

## Migration Plan

The `ez-vend-server` crate is a new binary with no migration impact on existing WASM deployments. Deployment steps:

1. Build Docker image (single-tenant: `cargo build --release`; multi-tenant: `--features multi-tenant`)
2. Set `DATABASE_URL` (SQLite path or Postgres URL)
3. Run server — migrations apply automatically at startup
4. Create organiser key: `ez-vend-server key create --role organiser --label "Organiser"`
5. Generate pairing QR: `ez-vend-server key pair --id <id>`
6. Scan QR on Kassen-App → server URL and key auto-configured
7. Configure `CORS_ALLOWED_ORIGINS` to match the app's GitHub Pages URL

Rollback: the server holds no data that is authoritative — all purchase data exists independently in each device's IndexedDB. Stopping the server reverts the app to offline-only mode without data loss.

---

## Open Questions

- **Mobile sync auth (single-tenant only):** In single-tenant/self-hosted mode, should `POST /api/sync` from `ez-vend-mobile` require an API key (cashier role), or remain unauthenticated? There is no tenant ambiguity in single-tenant mode, so either option is structurally valid. Deferred — resolved when adapting `qr-labels-webcam-mobile-sync` change.
  In multi-tenant mode this is **not** open: `/api/sync` must use API-key auth. Schema routing (§3) resolves the target tenant schema from the key's `tenant_id` in `AuthContext`; with no `AuthContext` there is no input to route on, and `event_code` (a free-text, collision-prone field — see Risks) cannot stand in for it. Multi-tenant mode already requires auth at startup (§5), so this falls out of decisions made elsewhere in this document rather than needing a separate choice.
- **`sequence` portability:** SQLite lacks `SEQUENCE`. Best approach for cross-DB compatible monotonic sequence? Options: `ROWID`-based, application-level via atomic counter, or a `sequences` auxiliary table.
- **Key hash algorithm:** `bcrypt` or `argon2`? `argon2` is recommended for new systems; needs a Rust crate (`argon2` crate). Confirm acceptable latency for per-request key hash verification.
