---
title: ADR Device-to-Device Transfer
nav_order: 3
parent: Technical Docs
---

# ADR: Device-to-Device Booth Data Transfer

Date: 2026-03-30
Status: Pending
Decision Maker: TBD

## Context

The ez-booth application requires the ability to transfer booth data from one device to another without requiring:

- internet connectivity
- WiFi router or access point infrastructure
- cloud services or external servers

The initial implementation scoped transfer to a single booth. Full-backup import with per-event reconciliation is now also supported: when the same event was independently created on multiple devices with different UUIDs, the import service resolves the canonical local booth by name+date match and merges into it. Ambiguous cases (multiple local matches) surface a conflict wizard for operator review.

### Background

Real-world booth data can range from tens of kilobytes to multiple megabytes depending on the number of vendors, purchases, and retained history. A practical device-to-device workflow must therefore handle more than just tiny payloads and remain reliable for substantially larger booth transfers.

QR-code-based booth data transfer was explored before this ADR and rejected because it does not match expectations for data density or user experience when transferring realistic booth payloads.

In practice, the most common expected transfer is Windows-to-Windows. Mixed-device transfer should remain possible, but the solution does not need to force a single mechanism across all device combinations if a multi-option approach yields better reliability.

### Platform Priority

1. Windows
2. macOS
3. iOS
4. Android

### Technical Constraints

- the application runs in web browsers as a WASM app
- transfer must work without internet and without WiFi router infrastructure
- the solution should favor mature, reliable platform capabilities over experimental browser APIs
- the existing export/import infrastructure should be reused where possible

### Existing Capabilities

- export and import services already exist in `crates/storage/src/export/`
- MessagePack serialization and gzip compression are already used in the codebase
- backup validation and import conflict handling already exist

## Decision Drivers

- true device-to-device operation without internet or WiFi router
- strong support for Windows and macOS first
- workable support for iOS and Android
- high data density suitable for full booth payloads
- low operational risk and predictable user experience
- reasonable implementation complexity using existing services
- maximum practical compatibility across browser and device combinations
- transfer payloads that remain inspectable and debuggable during development and support
- integrity verification for transferred data regardless of transport option

## Options Considered

### Option 1: Web Bluetooth Transfer

#### Concept

Use the Web Bluetooth API for direct BLE transfer between devices.

The ideal model is:

- one device advertises as a BLE peripheral
- the other device scans and connects as a BLE central
- data transfers through custom GATT characteristics in chunks

#### Pros

- true device-to-device radio communication
- no router or internet required
- suitable data density for booth transfer
- works well in principle for nearby devices

#### Cons

- critical browser limitation: Web Bluetooth in browsers does not support peripheral advertising for this use case
- browsers can scan and connect, but cannot reliably expose the app as a discoverable BLE peripheral
- iOS Safari does not support Web Bluetooth at all, and all iOS browsers remain affected because they use WebKit
- Firefox does not support Web Bluetooth and is not a viable target for this approach
- high implementation and support complexity

#### Platform Fit

- Windows: partial browser support, but blocked by advertising limitation
- macOS: partial browser support, but blocked by advertising limitation
- iOS: not viable
- Android: partial browser support, but still limited by browser role restrictions

#### Browser Support Matrix

| Platform | Chrome | Edge | Safari | Firefox | Notes |
|----------|--------|------|--------|---------|-------|
| Windows 10+ | supported | supported | n/a | not supported | central role only; no browser advertising/peripheral role |
| macOS | supported | supported | not supported | not supported | central role only; no browser advertising/peripheral role |
| Android 6+ | supported | supported | n/a | not supported | central role only; no browser advertising/peripheral role |
| iOS | not supported | not supported | not supported | not supported | WebKit-based browsers do not provide Web Bluetooth |

The critical limitation is role support rather than raw API availability alone. Browsers that implement Web Bluetooth act as BLE central clients. They do not reliably act as BLE peripherals and do not provide the discoverable advertising behavior needed for a straightforward browser-to-browser peer flow.

#### Outcome

Deferred as a supplemental option. BLE remains unsuitable as the only product path across the full platform set, but it may still be worth a Windows-focused proof of concept because Windows-to-Windows is the primary expected transfer case.

For Web Bluetooth to be viable, it would need:

- an explicit sender/receiver role model instead of true peer discovery
- a coordination step such as QR code or short-code exchange
- measured confirmation that throughput and reliability are acceptable for typical booth payloads
- a fallback path for unsupported browsers and mixed-device transfers

The proof of concept should be limited to Windows-to-Windows transfer in current Chromium-based browsers.

The proof of concept should only advance to product implementation if all of the following are demonstrated:

- successful transfer between two Windows devices using supported Chromium-based browsers with no internet connection and no WiFi router
- reliable completion for representative booth payload sizes from tens of kilobytes up to multi-megabyte transfers
- explicit integrity verification before import succeeds
- an operator flow that is understandable without technical troubleshooting or developer-only tooling
- failure handling that leaves the receiving side in a safe state and clearly instructs the operator to retry or fall back to file transfer

### Option 2: WebRTC Data Channels With Manual Signaling

#### Concept

Use WebRTC data channels for high-density transfer and exchange signaling information manually, for example with small QR payloads or short codes.

The intended flow is:

- device A generates a WebRTC offer
- devices exchange offer and answer out of band
- WebRTC establishes a peer connection
- booth data transfers over the data channel

#### Pros

- high transfer speed once connected
- broad browser support for WebRTC itself
- good data density and reliability after connection establishment

#### Cons

- WebRTC still needs a workable network path between devices
- in practice this usually means same-network connectivity, Bluetooth PAN, or other infrastructure-like support
- it does not reliably satisfy the requirement of isolated device-to-device transfer with no WiFi router and no internet across the full platform set
- multi-step signaling adds user friction and failure cases

#### Platform Fit

- Windows: technically possible in some environments, but dependent on network path
- macOS: technically possible in some environments, but dependent on network path
- iOS: WebRTC support exists, but transport assumptions remain
- Android: workable in some scenarios, but not a universal infrastructure-free solution

#### Outcome

Rejected. WebRTC is strong for connected peers, but it does not cleanly meet the requirement of direct device-to-device transfer without relying on supporting network conditions.

### Option 3: Web Share API Plus Native OS Transfer Mechanisms

#### Concept

Export booth data as a file and let the operating system perform the actual device-to-device transfer using native share capabilities.

The flow is:

- export a single booth into a transfer file
- invoke native share where supported
- the OS handles peer discovery and transfer
- the receiving device imports the file into ez-booth

Examples include:

- AirDrop on Apple platforms
- Nearby Share or equivalent native sharing on supported platforms
- manual file transfer fallback where native peer sharing is not available in the browser context

#### Hardware Prerequisites

Native OS sharing mechanisms (AirDrop, Nearby Share, Windows Nearby Sharing) require:

- **WiFi and Bluetooth hardware** enabled on both devices
- no WiFi router or access point needed - devices create peer-to-peer wireless links

**Important:** "Without WiFi router infrastructure" means no central access point is required. However, devices still need WiFi and Bluetooth radios for peer-to-peer communication. Many older desktop PCs lack these radios and must use the file export/import fallback method.

**Network limitations:** Even when WiFi is available, hotspots with client isolation (AP isolation) prevent devices from discovering each other. In these scenarios, file export/import or AirDrop (which creates its own wireless link) are the viable options.

#### Pros

- leverages mature OS-level transfer capabilities instead of fragile browser-only peer protocols
- fits iOS much better than Web Bluetooth
- reuses the existing export and import architecture
- supports large payloads including multi-megabyte booth datasets
- keeps the product aligned with a web-first architecture
- gives a practical desktop fallback through ordinary file export and import

#### Cons

- transfer is not fully contained inside a single in-app peer discovery flow
- desktop browser support for invoking native share is less uniform than on mobile
- import UX on the receiving side still needs design work

#### Platform Fit

- Windows: viable with file export/import fallback and native share where available
- macOS: viable with file export/import fallback and strong OS sharing options
- iOS: strong fit through native sharing
- Android: viable through native sharing support

#### Outcome

Recommended. This is the most practical and robust option for the current product constraints.

### Option 4: Local WiFi HTTP Server

#### Concept

Make one device act as a local server and let the other device connect through a local network.

#### Pros

- straightforward transport model if a network is available
- high data density and good throughput

#### Cons

- requires network infrastructure or a comparable setup that violates the stated constraint
- browser runtime limitations make local server behavior awkward and inconsistent
- not appropriate for the direct device-to-device requirement

#### Outcome

Rejected. This option does not satisfy the infrastructure constraint.

### Option 5: Native Application Wrapper

#### Concept

Wrap the app in a native shell such as Tauri or Electron to gain access to native platform APIs for peer-to-peer transfer, including stronger Bluetooth options.

#### Pros

- unlocks native APIs unavailable to browsers
- could enable a more seamless in-app direct transfer experience on desktop

#### Cons

- changes the deployment and support model significantly
- adds native packaging, distribution, and maintenance overhead
- does not help enough for the current web-first scope

#### Outcome

Deferred. This is a future architectural direction, not the right next step for the current product.

## Decision

Adopt a multi-option transfer strategy.

