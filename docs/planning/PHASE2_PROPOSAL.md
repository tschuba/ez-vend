---
title: Phase 2 Proposal
nav_order: 2
parent: Planning
---

# Phase 2 Proposal

Prepared from the post-Phase-1 state on `main` after Milestone 1 and Milestone 2 were merged.

## Goal

Phase 2 should shift from emergency safety hardening to product readiness, maintainability, and operator confidence.

Phase 1 already addressed the highest-risk issues in:
- checkout correctness
- fee and payout consistency
- silent data loss detection
- browser validation infrastructure
- Safari/manual validation workflows

The next useful phase is to reduce noise, simplify the codebase, tighten operator workflows, and make the project easier to evolve safely.

## Current Main Branch Health

Current `main` is functionally healthy:
- `cargo test --workspace --lib` passes
- checkout/report logic has regression coverage
- Chrome and Safari browser validation infrastructure is in place

However, normal development still shows warning noise and some documentation drift.

## Remaining Warning / Cleanup Hotspots

### 1. UI dead-code warnings

The `ez-vend-ui` crate still emits warnings for code that appears partially prepared or no longer used:
- unused `ButtonVariant::Ghost`
- unused `ButtonSize::{Small, Large}`
- unused `InputType::{Number, Email, Password}`
- unused `ModalSize::{ExtraLarge, FullScreen}`
- unused `clamp_page`
- unused `ToastContext::error_with_full`
- unused `TwoStepDeleteController` methods
- unused `UiError`, `UiResult`, and `error_to_message_key`
- unused formatting helpers like `format_datetime`, `currency_code`
- unused `translate`
- `transaction_service` field in app state not currently read

These warnings do not indicate a broken app, but they make real regressions harder to spot.

### 2. Legacy compatibility layer drift

`crates/ez-vend-core` still has at least one unused import warning and appears to be a legacy compatibility layer that is no longer central to the active app path.

This suggests a Phase 2 decision is needed:
- either actively maintain it as a supported layer
- or reduce / isolate / de-emphasize it clearly

### 3. Project documentation drift

`README.md` still describes older phase completion percentages and next steps that no longer match the actual state of the project after the recent milestones.

This makes onboarding harder and risks misleading future planning.

### 4. Recovery and correction UX can be refined further

Phase 1 and Milestone 2 introduced:
- partial recovery warnings
- centralized draft recovery
- correction/deletion regression coverage

But the operator workflow could still be improved by making recovery guidance easier to follow during real event operation.

## Recommended Phase 2 Name

`Phase 2: Product Readiness, UX Cleanup, and Maintainability`

## Recommended Milestones

### Milestone 1: Warning Reduction And Codebase Cleanup

Goal: make compiler/test output quiet enough that new warnings stand out immediately.

Scope:
- remove or use dead-code items in `ez-vend-ui`
- remove unused imports in `ez-vend-core`
- decide whether compatibility-only code should stay, move, or be documented as legacy
- clean up helper functions and component variants that are not actually used

Success criteria:
- `cargo test --workspace --lib` produces substantially fewer warnings
- remaining warnings are intentional and documented

### Milestone 2: Operator Workflow Polish

Goal: make day-of-event usage more confident and less error-prone.

Scope:
- improve correction/delete affordances and messaging
- review whether recovery warnings should include stronger “what to do next” guidance
- add manual/UAT emphasis for operator recovery and correction flows
- consider better summary cues when recovered data is partial

Success criteria:
- operators can understand recovery and correction outcomes without relying on console inspection
- manual validation confirms the guidance is sufficient in practice

### Milestone 3: Documentation And Onboarding Refresh

Goal: ensure the repo tells the truth about the current system.

Scope:
- update `README.md` to reflect the real current status
- document the current validation workflow and branch strategy more explicitly
- document the meaning of the reusable validation artifacts:
  - `docs/validation/SAFARI_VALIDATION_CHECKLIST.md`
  - `docs/validation/UAT_Ausfuehrungsplan_DE_EN.html`
  - `docs/planning/PHASE1_M2_PREPARATION.md`

Success criteria:
- new contributors can understand how to run, validate, and extend the app without relying on old roadmap text

### Milestone 4: Optional UX / Reporting Refinement

Goal: improve day-to-day usability without changing financial correctness.

Scope candidates:
- report readability improvements
- stronger empty/loading states
- better pagination and transaction detail ergonomics
- improved localized formatting consistency

This milestone should happen only after the cleanup milestones above.

## Explicit Non-Goals

Unless a new bug forces it, Phase 2 should avoid:
- changing the validated fee/payout logic again
- broad data model redesigns
- replacing the storage layer
- major visual redesign work unrelated to usability
- deployment or hosting projects

## Suggested First Implementation Order

1. warning cleanup in active crates
2. README / validation-documentation refresh
3. operator workflow messaging polish
4. optional UX refinement

## Acceptance Criteria For Phase 2

Phase 2 is successful when:
- warning noise is significantly reduced
- README and repo documentation match the real current project state
- operator correction/recovery workflow is clearer in both UI and validation docs
- future validation sessions are easier because the system and docs are less noisy and more intentional

## Notes For Future Work

If a later phase introduces exports, imports, migrations, or release automation, those should be treated as separate product-readiness milestones rather than bundled into this cleanup phase.

The currently agreed backup/recovery follow-up is captured in `docs/planning/DATA_BACKUP_IMPLEMENTATION_PLAN.md` so it can be executed as a separate track later.
