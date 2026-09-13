---
title: Phase 1 Milestone 2 Preparation
nav_order: 3
parent: Planning
---

# Phase 1 Milestone 2 Preparation

Prepared after Milestone 1 validation, Safari verification, and PR creation.

## Purpose

Milestone 1 focused on the highest-risk failures in checkout, fee calculation, and silent data loss.
Milestone 2 should now harden the remaining safety gaps that were still visible during validation or test execution.

This document is the handoff artifact for the next milestone branch.

## Recommended Milestone Name

`Phase 1 Milestone 2: Safety, Recovery, and Consistency Cleanup`

## Why This Milestone Exists

Milestone 1 delivered:
- validated `Purchase` and `PurchaseItem` creation
- fee/report consistency fixes
- corrupted purchase diagnostics
- safer checkout/vendor UI error handling
- Chrome and Safari browser coverage
- reusable Safari and UAT validation assets

After that work, the main remaining risks are:
- deprecated fee calculation paths still exist and still emit warnings during test runs
- corruption is detected and logged, but recovery UX is still limited
- compound-key purchase behavior is still easy to misuse conceptually
- storage and recovery behavior is safer, but not yet centralized as a clear recovery workflow
- broader regression coverage is still concentrated around checkout/storage rather than full correction flows

## Scope

### 1. Remove remaining deprecated fee paths

Goal: eliminate all production and test reliance on `ChargingConfig::calculate_fees()` where payout-derived logic should be authoritative.

Concrete work:
- replace remaining `calculate_fees()` call sites with payout-derived calculations
- update tests that intentionally exercise the deprecated path so they validate the replacement behavior instead
- remove or fully isolate the deprecation warning from normal test runs

Success criteria:
- no checkout/report flow depends on deprecated fee calculation helpers
- domain and UI test runs no longer emit fee-calculation deprecation noise

### 2. Make corruption handling actionable for users

Goal: move from "detect and log corruption" to "detect, preserve what can be preserved, and guide the operator clearly".

Concrete work:
- define a user-facing corruption handling path for purchase loads
- surface a translated warning when corrupted records are encountered
- document what the user should do next when corrupted data is found
- add a recovery-oriented API or helper boundary instead of keeping diagnostics as a low-level-only capability

Potential implementation direction:
- introduce a service-level result type that returns valid purchases plus corruption diagnostics
- show a non-blocking warning toast/banner when partial data recovery occurs
- preserve continued app use for healthy records

Success criteria:
- corrupted storage no longer fails silently and no longer relies only on console inspection
- user-facing EN/DE messaging exists for partial recovery situations

### 3. Tighten compound-key purchase semantics

Goal: reduce ambiguity around purchase identity and deletion/lookups in IndexedDB.

Concrete work:
- audit `find_by_id()` usage and decide whether booth-scoped retrieval should become the primary path
- add explicit tests around delete, edit/correction, and fetch behavior using compound keys
- document the intended repository contract so future tests and features do not assume ID-only lookup semantics

Success criteria:
- purchase retrieval/deletion semantics are explicit and consistently tested
- future code paths do not accidentally assume globally unique IndexedDB key access

### 4. Centralize checkout recovery behavior

Goal: turn the current safer draft handling into a well-defined recovery flow.

Concrete work:
- consolidate checkout draft load/save/clear behavior behind clearer helper boundaries
- distinguish between recoverable draft issues and blocking checkout issues
- ensure corrupted draft handling, quota issues, and recovery success use consistent translated messaging
- add tests for recovery transitions, not just individual persistence operations

Success criteria:
- checkout draft recovery is easier to reason about and maintain
- recovery messaging is consistent across restore, clear, corruption, and quota failure cases

### 5. Expand regression coverage for correction workflows

Goal: cover the workflows most likely to be used during real event operations after mistakes happen.

Concrete work:
- add browser tests for purchase deletion/correction flow
- add regression coverage for booth summary recalculation after purchase deletion
- add report consistency checks after correction operations
- consider adding Safari execution coverage for the most critical correction paths

Success criteria:
- deleting/correcting a purchase cannot leave totals or reports inconsistent
- booth/vendor reports remain correct after correction operations

## Explicit Non-Goals

Do not include these in Milestone 2 unless required by discovered bugs:
- large UI redesigns
- report layout restyling
- migration/import-export projects
- deployment or CI rollout changes beyond what is needed for validation
- broad architecture rewrites unrelated to safety/recovery

## Recommended Branch Strategy

- milestone branch: `milestone/phase1-m2-safety-recovery`
- validation branch: `feature/phase1-m2-safety-recovery-validation`

Keep the same workflow used for Milestone 1:
- implement on milestone branch
- validate on validation branch
- merge to `main` only after manual validation approval

## Recommended Atomic Commit Plan

1. `refactor: remove deprecated fee calculation path`
2. `feat: surface corrupted purchase recovery warnings`
3. `test: cover purchase correction and recalculated reports`
4. `refactor: centralize checkout recovery flow`
5. `docs: add milestone 2 recovery validation notes`

If the work ends up smaller or more coupled, combine commits carefully, but keep each commit reversible.

## Acceptance Criteria

Milestone 2 is complete when:
- fee calculation warnings are removed from normal test runs
- corrupted purchase data produces user-visible EN/DE messaging and preserves valid records
- compound-key purchase behavior is documented and covered by tests
- correction flows keep booth totals and vendor reports consistent
- checkout recovery behavior is centralized and regression-tested

## Validation Plan

### Automated

- `cargo test --workspace --lib`
- `./run-tests.sh --chrome`
- `./run-tests.sh --safari`

### Manual

Use the existing documents created in Milestone 1:
- `docs/validation/SAFARI_VALIDATION_CHECKLIST.md`
- `docs/validation/UAT_Ausfuehrungsplan_DE_EN.html`

For Milestone 2 specifically, add manual emphasis on:
- purchase deletion and correction flows
- report recalculation after corrections
- corruption warning visibility and operator guidance
- draft recovery behavior after quota or parse failure

## Suggested First Implementation Order

1. remove deprecated fee calculation usage
2. add correction-flow regression tests
3. lift corruption diagnostics into user-facing behavior
4. centralize checkout recovery helpers
5. update validation notes and execute manual verification

## Rationale For This Scope

This milestone stays tightly aligned with the priorities established during Milestone 1:
- checkout must remain correct under real mistakes
- calculations must stay trustworthy on screen and in reports
- data loss must not happen silently
- operators must be informed clearly when recovery is partial or required

It also avoids prematurely expanding into broader product work before the remaining safety/recovery edges are closed.
