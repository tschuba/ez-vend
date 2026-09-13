---
title: Current Implementation Analysis
nav_order: 3
parent: Redesign
---

# ez-booth Current Implementation Analysis

---
**Document Status:** Historical Reference
**Last Updated:** 2026-04-04
**Purpose:** Analysis of the original Java-based ez-booth application that informed the redesign.

---

**Analysis Date:** March 19, 2026  
**Project Version:** 1.0.0  
**Analyzed by:** GitHub Copilot CLI

---

## Executive Summary

`ez-booth` is a Java-based booth management system built on Spring Boot with a Vaadin web UI. The application follows a modular architecture with gRPC for service communication, SQLite for data persistence, and JPackage for portable distribution. The codebase consists of approximately 12,191 lines of Java code across 155 source files, with a total project size of 315MB.

---

## 1. Technology Stack

### Core Technologies
- **Language:** Java 25
- **Framework:** Spring Boot 3.5.6
- **Build System:** Maven 3.x
- **UI Framework:** Vaadin 24.9.2
- **RPC Protocol:** gRPC 1.74.0 / Protocol Buffers 4.31.1
- **Database:** SQLite 3.50.3.0 with Hibernate JPA
- **Architecture:** Spring Modulith 1.4.3

### Key Dependencies
- **Lombok 1.18.42** - Boilerplate reduction
- **Spring gRPC 0.11.0** - gRPC integration
- **Thymeleaf** - HTML templating for reports
- **jsoup 1.21.2** - HTML parsing
- **Vaadin Barcode** - Barcode generation
- **Line Awesome** - Icon library

### Build & Distribution
- **JLink** - Custom JRE creation (ZIP-9 compression)
- **JPackage** - Application image packaging
- **Maven Assembly** - Distribution archives (Linux .tar.gz, Windows .zip)
- **Spotless** - Code formatting (Google Java Format)

---

## 2. Architecture Overview

### Module Structure

The project follows a **multi-module Maven architecture** with clear separation of concerns:

```
ez-booth/
├── core/              # Domain model & service interfaces (shared)
├── server/            # Backend implementation (gRPC services, JPA, reporting)
├── vaadin-ui/         # Frontend application (Vaadin web UI, gRPC client)
├── test/              # Shared test utilities
└── distribution/      # Packaging configuration
```

### Communication Architecture

```
┌─────────────────┐         ┌──────────────────┐
│   Vaadin UI     │         │   Server         │
│  (Frontend)     │◄───────►│  (Backend)       │
│                 │  gRPC   │                  │
│  - Web Browser  │         │  - Business      │
│  - UI Components│         │    Logic         │
│  - gRPC Client  │         │  - gRPC Services │
└─────────────────┘         │  - SQLite DB     │
                            │  - Report Gen    │
                            └──────────────────┘
```

### Key Architectural Patterns
- **Hexagonal Architecture** - Service interfaces in `core`, implementations in `server`
- **gRPC Service Layer** - Clean client-server separation
- **Repository Pattern** - JPA repositories for data access
- **Proto-based DTOs** - Type-safe data transfer via Protocol Buffers
- **Spring Modulith** - Module boundaries with event-driven communication

---

## 3. Data Model

### Core Entities (from Protobuf definitions)

#### **Booth** (`model.proto`)
- Primary entity representing a vendor booth/marketplace event
- Fields: ID, description, date, participation_fee, sales_fee, fees_rounding_step, closed status
- Manages financial configuration for the event

#### **Vendor** (`model.proto`)
- Represents a seller/vendor at a booth
- Composite key: BoothKey + vendorId
- Minimal structure (extensible design)

#### **Purchase** (`model.proto`)
- Transaction record for items sold
- Contains multiple PurchaseItems
- Fields: key, items[], value, purchased_on timestamp

#### **PurchaseItem** (`model.proto`)
- Line item in a purchase
- Links to vendor and purchase
- Fields: itemId, vendor, price, purchased_on

### Protobuf Services (182 lines total)