- Primary cross-platform approach: Option 3, Web Share API plus native OS transfer mechanisms where available
- Universal fallback: standard file export and import
- Additional Windows-focused option under investigation: Option 1, Web Bluetooth transfer with explicit role coordination

## Rationale

This option best matches the actual constraints and platform priorities.

- It avoids depending exclusively on browser API gaps that make Web Bluetooth unreliable or impossible for the required role on many platforms.
- It avoids the hidden network dependency in WebRTC-based approaches.
- It uses the strongest parts of the current product: export, import, validation, and structured backup data.
- It supports a practical user story on every target platform, even when the exact transfer UX differs.
- It keeps the most important scenario, Windows-to-Windows transfer, open for a more direct option if a proof of concept succeeds.
- It favors transports that can share a common transfer payload with strong integrity checks and good debuggability.

For mobile devices, native OS sharing is the most realistic path to true infrastructure-free transfer.

For desktop platforms, file export and import is an acceptable fallback because Windows and macOS are the highest-priority platforms and users on those systems can reliably complete transfer through standard file-based workflows even when direct browser-triggered native sharing is inconsistent.

For Windows-to-Windows, Web Bluetooth may still become a useful additional path if it proves reliable enough despite the browser role restrictions. That path should be treated as an optimization for a common environment, not as the sole supported architecture.

## Decisions Made

### Transfer Format

Use a gzipped JSON transfer envelope as the preferred transfer format across all transfer options.

Rationale:

- maximizes compatibility across browser, desktop, and mobile workflows
- remains inspectable and debuggable during development and support
- achieves good practical density once compressed
- works well as a shared payload format across Web Share, file export/import, and any future Windows-focused Bluetooth flow

The payload should carry explicit format metadata and versioning so transfers remain inspectable and evolvable.

### Integrity Verification

Use embedded integrity metadata for all transfer options.

Rationale:

- integrity matters more than confidentiality for this use case
- transfer corruption must be detectable regardless of transport mechanism
- a shared checksum-based validation model keeps behavior consistent across all options
- integrity metadata improves supportability and troubleshooting

### Initial Scope

Focus on single-booth transfer.

Rationale:

- matches the stated use case
- keeps UX and validation simpler
- full-database export already exists as a separate backup concern

### Desktop Fallback Strategy

Use standard file export and file import as the desktop fallback.

Rationale:

- works reliably on Windows and macOS
- avoids dependence on uneven browser support for native sharing
- keeps the concept understandable and supportable

### Windows-Specific Transfer Exploration

Investigate Web Bluetooth as an optional Windows-to-Windows transfer path, not as a universal requirement.

Rationale:

- Windows is the highest-priority platform
- Windows-to-Windows is the most common expected transfer scenario
- even an imperfect browser-only Bluetooth path may be worth offering if it performs better than manual file exchange in the common case
- mixed-device support can still be covered by Web Share and file-based fallback

Implementation gate:

- keep this option behind a proof-of-concept milestone until the acceptance criteria in Option 1 are met
- do not let this option block delivery of Web Share and file-based transfer
- if the proof of concept shows inconsistent browser behavior, weak throughput, or unclear operator UX, keep Web Bluetooth as a deferred experiment rather than a supported workflow

## Consequences

### Positive

- practical path forward without changing the app into a native product
- strong reuse of current export and import services
- supports full booth data payloads from small datasets to multi-megabyte archives
- clear fallback story for desktop platforms
- lower implementation risk than browser-only peer protocols
- room to add a Windows-optimized path without making it mandatory for every platform

### Negative

- receiving-side import flow still needs UX design
- transfer UX will vary somewhat by platform
- the selected approach is less magical than a fully in-app peer discovery flow
- Web Bluetooth remains a research item with real platform and browser constraints

### Neutral

- further documentation will be needed for operator-facing instructions
- the exact transfer file extension and file association details can be decided during implementation planning
- the exact checksum and transfer-envelope schema can be finalized during implementation planning

## Open Question

The remaining open product question is the receiving-side import UX.

This ADR leaves that design open for a follow-up decision, including whether the first version should support:

- basic file picker import only
- drag and drop import
- registered file handling where supported
- a combination of the above

## Next Steps

1. decide the receiving-side import UX
2. create a focused implementation plan for single-booth transfer
3. define the gzipped JSON transfer envelope and integrity metadata
4. prototype the Windows-to-Windows Web Bluetooth option and measure reliability and throughput
5. document platform-specific transfer flows for operators

## Related Documents

- `docs/planning/DATA_BACKUP_IMPLEMENTATION_PLAN.md`
- `docs/validation/VALIDATION_WORKFLOW.md`
- `docs/BRANCH_STRATEGY.md`
- `README.md`
