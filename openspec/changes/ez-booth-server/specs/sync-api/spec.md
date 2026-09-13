## ADDED Requirements

### Requirement: Purchase batch upload
The server SHALL accept `POST /api/sync` with a JSON body `{ purchases: [...], client_id: "<uuid>" }`. Each purchase object MUST include `id` (UUID), `vendor_id`, `price_cents`, `occurred_at`, and `event_code`. The server MUST insert accepted purchases using `ON CONFLICT DO NOTHING` based on the dedup index on `entity_id`. The server MUST return HTTP 200 with `{ accepted: <count> }` on success. In builds compiled with `--features multi-tenant`, `/api/sync` MUST require a valid `AuthContext` (any role) so the target tenant schema can be resolved from the key; in single-tenant builds, whether `/api/sync` requires auth is deferred per `design.md`'s Open Questions.

#### Scenario: Upload new purchases
- **WHEN** a mobile client POSTs 5 purchases with unique UUIDs
- **THEN** the server returns HTTP 200 with `{ "accepted": 5 }` and all 5 are stored

#### Scenario: Upload with duplicate purchases
- **WHEN** a client POSTs 5 purchases of which 3 were already uploaded in a previous request
- **THEN** the server returns HTTP 200 with `{ "accepted": 2 }` and the 3 duplicates are silently skipped

#### Scenario: Upload with malformed body
- **WHEN** a client POSTs a body missing the `purchases` field
- **THEN** the server returns HTTP 400 with a descriptive error message

#### Scenario: Upload empty purchase list
- **WHEN** a client POSTs `{ purchases: [], client_id: "..." }`
- **THEN** the server returns HTTP 200 with `{ "accepted": 0 }`

#### Scenario: Multi-tenant upload without auth
- **WHEN** the server is built with `--features multi-tenant` and a client POSTs to `/api/sync` without a valid `Authorization` header
- **THEN** the server returns HTTP 401, since tenant schema resolution requires an `AuthContext`

---

### Requirement: Paginated purchase download
The server SHALL accept `GET /api/sync?since={sequence}` and return up to 500 `purchase_upserted` events with `sequence > since`. The response MUST include `{ purchases: [...], next_sequence: <n> }`. When `purchases` is non-empty the server MUST guarantee `next_sequence > since`. The client loops while `purchases.length == 500`, up to 20 iterations.

#### Scenario: Download with available purchases
- **WHEN** a Kassen-App GETs `/api/sync?since=0` and 12 purchases exist on the server
- **THEN** the server returns all 12 purchases and `next_sequence` greater than 0

#### Scenario: Download at page boundary
- **WHEN** 500 purchases exist with sequences > the `since` cursor
- **THEN** the server returns exactly 500 purchases, allowing the client to detect more pages exist

#### Scenario: Download beyond last page
- **WHEN** a Kassen-App GETs with a `since` value equal to the highest stored sequence
- **THEN** the server returns `{ "purchases": [], "next_sequence": <since> }`

#### Scenario: Missing since parameter
- **WHEN** a client GETs `/api/sync` without the `since` query parameter
- **THEN** the server returns HTTP 400 with a descriptive error message

---

### Requirement: Purchase deduplication index
The server database MUST maintain a partial unique index on `events(entity_id)` where `event_type = 'purchase_upserted'`. This index enforces idempotency for `POST /api/sync` via `ON CONFLICT DO NOTHING`.

#### Scenario: Concurrent duplicate uploads
- **WHEN** two clients simultaneously POST the same purchase UUID
- **THEN** exactly one record is stored and both requests return HTTP 200 without error

---

### Requirement: Client collision warning
The server MUST log a warning (not return an error) when two different `client_id` values POST purchases with the same `event_code` within the same calendar month.

#### Scenario: Two clients with same event_code
- **WHEN** client A and client B both POST purchases with `event_code = "FM-0526"` in the same month
- **THEN** the server logs a warning and continues accepting purchases from both clients

---

### Requirement: Health check endpoint
The server SHALL expose `GET /health` returning HTTP 200 with `{ "status": "ok" }` when the server is running and the database connection is healthy.

#### Scenario: Server healthy
- **WHEN** a load balancer or Coolify health check calls `GET /health`
- **THEN** the server returns HTTP 200 with `{ "status": "ok" }`

#### Scenario: Database unreachable
- **WHEN** the database connection is lost and `GET /health` is called
- **THEN** the server returns HTTP 503 with `{ "status": "degraded", "detail": "database unavailable" }`