1. **BoothService** - CRUD operations for booth management
2. **PurchaseService** - Checkout and purchase history
3. **VendorService** - Vendor registration and retrieval
4. **ChargingService** - Fee calculation logic
5. **ReportingService** - Vendor report generation
6. **DataExchangeService** - Data sync/export/merge for multi-instance sync

---

## 4. Core Features Implementation

### 4.1 Booth Management
**Location:** `server/services/BoothGrpcService.java`, `vaadin-ui/views/BoothDetailsView.java`

- Create, read, update, delete booth configurations
- Open/close booth operations with timestamp tracking
- Configurable fee structures (participation fee, sales fee, rounding step)
- Date-based event tracking

### 4.2 Vendor Registration
**Location:** `server/services/VendorGrpcService.java`, `vaadin-ui/views/`

- Vendor enrollment per booth
- Unique vendor ID generation within booth scope
- Minimal vendor data (ID-based, extensible)

### 4.3 Point-of-Sale (Checkout)
**Location:** `services/PurchaseService.java`, `vaadin-ui/components/checkout/`

- Multi-item checkout with vendor attribution
- Price entry per item
- Timestamp tracking for audit trail
- Optional receipt printing capability
- Purchase history per booth

### 4.4 Financial Calculations
**Location:** `services/ChargingService.java`

- **Fee Calculation:**
  - Participation fee (flat rate per vendor)
  - Sales fee (percentage of total sales)
  - Configurable rounding steps
- **Balance Calculation:**
  - Total revenue computation
  - Fee breakdown per vendor
  - Net payout calculation

### 4.5 Reporting
**Location:** `server/reporting/`, `services/ReportingService.java`

- **Vendor Reports:**
  - HTML-based report generation (Thymeleaf templates)
  - Sales summary per vendor
  - Fee breakdown (participation + sales fees)
  - Total revenue calculation
  - Barcode integration for vendor identification
- **Report Output:** HTML files with embedded CSS styling
- **Report Storage:** File-based storage with URI references

### 4.6 Data Exchange & Synchronization
**Location:** `services/DataExchangeService.java`, `vaadin-ui/data/DataExchangeView.java`

- **Export:** Extract booth data (vendors, purchases) to portable format
- **Sync:** Bidirectional data exchange between instances
- **Merge:** Consolidate data from multiple booth instances
- Enables offline operation with later reconciliation

### 4.7 Persistence Layer
**Location:** `server/data/`, `server/model/`

- **Database:** SQLite with Hibernate JPA
- **Entity Mapping:** Proto-to-Entity conversion via `EntitiesMapper.java`
- **Repositories:** Spring Data JPA repositories
  - `BoothRepository`
  - `VendorRepository`
  - `PurchaseRepository`
- **Initialization:** Custom SQLite dialect configuration

---

## 5. User Interface

### Vaadin-Based Web UI
- **Framework:** Vaadin 24.9.2 (Java-based web framework)
- **Architecture:** Server-side rendering with client-side components
- **Styling:** Custom CSS + Line Awesome icons

### Main Views (Routes)
1. **EntryView** - Landing/dashboard
2. **BoothDetailsView** - Booth configuration and management
3. **VendorReportView** - Report generation interface
4. **VendorReportPrintView** - Print-optimized report display
5. **DataExchangeView** - Data sync/import/export interface

### UI Components
- **Checkout Components** (`ui/components/checkout/`) - POS interface
- **Model Components** - Data display/edit forms
- **Renderers** - Custom data presentation (ColumnRenderer, VendorRenderer)
- **Layouts** - App shell and navigation (`ui/layouts/app/`)

### UI Utilities
- Button helpers
- Badge components
- Routing utilities
- CSS unit helpers
- Constraint validation
- Delay utilities

---

## 6. Deployment & Distribution

### Packaging Strategy

#### 1. **JAR Distribution**
```bash
./mvnw clean package -Pproduction -pl server,vaadin-ui -am -DskipTests
```
- Produces Spring Boot executable JARs
- Separate server and UI artifacts
- Production-optimized Vaadin frontend build

