## ADDED Requirements

### Requirement: Key creation
The CLI command `key create --role <role> --label <label>` SHALL create a new API key record in the database. The key value MUST be generated as a cryptographically random string, displayed once in plaintext to the operator, and stored only as a hash (argon2). The key MUST be assigned a short auto-generated human-readable ID (e.g., 8 alphanumeric characters). `--role` MUST be one of `organiser` or `cashier`. `--label` is optional but SHOULD be encouraged.

#### Scenario: Create organiser key with label
- **WHEN** the operator runs `key create --role organiser --label "Thomas"`
- **THEN** a key record is stored and the output shows ID, label, role, and the plaintext key value (displayed once only)

#### Scenario: Create cashier key without label
- **WHEN** the operator runs `key create --role cashier`
- **THEN** a key record is created with no label; the output notes the label is absent

#### Scenario: Invalid role value
- **WHEN** the operator runs `key create --role admin`
- **THEN** the CLI exits with a non-zero status and lists valid role values

---

### Requirement: Key listing
The CLI command `key list` SHALL display all non-revoked API keys with columns: ID, role, label, created date, and last used date. Revoked keys MUST NOT appear unless `--include-revoked` is passed.

#### Scenario: List active keys
- **WHEN** the operator runs `key list` and 3 active keys exist
- **THEN** all 3 are shown with ID, role, label, created, and last_used_at (or "never")

#### Scenario: List including revoked
- **WHEN** the operator runs `key list --include-revoked`
- **THEN** revoked keys are shown with a "revoked" status indicator

---

### Requirement: Key revocation
The CLI command `key revoke --id <id>` SHALL mark the key as revoked by setting `revoked_at` to the current timestamp. Revocation MUST take effect immediately — subsequent API requests using the revoked key MUST be rejected with HTTP 401. All outstanding pairing codes for the revoked key MUST be invalidated simultaneously.

#### Scenario: Revoke an active key
- **WHEN** the operator runs `key revoke --id kf7x2p9q`
- **THEN** the key is marked revoked and the CLI confirms: "Revoked: kf7x2p9q (Helper Maria)"

#### Scenario: API request with revoked key
- **WHEN** a client sends a request using a revoked API key
- **THEN** the server returns HTTP 401 immediately (no grace period)

#### Scenario: Pairing codes invalidated on revoke
- **WHEN** a key with an outstanding unexpired pairing code is revoked
- **THEN** a subsequent `POST /api/pair` with that pairing code returns HTTP 410 Gone

#### Scenario: Revoke non-existent key
- **WHEN** the operator runs `key revoke --id unknown`
- **THEN** the CLI exits with a non-zero status and "Key not found: unknown"

---

### Requirement: Pairing code generation
The CLI command `key pair --id <id>` SHALL generate a fresh short-lived pairing code for an existing, non-revoked key. The code MUST be a 6-digit numeric string, unique, expire after 15 minutes, and be single-use. The command MUST output the pairing code as a QR code in the terminal (unicode block characters) encoding `ez-booth://pair?s={server_url}&c={code}`, plus the numeric code for manual entry. Multiple outstanding pairing codes for the same key are permitted.

#### Scenario: Generate pairing code
- **WHEN** the operator runs `key pair --id kf7x2p9q`
- **THEN** a pairing code is stored and the terminal shows a QR code and the 6-digit numeric code

#### Scenario: Repeat pairing for device migration
- **WHEN** a cashier loses their device and the operator runs `key pair --id kf7x2p9q` again
- **THEN** a new pairing code is generated; the previous code (if unused and unexpired) remains valid

#### Scenario: Pair revoked key
- **WHEN** the operator runs `key pair --id` for a revoked key
- **THEN** the CLI exits with a non-zero status: "Cannot generate pairing code for a revoked key"

#### Scenario: Direct key QR opt-out
- **WHEN** the operator runs `key pair --id kf7x2p9q --direct`
- **THEN** the terminal prints a warning and a QR encoding `ez-booth://setup?s={server_url}&k={plaintext_key}&r={role}`

---

### Requirement: Pairing code exchange
The server SHALL expose `POST /api/pair` as an unauthenticated endpoint. The request body MUST contain `{ "code": "<6-digit>" }`. If the code exists, is unexpired, and has not been used, the server MUST mark it as used, set `used_at`, and return `{ "key": "<plaintext_key>", "role": "<role>" }`. Expired or already-used codes MUST return HTTP 410 Gone. Because a 6-digit code has only 10^6 possible values and a successful guess returns a full plaintext key, the server MUST rate-limit `/api/pair` (e.g. a per-IP attempt counter) and return HTTP 429 after a small number of failed attempts within a short window.

#### Scenario: Valid pairing code exchange
- **WHEN** a Kassen-App POSTs a valid unexpired code to `/api/pair`
- **THEN** the server returns HTTP 200 with the plaintext API key and role; the code is marked used

#### Scenario: Code used twice
- **WHEN** a second POST to `/api/pair` with the same code occurs after the first succeeded
- **THEN** the server returns HTTP 410 Gone

#### Scenario: Expired code
- **WHEN** a POST to `/api/pair` uses a code whose `expires_at` is in the past
- **THEN** the server returns HTTP 410 Gone

#### Scenario: Unknown code
- **WHEN** a POST to `/api/pair` uses a code not in the database
- **THEN** the server returns HTTP 404 Not Found

#### Scenario: Pairing code brute-force rate limit
- **WHEN** a client makes repeated `POST /api/pair` requests with incorrect codes from the same source within a short window
- **THEN** the server returns HTTP 429 after a small number of failed attempts, before the attacker can exhaust the 6-digit code space within the 15-minute TTL

---

### Requirement: Key label for identification
API keys SHOULD carry an optional human-readable label set at creation time. The label MUST appear in `key list` output and in revocation confirmation messages to help operators identify which key belongs to which person or device.

#### Scenario: Identify key by label during revocation
- **WHEN** the operator runs `key list` before an event
- **THEN** each key shows its label (e.g., "Kasse 1", "Helper Maria") enabling the operator to identify which key to revoke without guessing

---

### Requirement: Last used tracking
The server SHALL update `last_used_at` on the API key record each time a request is successfully authenticated with that key. This timestamp MUST appear in `key list` output.

#### Scenario: Key used for the first time
- **WHEN** a cashier makes their first authenticated request after pairing
- **THEN** `key list` shows the current timestamp in the "last used" column for that key

#### Scenario: Key never used
- **WHEN** a key was created but no authenticated request has used it yet
- **THEN** `key list` shows "never" in the "last used" column
