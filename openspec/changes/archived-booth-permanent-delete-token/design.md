## Context

The current booth deletion flow is inconsistent. The UI opens a confirmation modal and, depending on purchase history, may delete a booth immediately or after a short token check, but the underlying domain service still rejects archived booths and allows active booths. There is also no clear path for permanent deletion of archived booths, even though archived booths are the only records that should be eligible for irreversible removal.

This change affects the booth lifecycle across UI and domain layers. The implementation must prevent accidental loss of active booth data while preserving a deliberate, token-confirmed path for archived booth cleanup.

## Goals / Non-Goals

**Goals:**
- Prevent permanent deletion of active booths.
- Allow permanent deletion of archived booths only after explicit token confirmation.
- Make the deletion rule authoritative in the domain/service layer, not only in the UI.
- Reuse the existing confirmation-token UX pattern so the new flow feels consistent.

**Non-Goals:**
- No new archival workflow.
- No change to how booths are archived or exported.
- No storage schema migration.
- No attempt to turn the token into a cryptographic authorization mechanism; it remains an operator confirmation step.

## Decisions

1. Enforce the deletion rule in the booth service, not just the UI.
   The current UI calls the repository delete directly, which bypasses domain validation. The service should become the gatekeeper for permanent deletion so active booths cannot be removed through an alternate code path.
   Alternatives considered:
   - UI-only validation: simpler, but leaves direct-delete paths vulnerable.
   - Repository-level checks: helpful, but pushes business rules into persistence code.

2. Keep permanent deletion token-confirmed and booth-specific.
   The existing confirmation-token pattern already derives a booth-specific token from the booth ID. Reusing that pattern avoids introducing a second confirmation scheme and keeps the new flow consistent with the archive wizard.
   Alternatives considered:
   - A new token format: unnecessary user friction and more code to maintain.
   - Password or backup-derived confirmation: heavier than the requirement calls for.

3. Treat archived status as the only eligible state for irreversible deletion.
   Active booths should be rejected before any destructive operation starts. The UI should reflect that state by directing operators to archive first, while the actual delete action remains available only for archived booths with a valid token.
   Alternatives considered:
   - Allowing active deletion with a token if there are no purchases: this is the current unsafe behavior and should be removed.
   - Hiding the action entirely for active booths: acceptable UX, but the service still needs to enforce the rule.

4. Keep the change data-only from a persistence perspective.
   Booth records do not need a migration. The deletion rule changes at runtime, and existing archived booths remain eligible once the new token-confirmed path is used.
   Alternatives considered:
   - Marking booths with a new deletion state: unnecessary for the requested behavior.

## Risks / Trade-offs

[UI and service can drift] -> Keep the domain service authoritative and route the UI through that path so the rule is enforced in one place.

[Confirmation tokens are not secret] -> Treat the token as a deliberate operator confirmation only; do not present it as an access control mechanism.

[Direct repository deletion may survive in other call sites] -> Audit the booth delete call chain and remove or privatize any bypass paths.

[Archived booth deletion is irreversible] -> Preserve the existing archived workflow assumptions and make the confirmation prompt explicit about permanence.

## Migration Plan

- Deploy the service rule and UI gating together so operators do not see a broken intermediate state.
- No database migration is required.
- If rollback is needed, revert the service guard and UI gating together so deletion behavior returns to the previous state.

## Open Questions

- None at this time. The current confirmation-token pattern is sufficient for the requested behavior, and no new data model or migration decisions are required.