#### 2. **JPackage Application Image**
```bash
./mvnw clean install -Pproduction -Pdist -DskipTests
```
- **JLink:** Creates minimal Java runtime (compressed with ZIP-9)
- **JPackage:** Bundles JAR + runtime into application image
- Custom runtime modules: `java.base`, `java.logging`, `java.desktop`, `java.management`, `java.naming`, `java.security.jgss`, `java.instrument`, `java.sql`, `java.xml`, `jdk.unsupported`, `java.net.http`, `jdk.zipfs`
- Launch mechanism: `-Djdk.lang.Process.launchMechanism=FORK`

#### 3. **Distribution Archives**
- **Linux:** `.tar.gz` archive
- **Windows:** `.zip` archive
- Assembly descriptors in `src/assembly/`

### Portability Features
- **Embedded runtime** - No Java installation required
- **Embedded database** - SQLite file-based storage
- **Self-contained** - All dependencies bundled
- **No installation** - Unzip and run

---

## 7. Code Quality & Standards

### Code Formatting
- **Spotless Maven Plugin** - Automated formatting enforcement
- **Google Java Format** - Consistent code style
- **License Headers** - Copyright notice on all files
- **Ratchet from origin/main** - Only format changed files

### Testing
- **Spring Boot Test** - Integration testing support
- **Spring Modulith Test** - Module boundary verification
- **Test Module** - Shared test utilities and test data

### Build Configuration
- **Maven Compiler:** Java 25 with `-parameters` flag
- **Annotation Processing:** Lombok support
- **Source Encoding:** UTF-8

---

## 8. Strengths of Current Implementation

### ✅ Well-Architected
- Clear module separation (core, server, UI)
- Service-oriented with gRPC contracts
- Domain-driven design with Protocol Buffers

### ✅ Portable & Self-Contained
- JPackage distribution with embedded runtime
- SQLite for zero-configuration persistence
- No external dependencies at runtime

### ✅ Modern Java Stack
- Latest Java 25, Spring Boot 3.5.6
- Spring Modulith for module boundaries
- Vaadin for rapid UI development

### ✅ Offline-First Design
- Local SQLite database
- File-based report generation
- Data export/sync capabilities

### ✅ Production-Ready Features
- Report generation with HTML/PDF output
- Barcode support for vendor tracking
- Fee calculation with configurable rounding
- Audit trail with timestamps

---

## 9. Challenges & Limitations

### ⚠️ Resource Intensity

#### Memory Footprint
- **Spring Boot:** Heavyweight framework (~100-200MB base memory)
- **Vaadin:** Server-side rendering requires JVM session per user
- **Hibernate/JPA:** ORM overhead for simple SQLite queries
- **gRPC:** Separate server process for backend services

#### Disk Space
- **Project Size:** 315MB (includes dependencies, build artifacts)
- **Runtime Distribution:** Estimated 50-100MB per platform (with JLink-optimized JRE)
- **JVM Overhead:** Full Java runtime even for small deployments

#### Startup Time
- **Spring Context:** Slow initialization (component scanning, bean creation)
- **Vaadin Frontend:** Build-time compilation required for production
- **JPA Schema:** Hibernate initialization overhead

### ⚠️ Complexity

#### Technology Stack
- **Multiple Frameworks:** Spring Boot + Vaadin + gRPC + Hibernate
- **Learning Curve:** Requires Java, Spring, Vaadin, Protocol Buffers knowledge
- **Build Process:** Multi-stage Maven build with profiles
- **Debugging:** Complex stack traces across multiple layers

#### Architecture
- **gRPC Overhead:** Overkill for single-process scenarios
- **Proto Mapping:** Dual model layer (Proto DTOs ↔ JPA Entities)
- **Module Coordination:** Spring Modulith adds complexity for small teams

### ⚠️ Platform Dependencies

#### Java 25 Requirement
- **Bleeding Edge:** Java 25 is not LTS (Last LTS: Java 21)
- **Compatibility:** May face tooling/library compatibility issues
- **Deployment:** Requires recent JPackage version

