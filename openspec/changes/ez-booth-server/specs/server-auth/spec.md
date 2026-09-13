## ADDED Requirements

### Requirement: Auth mode activation via environment variables
The server SHALL determine its auth mode at startup from the presence of environment variables. If `OIDC_ISSUER_URL` is set, OIDC mode is active. If API keys exist in the database, API key mode is active. If neither is configured, auth is disabled. If both `OIDC_ISSUER_URL` is set AND API keys exist in the database, the server MUST fail at startup with a clear error.

#### Scenario: OIDC mode activated
- **WHEN** the server starts with `OIDC_ISSUER_URL` set
- **THEN** OIDC middleware is active; requests without a valid Bearer token are rejected with HTTP 401

#### Scenario: API key mode active
- **WHEN** the server starts without `OIDC_ISSUER_URL` and API keys exist in the database
- **THEN** API key middleware is active; requests without a valid `Authorization: Bearer <key>` header are rejected with HTTP 401

#### Scenario: Auth disabled
- **WHEN** the server starts without `OIDC_ISSUER_URL` and no API keys in the database
- **THEN** all requests are accepted and treated as implicit Organiser role

#### Scenario: Conflicting auth configuration
- **WHEN** the server starts with `OIDC_ISSUER_URL` set and API keys present in the database
- **THEN** the server exits with a non-zero status code and a clear error: "Configure OIDC or API key auth, not both"

---

### Requirement: Auth-agnostic route handlers
All route handlers SHALL receive an `AuthContext` extracted by middleware and MUST NOT inspect the underlying auth mechanism directly. Role-based access control MUST be enforced via the `AuthContext` alone.

#### Scenario: Organiser-only route with API key auth
- **WHEN** a request with a valid organiser-role API key reaches a route requiring Organiser role
- **THEN** the request is processed successfully

#### Scenario: Cashier key rejected on organiser route
- **WHEN** a request with a valid cashier-role API key reaches a route requiring Organiser role
- **THEN** the server returns HTTP 403 Forbidden

#### Scenario: NoAuth passes all role checks
- **WHEN** auth is disabled and any request reaches any route
- **THEN** the request is processed as if the caller has Organiser role

---

### Requirement: OIDC token validation
When OIDC mode is active, the server SHALL validate `Authorization: Bearer <jwt>` on each request. The server MUST fetch the OIDC discovery document from `{OIDC_ISSUER_URL}/.well-known/openid-configuration` at startup, cache the JWKS endpoint, and validate token signatures against the cached JWKS. The server MUST refresh the JWKS cache on signature validation failure (key rotation), with a rate limit on refreshes to avoid hammering the provider.

#### Scenario: Valid OIDC token
- **WHEN** a request carries a valid, non-expired JWT signed by the configured OIDC provider
- **THEN** the server extracts `AuthContext` with the correct role from the token claims and processes the request

#### Scenario: Expired OIDC token
- **WHEN** a request carries an expired JWT
- **THEN** the server returns HTTP 401 with `WWW-Authenticate: Bearer error="invalid_token"`

#### Scenario: Token signed by unknown key
- **WHEN** a request carries a JWT signed by an unknown key
- **THEN** the server attempts a JWKS refresh once, then returns HTTP 401 if validation still fails

#### Scenario: OIDC provider unreachable at startup
- **WHEN** `OIDC_ISSUER_URL` is set but the discovery endpoint is unreachable at startup
- **THEN** the server exits with a non-zero status and a clear error identifying the unreachable URL

---

### Requirement: OIDC role mapping from Authentik groups
When using Authentik as the OIDC provider, the server SHALL map Authentik group membership to `AuthRole`. The groups claim in the JWT MUST be mapped: membership in `ez-vend-organisers` → `AuthRole::Organiser`; membership in `ez-vend-cashiers` → `AuthRole::Cashier`. Tokens with neither group MUST be rejected with HTTP 403.

#### Scenario: Token with organiser group
- **WHEN** an Authentik JWT contains `groups: ["ez-vend-organisers"]`
- **THEN** the request is processed with `AuthRole::Organiser`

#### Scenario: Token with cashier group
- **WHEN** an Authentik JWT contains `groups: ["ez-vend-cashiers"]`
- **THEN** the request is processed with `AuthRole::Cashier`

#### Scenario: Token with no recognised group
- **WHEN** an Authentik JWT contains no `ez-vend-*` group
- **THEN** the server returns HTTP 403 Forbidden

---

### Requirement: Multi-tenant auth requirement
When compiled with `--features multi-tenant`, the server MUST require auth. If neither `OIDC_ISSUER_URL` is set nor any API keys exist at startup, the server MUST exit with a non-zero status and a clear error.

#### Scenario: Multi-tenant started without auth
- **WHEN** the server is built with `--features multi-tenant` and starts with no auth configuration
- **THEN** the server exits immediately with: "Multi-tenant mode requires authentication. Set OIDC_ISSUER_URL or create API keys."
