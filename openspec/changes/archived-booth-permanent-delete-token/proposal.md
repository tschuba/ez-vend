## Why

The current booth deletion behavior is inconsistent: active booths can be permanently deleted without the safeguards used elsewhere, while archived booths do not have a clear path for permanent deletion. This change tightens the lifecycle rules so irreversible deletion is only available for archived booths and requires explicit confirmation.

## What Changes

- Disallow permanent deletion of active booths.
- Allow permanent deletion of archived booths only after confirmation via token.
- Preserve archival status as the gate for destructive deletion instead of treating active booths as directly deletable records.
- Surface a clear failure path when a deletion request targets an active booth or omits the required token.

## Capabilities

### New Capabilities
- `booth-permanent-deletion`: controlled permanent deletion flow for archived booths, including token-based confirmation and rejection of active booth deletion requests.

### Modified Capabilities
- None.

## Impact

Affected booth lifecycle logic, deletion UI or command handlers, confirmation flow, and any storage or service code that currently allows direct deletion of active booths.