#### JPackage Limitations
- **Platform-Specific:** Must build on target platform (Linux build for Linux, etc.)
- **Update Process:** Full application replacement for updates
- **Size:** Each platform needs separate distribution

### ⚠️ Scalability Concerns

#### Multi-Instance Sync
- **Manual Sync:** File-based or network-based data exchange (not automated)
- **Conflict Resolution:** Unclear merge strategy for concurrent edits
- **Data Consistency:** No CRDT or vector clock implementation evident

#### Concurrent Access
- **SQLite Limitations:** Single-writer database (no concurrent write performance)
- **File Locking:** Potential issues with network file systems

### ⚠️ Web Technology Mismatch
- **Server-Side UI:** Vaadin requires constant server connection
- **Session State:** Stateful server sessions don't align with "portable, offline" goal
- **Browser Dependency:** Requires modern web browser on client machine

---

## 10. Migration Opportunities for Rust/WASM

### 🎯 High-Impact Improvements

#### 1. **Binary Size Reduction**
- **Current:** ~50-100MB with JLink runtime
- **Rust Target:** ~5-15MB statically-linked binary
- **WASM Target:** ~1-3MB for browser-based UI
- **Benefit:** 10-50x reduction in distribution size

#### 2. **Memory Efficiency**
- **Current:** 100-200MB+ JVM heap + metaspace
- **Rust Target:** 5-20MB typical RSS for equivalent functionality
- **Benefit:** 10-20x reduction in runtime memory

#### 3. **Startup Performance**
- **Current:** 5-15 seconds (Spring Boot + Vaadin initialization)
- **Rust Target:** <100ms for native binary
- **Benefit:** 50-150x faster startup

#### 4. **True Offline Operation**
- **Current:** Requires server process + browser
- **WASM Target:** Full application runs in browser (offline PWA)
- **Benefit:** No server process needed, works offline in browser

#### 5. **Cross-Platform Consistency**
- **Current:** Platform-specific JPackage builds
- **Rust Target:** Single WASM binary runs on all platforms with browser
- **Benefit:** Build once, run everywhere (truly portable)

### 🔧 Technical Migration Paths

#### Backend: Java/Spring → Rust
```
Spring Boot Services  →  Axum/Actix-Web (HTTP API)
gRPC Services         →  Tonic (gRPC) or REST API
Hibernate/JPA         →  Diesel/SQLx (SQLite)
Thymeleaf Reports     →  Tera/Handlebars templates
```

#### Frontend: Vaadin → WASM
```
Vaadin (Java)         →  Leptos/Yew/Dioxus (Rust WASM)
Server-side rendering →  Client-side WASM
Java components       →  Rust components
```

#### Data Layer
```
Proto + JPA Entities  →  Single Rust struct layer (Serde)
Hibernate ORM         →  SQLx/Diesel (compile-time query checking)
Manual mapping        →  Derive macros (zero-cost abstractions)
```

### 🎨 Architecture Redesign

