---
title: Comparison to Original
nav_order: 3
---

# What Changed from ez-booth to ez-vend-rs?

This guide focuses on the practical differences for people who already know the original Java-based `ez-booth`.

If you want the short version: the new app keeps the same core booth workflow, but delivers it as a smaller, browser-first Rust and WebAssembly application with a different storage and backup model.

## Quick Comparison

| Topic | Original `ez-booth` | `ez-vend-rs` |
| --- | --- | --- |
| Runtime model | Java app with local server pieces | Browser-first Rust/WASM app |
| Distribution | Platform-specific packaged download | Static web bundle plus optional launcher |
| Startup feel | Heavier desktop-style startup | Fast browser load |
| Local data storage | SQLite file | IndexedDB in the browser |
| Backup shape | File-oriented app data | JSON export and import |
| Offline use | Yes | Yes |
| Languages | Original app context | German primary with English fallback |

## What Changes for Users

### 1. How You Start the App

The original app was centered on a Java-based deployment model.

`ez-vend-rs` is centered on a browser-first model:

- open a served URL during development or hosted use
- or run the optional local launcher for desktop-style packaged usage

That means less runtime setup, smaller downloads, and fewer platform-specific installation concerns.

## 2. Where Your Data Lives

This is the biggest user-facing change.

### Original `ez-booth`

- stored data in a local SQLite database file
- backup habits were file-oriented
- moving data between machines meant moving application data files

### `ez-vend-rs`

- stores booths, vendors, and purchases in the browser via IndexedDB
- stores lighter UI state, such as preferences and checkout drafts, in browser storage
- uses JSON export and import as the supported backup and recovery path

This makes the app easier to run in a browser, but it also means teams must understand browser storage clearly.

Read more:

- [Data Backup Guide](user-guides/DATA_BACKUP_GUIDE.md)
- [Multi-Device Booth Merge Guide](user-guides/MULTI_DEVICE_MERGE_GUIDE.md)

## 3. Backup and Restore Workflow

Instead of handling raw application database files, `ez-vend-rs` gives operators explicit export and import actions.

That adds:

- full export for all locally stored data
- booth export for one event
- import preview and validation
- conflict handling with `Merge`, `Skip`, and `Replace`

This is a more guided workflow than copying database files manually.

Read more:

- [Data Backup Guide](user-guides/DATA_BACKUP_GUIDE.md)
- [Validation Workflow](validation/VALIDATION_WORKFLOW.md)

## 4. Reporting and Printing

The core reporting purpose stays the same: operators still need clear vendor payouts and booth summaries.

What changed is the delivery shape:

- the redesign keeps reporting in the browser flow
- print output is driven by browser print handling and print-focused layouts
- report totals are aligned with the Rust domain calculation logic used elsewhere in the app

## 5. Operator Workflow Improvements

`ez-vend-rs` adds a number of practical operator-focused improvements:

- on-screen checkout keypad with configurable quick amounts
- persistent amount entry mode
- clearer correction and deletion guidance
- checkout draft recovery after interruption or refresh
- corruption detection and partial-recovery warnings
- bilingual validation and UAT materials

These are not just technical changes. They directly affect day-of-event confidence.

## Features That Carry Forward

The redesign still supports the main booth-management goals:

- booth setup with fee configuration
- vendor-based checkout
- fee and payout calculation
- vendor and booth reporting
- offline-capable event operation

## New Features in ez-vend-rs

These are meaningful additions or clearer workflows compared with the original model:

- browser-based export and import workflow for backups
- migration from the original `ez-booth` SQLite `booth.db` via `Settings > Migration`
- booth-level backup and restore support in addition to full exports
- merge-oriented multi-device booth recovery path
- draft recovery for interrupted checkout work
- stronger browser validation coverage, including Safari-sensitive flows
- reusable operator validation documents in German and English

Read more:

- [Data Backup Guide](user-guides/DATA_BACKUP_GUIDE.md)
- [Multi-Device Booth Merge Guide](user-guides/MULTI_DEVICE_MERGE_GUIDE.md)
- [Safari Validation Checklist](validation/SAFARI_VALIDATION_CHECKLIST.md)

## Features Not Yet Available

Some original or planned capabilities are still deferred or intentionally not implemented yet.

### Not Yet Available

- full PWA deployment flow for installable offline packaging
- broader sync-style workflows beyond the current export, import, and merge guidance

### Intentionally Out of Scope for Now

- cloud-first storage or automatic remote sync
- large server-backed redesign work that would move the app away from its offline-first browser model

## What This Means for Teams Moving Over

If your team already knows the original app, the most important adjustments are:

1. treat JSON export files as the primary backup artifact
2. understand that browser storage is local to one browser profile on one device
3. use `Settings > Migration` with the legacy `booth.db` file if you are moving existing data from the original app
4. rehearse import and merge workflows before a real event if multiple devices are involved
5. use the validation guides to confirm printing, reporting, and recovery expectations in your target browser

## Suggested Reading Order for Former ez-booth Users

1. [Getting Started](GETTING_STARTED.md)
2. [Data Backup Guide](user-guides/DATA_BACKUP_GUIDE.md)
3. [Multi-Device Booth Merge Guide](user-guides/MULTI_DEVICE_MERGE_GUIDE.md)
4. [Fee Calculation Guide](user-guides/FEE_CALCULATION.md)
5. [Validation Workflow](validation/VALIDATION_WORKFLOW.md)

## Want the Technical Backstory?

If you want the design rationale instead of the operator-facing summary:

- [Redesign Summary](redesign/REDESIGN_SUMMARY.md)
- [Original ez-booth Analysis](redesign/01_ANALYSIS.md)
- [Redesign Architecture](redesign/02_ARCHITECTURE.md)
- [Areas for Improvement](redesign/03_IMPROVEMENTS.md)
