---
title: Redesign Summary
nav_order: 1
parent: Redesign
---

# Redesign Summary

`ez-vend` is a Rust and WebAssembly redesign of the original `ez-booth` project.

This summary consolidates the high-level story that was previously spread across the original specification, the Java application analysis, the architecture design, the improvements document, and the current status log.

## Why the Redesign Happened

The original `ez-booth` provided a strong booth-management foundation, but it came with the cost and operational shape of a Java-based packaged application:

- heavier runtime requirements
- platform-specific distribution concerns
- a more complex deployment story
- a file-oriented local storage model
- limited flexibility for a lightweight browser-first rollout

The redesign goal was not to change the core booth problem. It was to keep the useful booth workflows while simplifying delivery, reducing runtime overhead, and making the application easier to operate and extend.

## Primary Goals

The redesign set out to deliver a system that is:

- smaller and more resource-efficient
- easier to run on different devices
- capable of offline-first booth operation
- more maintainable through clear crate boundaries
- safer for money and payout logic through a dedicated domain layer
- friendlier to bilingual operator workflows

## From Original App to Redesign

### Original App Shape

The original Java-based application used a modular architecture centered on:

- Java and Spring Boot
- Vaadin web UI
- gRPC-based service communication
- SQLite persistence
- packaged desktop-style distribution

That architecture was well structured, but it carried a heavier operational footprint.

### Redesign Shape

`ez-vend` shifts the product to:

- Rust 2021 workspace architecture
- client-side WebAssembly execution
- Leptos for the UI
- IndexedDB for browser-local persistence
- optional launcher packaging for local distribution

The result is a browser-first app that still works offline for its core workflows.

## Key Architecture Decisions

### Rust for Business-Critical Logic

Rust was chosen to make business-critical behavior easier to reason about and harder to break silently.

This especially matters for:

- exact decimal money handling
- validation boundaries
- typed domain models
- explicit error handling

The redesign keeps money math in `rust_decimal::Decimal` and treats fee and payout behavior as regression-sensitive.

### WebAssembly for Delivery Simplicity

WebAssembly allows the core application to run directly in the browser without shipping a full Java runtime.

That supports the redesign goals of:

- smaller distribution size
- faster startup feel
- less platform-specific packaging pressure
- simpler local and hosted delivery models

### IndexedDB for Offline-First Local Persistence

The redesign uses IndexedDB because the main product requirement is still local, dependable booth operation.

IndexedDB supports:

- persistent local structured data in the browser
- offline-first behavior without a backend
- storage of booths, vendors, purchases, and metadata

`localStorage` remains in use for lighter concerns such as UI preferences and checkout draft recovery.

### Cargo Workspace with Focused Crates

The app is split into focused crates rather than one large application crate:

- `domain`
- `ez-vend-storage`
- `ez-vend-ui`
- `ez-vend-app`
- `ez-vend-launcher`

This keeps the domain model, persistence, UI, startup, and packaging concerns separate.

### Domain-First Design

The redesign intentionally keeps business logic out of ad hoc UI code.

That means:

- validation starts at the domain boundary
- fee and payout semantics stay centralized
- storage and UI layers consume domain behavior instead of redefining it

## What Improved in Practice

### Delivery Model

The redesign replaces the original heavier Java packaging model with a static web bundle and optional local launcher.

This makes it easier to:

- run the app in a normal browser
- serve it locally during development
- distribute a smaller local package when needed

### Runtime Footprint

The design target was a substantially smaller footprint than the original application stack.

The broader redesign analysis targeted improvements in:

- binary and bundle size
- runtime memory use
- startup responsiveness

The important documentation outcome is that the redesign intentionally optimized for lighter booth-day operation rather than adding a more complex server architecture.

### Operator Workflows

The redesign also improved the workflow surface, not just the technical stack.

Notable improvements include:

- checkout draft recovery
- clearer correction and deletion workflows
- backup and restore via explicit export and import tools
- stronger cross-browser validation support
- reusable Safari and UAT validation assets

### Documentation and Validation

The project now includes more explicit operational documentation than the original redesign drafts did, including:

- validation workflow guidance
- Safari-specific validation checklists
- bilingual UAT guidance
- backup and merge operator guides

## Implementation Progress at a Glance

### Foundation Work

The implemented system now includes:

- domain models for booth, vendor, purchase, and money logic
- IndexedDB-backed repositories
- checkout, report, and recovery logic
- bilingual UI support
- operator-facing validation assets
- export and import support for backup and recovery

### Current Product Readiness Direction

After the original redesign roadmap, the project added a later product-readiness track focused on:

- warning reduction and cleanup
- operator workflow polish
- documentation and onboarding refresh
- optional future UX and reporting refinements

That later work is why the current documentation has both the original redesign phases and newer milestone-based tracking language in the repository history.

## Important Trade-Offs

The redesign made several intentional trade-offs.

### Browser Storage Instead of File-Native App Storage

This simplifies deployment, but it means teams must understand that local data lives inside the current browser profile.

That is why backup/export guidance is a first-class part of the current documentation set.

### Offline-First Instead of Cloud-First

The project favors local reliability, privacy, and low operating friction over remote-service complexity.

That means:

- no cloud dependency for core booth use
- no automatic remote sync by default
- explicit export/import workflows instead of hidden network magic

### Strong Domain Boundaries Instead of Fast UI-Only Rules

This can make some implementation paths feel more deliberate, but it protects fee and payout correctness and keeps the system more testable.

## What Still Remains

Some items discussed in the older redesign documents remain planned or deferred, including:

- migration from original `ez-booth` SQLite data
- fuller PWA packaging and install flow
- broader synchronization or transfer options beyond the current backup and merge workflows

Those items are still useful future work, but they are not the current baseline for the product as it exists today.

## Recommended Reading Paths

### If You Want the Big Picture

1. [README](https://github.com/tschuba/ez-vend/blob/main/README.md)
2. [Architecture Overview](https://github.com/tschuba/ez-vend/blob/main/ARCHITECTURE.md)
3. [What Changed from ez-booth to ez-vend?](../COMPARISON_TO_ORIGINAL.md)

### If You Want the Technical History

1. [00_SPEC.md](00_SPEC.md)
2. [01_ANALYSIS.md](01_ANALYSIS.md)
3. [02_ARCHITECTURE.md](02_ARCHITECTURE.md)
4. [03_IMPROVEMENTS.md](03_IMPROVEMENTS.md)
5. [04_IMPLEMENTATION.md](04_IMPLEMENTATION.md)
6. [05_STATUS.md](05_STATUS.md)

### If You Want Current Operational Guidance

1. [Getting Started](../GETTING_STARTED.md)
2. [Validation Workflow](../validation/VALIDATION_WORKFLOW.md)
3. [Data Backup Guide](../user-guides/DATA_BACKUP_GUIDE.md)
4. [Multi-Device Booth Merge Guide](../user-guides/MULTI_DEVICE_MERGE_GUIDE.md)

## Summary

The redesign kept the booth-management purpose of the original application while changing the technical delivery model in a major way.

The most important outcomes are:

- Rust-based domain logic for correctness-sensitive behavior
- browser-first WASM delivery for lighter distribution
- IndexedDB-backed offline-first persistence
- clearer backup, recovery, and validation workflows
- a more maintainable crate-based architecture

That makes `ez-vend` not just a port, but a deliberate simplification of how the product is built, run, and supported.
