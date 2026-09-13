## 1. Domain and service rules

- [ ] 1.1 Update the booth deletion service path so permanent deletion is allowed only for archived booths and rejected for active booths.
- [ ] 1.2 Require and validate the confirmation token on the archived deletion path.
- [ ] 1.3 Add regression tests covering active-booth rejection, archived-booth success, and missing or invalid token rejection.

## 2. UI flow and gating

- [ ] 2.1 Route the booth list delete action through the updated service path instead of deleting directly through the repository.
- [ ] 2.2 Keep the confirmation-token modal for archived booths and block active booths from reaching the permanent-delete confirmation path.
- [ ] 2.3 Update the user-facing error and success handling so operators get a clear message when active deletion is refused.

## 3. Verification

- [ ] 3.1 Run the narrow domain and UI tests that cover booth deletion and token confirmation.
- [ ] 3.2 Fix any regressions exposed by the targeted test run and re-run the same checks.
