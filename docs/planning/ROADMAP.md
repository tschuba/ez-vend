---
title: Roadmap
nav_order: 1
parent: Planning
---

# Roadmap

This roadmap gives a simple view of what is already in place, what remains active, and what is still future work.

It is intentionally lightweight. Detailed historical planning documents remain available in this directory for deeper context.

## Current Baseline

The project already includes:

- booth management and fee configuration
- checkout, reporting, and printing flows
- IndexedDB persistence and browser-local recovery behavior
- JSON export and import for backups
- Chrome and Safari validation support
- bilingual German and English UI support

## Current Documentation and Product-Readiness Focus

The immediate work around the current product state focuses on:

- keeping documentation aligned with the real application behavior
- improving operator confidence in backup, recovery, and validation workflows
- reducing noise that makes real regressions harder to spot

## Active Follow-Up Areas

### UX and Reporting Refinement

Possible near-term follow-up work includes:

- report readability improvements
- stronger empty and loading states
- transaction-detail ergonomics
- wording or workflow polish where manual validation shows friction

### Final Manual Validation Passes

Some prepared validation assets are already in place and may still need formal execution for sign-off on specific milestones or release slices.

See:

- `PHASE2_PROPOSAL.md`
- `DATA_BACKUP_IMPLEMENTATION_PLAN.md`
- `../validation/PHASE2_M2_VALIDATION_RESULTS.md`

## Planned Future Work

### Migration from Original ez-booth

Operators can now import data from the original `ez-booth` SQLite database through `Settings > Migration`.

The shipped workflow includes:

- manual upload of the legacy `booth.db` file
- platform-specific location guidance with copy-to-clipboard support
- strict validation with grouped, operator-friendly issue display
- automatic JSON backup before replacement import
- empty-state discoverability when no booths exist yet

See `SQLITE_MIGRATION_IMPLEMENTATION_PLAN.md` for implementation details.

### Progressive Web App Support

The app already fits a web-first architecture, but a fuller installable PWA workflow is still future work.

See `../technical/ADR_PWA_IMPLEMENTATION.md`.

### Broader Device-to-Device Transfer Options

The current supported path is export, import, and merge.

Wider transfer or sync models remain an architectural topic rather than a finished product feature.

See `../technical/ADR_DEVICE_TO_DEVICE_TRANSFER.md`.

## Explicit Non-Goals Right Now

The current direction does not prioritize:

- changing fee and payout semantics without a concrete bug or requirement
- replacing the storage layer
- cloud-first redesign work
- a major visual redesign detached from operator usability

## Supporting Documents

- [Phase 2 Proposal](PHASE2_PROPOSAL.md)
- [Data Backup And Recovery Implementation Plan](DATA_BACKUP_IMPLEMENTATION_PLAN.md)
- [Phase 1 Milestone 2 Preparation](PHASE1_M2_PREPARATION.md)
- [Redesign Status](../redesign/05_STATUS.md)