#### Proposed WASM Architecture
```
┌─────────────────────────────────────┐
│         Browser                     │
│  ┌───────────────────────────────┐  │
│  │  Rust WASM Application        │  │
│  │  ┌─────────────────────────┐  │  │
│  │  │  UI (Leptos/Yew)        │  │  │
│  │  │  - Components           │  │  │
│  │  │  - Routing              │  │  │
│  │  │  - State Management     │  │  │
│  │  └─────────────────────────┘  │  │
│  │  ┌─────────────────────────┐  │  │
│  │  │  Business Logic (Rust)  │  │  │
│  │  │  - Services             │  │  │
│  │  │  - Domain Model         │  │  │
│  │  │  - Calculations         │  │  │
│  │  └─────────────────────────┘  │  │
│  │  ┌─────────────────────────┐  │  │
│  │  │  Storage Layer          │  │  │
│  │  │  - IndexedDB (WASM)     │  │  │
│  │  │  - Local Storage        │  │  │
│  │  │  - Export/Import        │  │  │
│  │  └─────────────────────────┘  │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

**Key Benefits:**
- **No Server:** Entire application runs in browser
- **True Offline:** Browser-based storage (IndexedDB)
- **Portable:** Single HTML + WASM file
- **Fast:** Native performance in browser

#### Hybrid Architecture (Optional Backend)
```
┌──────────────┐         ┌─────────────────┐
│  WASM UI     │◄───────►│  Rust Server    │
│  (Browser)   │  HTTP   │  (Optional)     │
│              │         │  - Sync         │
│  - Full App  │         │  - Reports      │
│  - Offline   │         │  - Backup       │
└──────────────┘         └─────────────────┘
```

### 🔑 Core Capabilities to Replicate

#### 1. Data Model
```rust
// Equivalent to Proto definitions
#[derive(Serialize, Deserialize, Clone)]
struct Booth {
    id: String,
    description: String,
    date: NaiveDate,
    participation_fee: f32,
    sales_fee: f32,
    fees_rounding_step: f32,
    closed: bool,
    closed_on: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Vendor {
    booth_id: String,
    vendor_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Purchase {
    id: String,
    booth_id: String,
    items: Vec<PurchaseItem>,
    value: f32,
    purchased_on: DateTime<Utc>,
}
```

#### 2. Storage Options

**WASM (Browser):**
- **IndexedDB:** Structured storage via `web-sys` bindings
- **Local Storage:** Simple key-value pairs
- **Export:** JSON/binary serialization to files

**Native (Optional):**
- **SQLite:** Via `rusqlite` or `sqlx`
- **File-based:** JSON/CBOR serialization
- **In-memory:** `HashMap`-based for testing

#### 3. UI Framework Comparison

| Feature | Vaadin (Java) | Leptos (Rust) | Yew (Rust) | Dioxus (Rust) |
|---------|--------------|---------------|------------|---------------|
| **WASM** | ❌ | ✅ | ✅ | ✅ |
| **Reactivity** | Server-side | Fine-grained signals | Virtual DOM | Virtual DOM |
| **SSR** | ✅ | ✅ | ❌ | ✅ |
| **Learning Curve** | Medium | Medium | Medium | Low |
| **Bundle Size** | N/A | ~150KB | ~200KB | ~250KB |
| **Performance** | Server-bound | Native WASM | Native WASM | Native WASM |

**Recommendation:** **Leptos** for modern reactive UI with small bundle size

#### 4. Report Generation

**Current:** Server-side HTML generation with Thymeleaf  
**Rust Options:**
- **Browser-side:** Generate HTML in WASM, use `window.print()`
- **PDF Generation:** `printpdf` or `genpdf` (compile to WASM)
- **Template Engine:** `tera` or `handlebars-rust`

#### 5. Data Synchronization

**Challenge:** Multi-instance offline sync  
**Rust Solutions:**
- **CRDT Libraries:** `automerge-rs`, `yrs` (Yjs in Rust)
- **Event Sourcing:** Append-only logs with merge
- **Custom Protocol:** File-based export/import with conflict resolution
- **WebRTC:** Peer-to-peer sync (no server)

---

## 11. Recommended Implementation Strategy

### Phase 1: Foundation (Weeks 1-2)
1. **Data Model:** Define Rust structs with Serde serialization
2. **Storage:** Implement IndexedDB wrapper for WASM
3. **Core Services:** Port business logic (fees, calculations)
4. **Testing:** Unit tests for domain logic

### Phase 2: UI Core (Weeks 3-4)
1. **Framework Setup:** Initialize Leptos/Yew project
2. **Component Library:** Basic UI components (forms, tables, buttons)
3. **Routing:** Navigation between views
4. **State Management:** Global state with signals/context

### Phase 3: Features (Weeks 5-6)
1. **Booth Management:** CRUD operations
2. **Vendor Registration:** Add/edit vendors
3. **Checkout Flow:** POS interface
4. **Purchase History:** Display transactions

### Phase 4: Reports & Sync (Weeks 7-8)
1. **Report Generation:** HTML reports with print support
2. **Data Export/Import:** JSON/CBOR serialization
3. **Sync Protocol:** File-based or network-based
4. **Barcode Integration:** WASM-compatible barcode library

### Phase 5: Polish (Weeks 9-10)
1. **Styling:** CSS framework (Tailwind, etc.)
2. **PWA Features:** Service worker, offline manifest
3. **Testing:** Integration tests, E2E tests
4. **Documentation:** User guide, API docs

### Phase 6: Distribution (Week 11-12)
1. **Build Optimization:** Minimize WASM bundle size
2. **Packaging:** Single-page application bundle
3. **Deployment:** Static hosting or desktop wrapper (Tauri)
4. **Migration Tools:** Java→Rust data migration scripts

---

## 12. Risk Assessment

### 🟢 Low Risk
- **Core business logic** - Straightforward to port
- **Data model** - Simple structs with serialization
- **Calculations** - Pure functions, easy to test
- **UI components** - Well-defined in current implementation

### 🟡 Medium Risk
- **Browser compatibility** - WASM requires modern browsers (2-3 years old)
- **IndexedDB complexity** - Async API with quirks
- **Report generation** - PDF libraries less mature in Rust/WASM
- **Learning curve** - Team familiarity with Rust/WASM

### 🔴 High Risk
- **Data synchronization** - Complex conflict resolution needed
- **Barcode printing** - Browser printing APIs vary by platform
- **Migration path** - Must support existing Java data export format
- **Feature parity** - Ensuring all Vaadin features replicated

---

## 13. Success Metrics

### Performance
- **Startup time:** <500ms (vs. 5-15s current)
- **Memory usage:** <50MB (vs. 200MB+ current)
- **Distribution size:** <5MB (vs. 50-100MB current)
- **Offline capability:** 100% functional without network

### User Experience
- **Installation:** Single file download (HTML + WASM)
- **Platform support:** Any modern browser (Windows, Mac, Linux, mobile)
- **Sync reliability:** Conflict-free merge for 95%+ of cases

### Development
- **Build time:** <30s for full rebuild (vs. 2-3 min Maven)
- **Type safety:** Compile-time query checking (SQLx)
- **Test coverage:** >80% for business logic

---

## 14. Conclusion

The current `ez-booth` implementation is a **well-architected, production-ready Java application** with solid domain modeling and feature completeness. However, its reliance on Spring Boot, Vaadin, and the JVM creates **resource intensity** and **deployment complexity** that conflict with the goals of a truly portable, offline-first application.

**Migration to Rust + WebAssembly offers transformative improvements:**
- **10-50x smaller** distribution size
- **10-20x lower** memory usage  
- **50-150x faster** startup
- **True browser-based offline** operation (no server process)
- **Simpler deployment** (single HTML + WASM file)

The **primary technical challenges** are:
1. Implementing robust multi-instance sync (CRDT or custom protocol)
2. Browser-based report generation/printing
3. IndexedDB storage layer
4. Team ramp-up on Rust/WASM ecosystem

**Recommendation:** Proceed with Rust/WASM redesign using **incremental migration strategy** (Phase 1-6 plan), starting with core domain logic and expanding to UI, then advanced features. Maintain Java version for reference and data migration support.

---

## Appendices

### A. File Counts
- **Total Java Files:** 155
- **Lines of Code:** ~12,191
- **Protobuf Definitions:** 3 files, 182 lines
- **Project Size:** 315MB

### B. Key Files Reference
- **Core Domain:** `core/src/main/java/tschuba/ez/booth/services/`
- **Backend Services:** `server/src/main/java/tschuba/ez/booth/services/`
- **UI Views:** `vaadin-ui/src/main/java/tschuba/ez/booth/ui/views/`
- **Proto Definitions:** `core/src/main/protobuf/*.proto`
- **Report Templates:** `server/src/main/resources/reports/templates/`

### C. Build Commands
```bash
# Development build
./mvnw clean package

# Production JARs
./mvnw clean package -Pproduction -pl server,vaadin-ui -am -DskipTests

# JPackage distribution
./mvnw clean install -Pproduction -Pdist -DskipTests

# Distribution archives
./mvnw verify -DskipTests -pl .
```

---

**Document Version:** 1.0  
**Last Updated:** March 19, 2026
