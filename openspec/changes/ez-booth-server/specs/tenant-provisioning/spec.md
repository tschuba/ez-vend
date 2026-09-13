## ADDED Requirements

### Requirement: Multi-tenant feature flag
The server MUST only expose multi-tenant capabilities when compiled with `--features multi-tenant`. Single-tenant builds MUST NOT include tenant CLI subcommands, schema routing middleware, or Postgres schema creation logic. Activating multi-tenant features on a SQLite database MUST cause a startup error.

#### Scenario: Multi-tenant mode with Postgres
- **WHEN** the server is built with `--features multi-tenant` and `DATABASE_URL` points to Postgres
- **THEN** schema routing middleware is active and tenant CLI commands are available

#### Scenario: Multi-tenant mode with SQLite
- **WHEN** the server is built with `--features multi-tenant` and `DATABASE_URL` points to SQLite
- **THEN** the server exits at startup with: "Multi-tenant mode requires PostgreSQL. SQLite is not supported for multi-tenant deployments."

---

### Requirement: Tenant creation
The CLI command `tenant create --name <name>` SHALL provision a new tenant. Provisioning MUST: generate a UUID tenant ID, create a Postgres schema named `tenant_{uuid}`, run all database migrations in that schema, and register the tenant in `public.tenants`. The command MUST output the tenant ID on success.

#### Scenario: Create a new tenant
- **WHEN** the operator runs `tenant create --name "Flohmarkt Thomas"`
- **THEN** a schema `tenant_{uuid}` is created, migrations are applied, the tenant is registered, and the tenant ID is printed

#### Scenario: Duplicate tenant name
- **WHEN** the operator runs `tenant create --name "Flohmarkt Thomas"` and a tenant with that name already exists
- **THEN** the CLI exits with a non-zero status and "Tenant with name 'Flohmarkt Thomas' already exists"

---

### Requirement: Tenant listing
The CLI command `tenant list` SHALL display all provisioned tenants with columns: ID, name, and created date.

#### Scenario: List tenants
- **WHEN** the operator runs `tenant list` and 3 tenants exist
- **THEN** all 3 are shown with their ID, name, and creation timestamp

#### Scenario: No tenants exist
- **WHEN** the operator runs `tenant list` and no tenants have been provisioned
- **THEN** the CLI outputs "No tenants provisioned" and exits with status 0

---

### Requirement: Tenant deletion
The CLI command `tenant delete --id <tenant-id>` SHALL remove a tenant by dropping its Postgres schema (`DROP SCHEMA tenant_{uuid} CASCADE`) and removing the tenant record from `public.tenants`. The command MUST require explicit confirmation unless `--force` is passed. Deletion is irreversible.

#### Scenario: Delete tenant with confirmation
- **WHEN** the operator runs `tenant delete --id <uuid>` and confirms the prompt
- **THEN** the schema is dropped, all tenant data is permanently removed, and the tenant is deregistered

#### Scenario: Delete tenant with --force
- **WHEN** the operator runs `tenant delete --id <uuid> --force`
- **THEN** deletion proceeds without a confirmation prompt

#### Scenario: Delete non-existent tenant
- **WHEN** the operator runs `tenant delete --id unknown`
- **THEN** the CLI exits with a non-zero status and "Tenant not found: unknown"

---

### Requirement: Per-tenant schema isolation
In multi-tenant mode, the server SHALL set `search_path` to the requesting tenant's schema at the start of every transaction. Route handlers MUST have no awareness of the tenant schema name. Cross-tenant data access MUST be structurally impossible through the route handler layer.

#### Scenario: Tenant A cannot read Tenant B data
- **WHEN** a request authenticated as Tenant A calls `GET /api/sync?since=0`
- **THEN** only events stored in Tenant A's schema are returned; Tenant B's events are not accessible

#### Scenario: Schema set per transaction not per connection
- **WHEN** a connection pool connection is reused for a Tenant B request after serving a Tenant A request
- **THEN** `search_path` is set to Tenant B's schema at the start of the new transaction before any query executes

---

### Requirement: Per-tenant key management
In multi-tenant mode, CLI key commands MUST require a `--tenant <id>` argument. Keys MUST be scoped to their tenant's schema; a key issued for Tenant A MUST NOT authenticate requests to Tenant B.

#### Scenario: Create key for specific tenant
- **WHEN** the operator runs `tenant key create --tenant <uuid> --role cashier --label "Kasse 1"`
- **THEN** the key is stored in `tenant_{uuid}.api_keys` and is only valid for that tenant's endpoints

#### Scenario: Cross-tenant key rejected
- **WHEN** a request presents Tenant A's API key to an endpoint resolved to Tenant B's schema
- **THEN** the server returns HTTP 401 Unauthorized

---

### Requirement: Tenant provisioning service extensibility
The tenant provisioning logic (create, list, delete) SHALL reside in a dedicated service module (`services/tenant.rs`) callable independently of the CLI. The CLI MUST be a thin adapter over this service. This enables future HTTP-based provisioning endpoints without modifying the service layer.

#### Scenario: Service callable without CLI
- **WHEN** a future HTTP handler calls `tenant_service::create(pool, name)` directly
- **THEN** the same provisioning logic executes as when called via the CLI, with no duplication
