---
title: Architecture Design
nav_order: 4
parent: Redesign
---

# ez-vend-rs Architecture Design

---
**Document Status:** Active Reference
**Last Updated:** 2026-04-04
**Purpose:** Detailed architecture design document for the Rust/WASM redesign.

---

**Document Version:** 1.0  
**Date:** March 19, 2026  
**Status:** Design Phase  
**Related Documents:** [00_SPEC.md](00_SPEC.md), [01_ANALYSIS.md](01_ANALYSIS.md), [REDESIGN_SUMMARY.md](REDESIGN_SUMMARY.md)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Design Principles](#design-principles)
3. [System Architecture](#system-architecture)
4. [Technology Stack](#technology-stack)
5. [Module Structure](#module-structure)
6. [Data Architecture](#data-architecture)
7. [User Interface Design](#user-interface-design)
8. [Deployment Models](#deployment-models)
9. [Cross-Browser Data Portability](#9-cross-browser-data-portability)
10. [Internationalization (i18n)](#10-internationalization-i18n)
11. [Data Migration from ez-booth](#11-data-migration-from-ez-booth)
12. [Security & Privacy](#12-security--privacy)
13. [Error Handling & User Support](#13-error-handling--user-support)
14. [Performance Targets](#14-performance-targets)
15. [Appendices](#appendices)

---

## Executive Summary

`ez-vend-rs` adopts a **WASM-first, offline-capable architecture** built entirely in Rust. The design eliminates the need for a traditional server process by running the complete application in the browser via WebAssembly, with optional backend services for advanced deployment scenarios.

### Key Architectural Decisions

1. **WASM-Native Application** - Full application logic runs in browser
2. **IndexedDB Storage** - Browser-based persistent storage
3. **Optional Backend** - Rust server for client-server deployments
4. **Modular Design** - Clean separation between core, UI, and storage layers
5. **Progressive Web App** - Installable, offline-capable web application

---

## Design Principles

### 1. Offline-First
- **Primary Goal:** Application must work without network connectivity
- **Strategy:** All data stored locally in browser (IndexedDB)
- **Sync:** Optional synchronization when network available

### 2. Minimal Resource Footprint
- **Target Binary Size:** <3MB WASM + <500KB JavaScript glue
- **Target Memory:** <50MB runtime memory usage
- **Target Startup:** <500ms time-to-interactive

### 3. Zero Installation Friction
- **Access Method:** Open URL in browser
- **PWA Installation:** Optional one-click install for offline access
- **No Prerequisites:** Works on any modern browser (2020+)

### 4. Cross-Platform Consistency
- **Single Codebase:** Same WASM binary for all platforms
- **Responsive UI:** Adapts to desktop, tablet, mobile
- **Browser Compatibility:** Chrome 90+, Firefox 88+, Safari 14+, Edge 90+

### 5. Type Safety & Performance
- **Compile-Time Guarantees:** Rust's type system prevents common errors
- **Zero-Cost Abstractions:** No runtime overhead for abstractions
- **Memory Safety:** No garbage collection pauses

### 6. Extensibility
- **Plugin Architecture:** Support for custom reports, sync protocols
- **Data Export:** Multiple formats (JSON, CSV, custom)
- **API Surface:** Clear boundaries for feature extensions

---

## System Architecture

### 3.1 Deployment Model 1: Browser-Only (Primary)

```
┌─────────────────────────────────────────────────────┐
│                    Browser                          │
│  ┌───────────────────────────────────────────────┐  │
│  │         ez-vend-rs WASM Application          │  │
│  │                                               │  │
│  │  ┌─────────────────────────────────────────┐ │  │
│  │  │         Presentation Layer              │ │  │
│  │  │  ┌────────────┐  ┌─────────────────┐   │ │  │
│  │  │  │  UI (Leptos)│  │  Components     │   │ │  │
│  │  │  │  - Views   │  │  - Forms        │   │ │  │
│  │  │  │  - Routing │  │  - Tables       │   │ │  │
│  │  │  │  - State   │  │  - Reports      │   │ │  │
│  │  │  └────────────┘  └─────────────────┘   │ │  │
│  │  └─────────────────────────────────────────┘ │  │
│  │                                               │  │
│  │  ┌─────────────────────────────────────────┐ │  │
│  │  │         Business Logic Layer            │ │  │
│  │  │  ┌────────────┐  ┌─────────────────┐   │ │  │
│  │  │  │  Services  │  │  Domain Model   │   │ │  │
│  │  │  │  - Booth   │  │  - Entities     │   │ │  │
│  │  │  │  - Vendor  │  │  - Value Objs   │   │ │  │
│  │  │  │  - Purchase│  │  - Calculations │   │ │  │
│  │  │  │  - Reports │  │  - Validation   │   │ │  │
│  │  │  └────────────┘  └─────────────────┘   │ │  │
│  │  └─────────────────────────────────────────┘ │  │
│  │                                               │  │
│  │  ┌─────────────────────────────────────────┐ │  │
│  │  │         Data Access Layer               │ │  │
│  │  │  ┌────────────┐  ┌─────────────────┐   │ │  │
│  │  │  │ Repository │  │  IndexedDB      │   │ │  │
│  │  │  │ Pattern    │  │  Wrapper        │   │ │  │
│  │  │  │ - CRUD     │  │  - Async API    │   │ │  │
│  │  │  │ - Queries  │  │  - Transactions │   │ │  │
│  │  │  └────────────┘  └─────────────────┘   │ │  │
│  │  └─────────────────────────────────────────┘ │  │
│  │                                               │  │
│  │  ┌─────────────────────────────────────────┐ │  │
│  │  │         Storage Layer                   │ │  │
│  │  │    ┌──────────┐    ┌────────────┐      │ │  │
│  │  │    │IndexedDB │    │LocalStorage│      │ │  │
│  │  │    │(Primary) │    │(Settings)  │      │ │  │
│  │  │    └──────────┘    └────────────┘      │ │  │
│  │  └─────────────────────────────────────────┘ │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

**Characteristics:**
- ✅ No server required
- ✅ Complete offline operation
- ✅ Instant deployment (static files)
- ✅ Minimal hosting cost (static CDN)
- ⚠️ Data sync requires file export/import or peer-to-peer

### 3.2 Deployment Model 2: Client-Server (Optional)

```
┌──────────────────────┐         ┌─────────────────────┐
│   Browser Client     │         │   Rust Server       │
│  ┌────────────────┐  │         │  ┌──────────────┐   │
│  │ WASM UI        │  │◄───────►│  │ Axum/Actix   │   │
│  │ - Components   │  │  HTTP/  │  │ Web Server   │   │
│  │ - Local Cache  │  │  WS     │  └──────────────┘   │
│  └────────────────┘  │         │  ┌──────────────┐   │
│  ┌────────────────┐  │         │  │ Services     │   │
│  │ IndexedDB      │  │         │  │ - Sync       │   │
│  │ (Offline Cache)│  │         │  │ - Reports    │   │
│  └────────────────┘  │         │  │ - Export     │   │
└──────────────────────┘         │  └──────────────┘   │
                                 │  ┌──────────────┐   │
                                 │  │ Storage      │   │
                                 │  │ - SQLite     │   │
                                 │  │ - PostgreSQL │   │
                                 │  └──────────────┘   │
                                 └─────────────────────┘
```

**Characteristics:**
- ✅ Centralized data management
- ✅ Multi-user collaboration
- ✅ Automatic synchronization
- ✅ Backend report generation (PDF)
- ⚠️ Requires server infrastructure
- ⚠️ Needs network connectivity

### 3.3 Deployment Model 3: Hybrid (Best of Both)

```
┌──────────────────────┐         ┌─────────────────────┐
│   Browser Client     │         │  Optional Server    │
│  ┌────────────────┐  │         │  (When Available)   │
│  │ Full WASM App  │  │   ╔════►│  ┌──────────────┐   │
│  │ (Standalone)   │  │   ║     │  │ Sync Service │   │
│  └────────────────┘  │   ║     │  └──────────────┘   │
│  ┌────────────────┐  │   ║     │  ┌──────────────┐   │
│  │ IndexedDB      │  │───╝     │  │ Backup Store │   │
│  │ (Primary)      │  │ Sync    │  └──────────────┘   │
│  └────────────────┘  │ Agent   └─────────────────────┘
└──────────────────────┘
     │
     └──► Works offline, syncs when online
```

**Characteristics:**
- ✅ Works completely offline
- ✅ Optional background sync when server available
- ✅ Degrades gracefully without server
- ✅ Best user experience
- 🎯 **Recommended for ez-vend-rs**

---

## Technology Stack

### 4.1 Frontend (WASM)

#### UI Framework: **Leptos 0.6+**
**Rationale:**
- Fine-grained reactivity with signals (minimal re-renders)
- Smallest WASM bundle size (~150KB after optimization)
- Server-side rendering support for future enhancements
- Excellent TypeScript-like type inference
- Active community and development

**Alternatives Considered:**
- **Yew:** More mature but larger bundle size (~200KB), virtual DOM overhead
- **Dioxus:** React-like API, good for React developers, slightly larger bundle
- **Sycamore:** Similar to Leptos but less adoption

#### CSS Framework: **Tailwind CSS**
**Rationale:**
- Utility-first approach for rapid development
- Excellent purging (only used classes in output)
- Small production bundle (~10-30KB)
- Great responsive design primitives

#### Component Library: **Custom + Leptos UI Primitives**
**Rationale:**
- Full control over bundle size
- Tailored to ez-booth use cases
- No unnecessary dependencies

### 4.2 Data Layer (WASM)

#### Storage: **IndexedDB via rexie 0.6+**
**Rationale:**
- Native browser storage (persistent across sessions)
- Structured data with indexes (fast queries)
- Async API (non-blocking)
- Transactions for consistency
- ~100MB+ storage quota (browser-dependent)

**Schema Design:**
```rust
// Database: ez_vend_db
// Stores:
//   - booths (key: booth_id)
//   - vendors (key: [booth_id, vendor_id])
//   - purchases (key: [booth_id, purchase_id])
//   - purchase_items (key: [booth_id, purchase_id, item_id])
```

#### Serialization: **Serde + serde_json/bincode**
**Rationale:**
- Zero-copy deserialization where possible
- JSON for export/import (human-readable)
- Bincode for efficient storage (smaller size)

### 4.3 Backend (Optional Server)

#### Web Framework: **Axum 0.7+**
**Rationale:**
- Tokio-based async runtime (excellent performance)
- Type-safe routing and extractors
- Modular middleware system
- Small binary size (~5-10MB)
- Active development by Tokio team

**Alternatives:**
- **Actix-Web:** Faster benchmarks but more complex API
- **Rocket:** Good DX but slower compile times

#### Database: **SQLite (rusqlite) or PostgreSQL (sqlx)**
**Rationale:**
- **SQLite:** Zero-config, file-based, perfect for single-server deployments
- **PostgreSQL:** Production-grade, multi-user, better concurrency

#### ORM/Query Builder: **sqlx 0.7+**
**Rationale:**
- Compile-time query validation
- Async/await support
- No runtime overhead
- Direct SQL control

### 4.4 Shared Libraries

#### Core Dependencies (All Phases)

```toml
[dependencies]
leptos = "0.6"                  # UI framework
leptos_i18n = "0.3"            # Internationalization (DE/EN)
rexie = "0.6"                   # IndexedDB wrapper
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"             # JSON serialization
wasm-bindgen = "0.2"           # WASM-JS bridge
web-sys = "0.3"                # Browser APIs
js-sys = "0.1"                 # JavaScript types
rust_decimal = "1.34"          # Currency precision
chrono = "0.4"                 # Date/time handling
uuid = "1.7"                   # Unique identifiers
```

**Rationale:**
- Essential dependencies for production quality
- rust_decimal prevents floating-point errors in financial calculations
- chrono provides consistent date handling and timezone support
- i18n support built-in from start for German/English localization

#### Optional Dependencies (Server Features)

- **reqwest** → Only if cloud sync implemented
- **tokio** → Server runtime
- **axum** → Server framework
- **sqlx** → Database driver

### 4.5 Development Tools

#### Build Tool: **Trunk 0.19+**
**Rationale:**
- WASM-specific build tool
- Hot reload during development
- Asset bundling (CSS, images)
- Optimized production builds

#### Testing: **wasm-bindgen-test**
**Rationale:**
- Browser-based test runner
- Tests run in actual browser environment

#### Formatting: **rustfmt**
**Rationale:**
- Official Rust formatter
- Consistent code style

#### Linting: **clippy**
**Rationale:**
- Official Rust linter
- Catches common mistakes and anti-patterns

---

## Module Structure

### 5.1 Workspace Layout

```
ez-vend-rs/
├── Cargo.toml              # Workspace root
├── locales/                # i18n translations (NEW)
│   ├── de.json            # German (primary)
│   ├── en.json            # English (fallback)
│   └── translations.json  # Config
├── crates/
│   ├── core/               # Domain model & business logic (shared)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── entities/   # Domain entities
│   │       ├── services/   # Business logic
│   │       ├── validation/ # Validation rules
│   │       └── error.rs    # Error types
│   │
│   ├── storage/            # Storage abstraction layer
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── repository/ # Repository traits
│   │       ├── indexeddb/  # IndexedDB implementation
│   │       └── sql/        # SQL implementation (optional)
│   │
│   ├── frontend/           # WASM UI application
│   │   ├── Cargo.toml
│   │   ├── index.html
│   │   ├── Trunk.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── components/ # UI components
│   │       ├── pages/      # Page views
│   │       ├── state/      # Global state management
│   │       ├── i18n/       # i18n setup & formatters (NEW)
│   │       └── api/        # Backend API client
│   │
│   ├── server/             # Optional backend (feature-gated)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── handlers/   # HTTP handlers
│   │       ├── middleware/ # Auth, logging, etc.
│   │       └── sync/       # Sync protocol
│   │
│   └── shared/             # Shared types for client-server
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── dto/        # Data transfer objects
│           └── protocol/   # Sync protocol definitions
│
├── docs/                   # Documentation
├── tests/                  # Integration tests
└── scripts/                # Build/deploy scripts
```

### 5.2 Dependency Graph

```
        ┌─────────┐
        │  core   │ (no dependencies)
        └─────────┘
         ▲       ▲
         │       │
         │       └──────────────┐
         │                      │
    ┌────┴────┐           ┌─────┴───┐
    │ storage │           │ shared  │
    └─────────┘           └─────────┘
         ▲                      ▲
         │                      │
         └──────────┬───────────┘
                    │
              ┌─────┴─────┐
              │  frontend │
              └───────────┘
```

**Dependency Rules:**
- `core` has no dependencies (pure business logic with domain models)
- `storage` depends on `core` (implements persistence for core entities)
- `shared` depends on `core` (defines serializable DTOs from core types)
- `frontend` depends on `core`, `storage`, `shared` (UI layer using all others)
- `server` (future) would depend on `core`, `storage`, `shared`

**Note:** Server crate is not part of P0 implementation (web-only phase).

---

## Data Architecture

### 6.1 Domain Model

#### Entities

```rust
// crates/core/src/entities/booth.rs

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Booth {
    pub id: BoothId,
    pub description: String,
    pub date: NaiveDate,
    pub fees: FeeConfig,
    pub status: BoothStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeConfig {
    pub participation_fee: Decimal,  // Use rust_decimal for currency
    pub sales_fee_percent: Decimal,
    pub rounding_step: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BoothStatus {
    Open,
    Closed { closed_at: DateTime<Utc> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vendor {
    pub id: VendorId,          // String-based ID (e.g., "V123", "42")
    pub booth_id: BoothId,
    pub created_at: DateTime<Utc>,
    // Minimal data - vendors created dynamically during checkout
    // Extensible design - can add name, contact, etc. later
}

// Vendor ID Design Rationale:
// - String-based to support both numeric ("1", "42") and alphanumeric ("V123", "A5") IDs
// - User enters vendor ID attached to sold products during checkout
// - Most common: purely numeric IDs for simplicity
// - Smart sorting ensures numeric IDs sort correctly (1, 2, 10 not 1, 10, 2)
// - Critical for vendor report printing order

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Purchase {
    pub id: PurchaseId,
    pub booth_id: BoothId,
    pub items: Vec<PurchaseItem>,
    pub total: Decimal,
    pub purchased_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseItem {
    pub id: ItemId,
    pub vendor_id: VendorId,
    pub price: Decimal,
    pub purchased_at: DateTime<Utc>,
}

// Value objects for type safety
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BoothId(Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VendorId(String);  // User-provided string (e.g., "V123", "42")

impl VendorId {
    /// Smart sorting: numeric IDs sorted numerically, alphanumeric sorted lexicographically
    pub fn compare_smart(&self, other: &VendorId) -> std::cmp::Ordering {
        // Try parsing both as integers
        match (self.0.parse::<u64>(), other.0.parse::<u64>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),     // Both numeric: compare numerically
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,   // Numeric before alphanumeric
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater, // Alphanumeric after numeric
            (Err(_), Err(_)) => self.0.cmp(&other.0),  // Both alphanumeric: lexicographic
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PurchaseId(Uuid);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(Uuid);
```

#### Services

```rust
// crates/core/src/services/booth_service.rs

pub trait BoothService {
    async fn create_booth(&self, description: String, date: NaiveDate, fees: FeeConfig) 
        -> Result<Booth, BoothError>;
    
    async fn get_booth(&self, id: BoothId) -> Result<Booth, BoothError>;
    
    async fn list_booths(&self) -> Result<Vec<Booth>, BoothError>;
    
    async fn update_booth(&self, id: BoothId, booth: Booth) -> Result<(), BoothError>;
    
    async fn close_booth(&self, id: BoothId) -> Result<Booth, BoothError>;
    
    async fn delete_booth(&self, id: BoothId) -> Result<(), BoothError>;
}

// crates/core/src/services/charging_service.rs

pub struct ChargingService;

impl ChargingService {
    pub fn calculate_fees(
        &self, 
        total_sales: Decimal, 
        config: &FeeConfig
    ) -> FeeCalculation {
        let participation = config.participation_fee;
        let sales_fee = total_sales * config.sales_fee_percent / Decimal::from(100);
        
        // Apply rounding
        let sales_fee_rounded = Self::round_to_step(sales_fee, config.rounding_step);
        
        FeeCalculation {
            participation_fee: participation,
            sales_fee: sales_fee_rounded,
            total_fees: participation + sales_fee_rounded,
            net_revenue: total_sales - participation - sales_fee_rounded,
        }
    }
    
    fn round_to_step(value: Decimal, step: Decimal) -> Decimal {
        if step.is_zero() {
            value
        } else {
            (value / step).round() * step
        }
    }
}

// crates/core/src/services/purchase_service.rs

pub trait PurchaseService {
    /// Process checkout with multiple items.
    /// Vendors are created automatically if they don't exist.
    async fn checkout(
        &self,
        booth_id: BoothId,
        items: Vec<CheckoutItem>,
    ) -> Result<Purchase, PurchaseError>;
    
    async fn get_purchase(&self, id: PurchaseId) -> Result<Purchase, PurchaseError>;
    
    async fn list_purchases(&self, booth_id: BoothId) -> Result<Vec<Purchase>, PurchaseError>;
}

#[derive(Clone, Debug)]
pub struct CheckoutItem {
    pub vendor_id: String,  // User-entered vendor ID (e.g., "V123")
    pub price: Decimal,
    pub purchased_at: DateTime<Utc>,
}

// crates/core/src/services/vendor_service.rs

pub trait VendorService {
    /// Get or create a vendor by ID.
    /// If vendor doesn't exist, creates it automatically.
    async fn get_or_create(
        &self,
        booth_id: BoothId,
        vendor_id: String,
    ) -> Result<Vendor, VendorError>;
    
    /// List vendors with smart sorting.
    /// Numeric IDs (e.g., "1", "42") sorted numerically: 1, 2, 10, 42
    /// Alphanumeric IDs sorted lexicographically after numeric IDs
    /// Critical for correct print order in vendor reports.
    async fn list_vendors(&self, booth_id: BoothId) -> Result<Vec<Vendor>, VendorError>;
    
    async fn get_vendor_sales(
        &self,
        booth_id: BoothId,
        vendor_id: String,
    ) -> Result<VendorSalesReport, VendorError>;
}

#[derive(Clone, Debug)]
pub struct VendorSalesReport {
    pub vendor: Vendor,
    pub items: Vec<PurchaseItem>,
    pub total_sales: Decimal,
    pub fees: FeeCalculation,
    pub net_revenue: Decimal,
}
```

### 6.2 Storage Schema

#### IndexedDB Schema (Browser)

```
Database: ez_vend_v1

Object Stores:

1. booths
   - keyPath: "id"
   - indexes: ["date", "status"]

2. vendors
   - keyPath: ["booth_id", "id"]
   - indexes: ["booth_id"]

3. purchases
   - keyPath: ["booth_id", "id"]
   - indexes: ["booth_id", "purchased_at"]

4. purchase_items
   - keyPath: ["booth_id", "purchase_id", "id"]
   - indexes: ["booth_id", "vendor_id", "purchased_at"]

5. settings
   - keyPath: "key"
   - stores: UI preferences, sync config
```

#### SQLite Schema (Server - Optional)

```sql
-- Enable foreign keys
PRAGMA foreign_keys = ON;

CREATE TABLE booths (
    id TEXT PRIMARY KEY,
    description TEXT NOT NULL,
    date TEXT NOT NULL,  -- ISO 8601 date
    participation_fee TEXT NOT NULL,  -- Decimal as string
    sales_fee_percent TEXT NOT NULL,
    rounding_step TEXT NOT NULL,
    status TEXT NOT NULL,  -- 'open' or 'closed'
    closed_at TEXT,  -- ISO 8601 timestamp
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE vendors (
    id TEXT NOT NULL,
    booth_id TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (booth_id, id),
    FOREIGN KEY (booth_id) REFERENCES booths(id) ON DELETE CASCADE
);

CREATE TABLE purchases (
    id TEXT NOT NULL,
    booth_id TEXT NOT NULL,
    total TEXT NOT NULL,  -- Decimal as string
    purchased_at TEXT NOT NULL,
    PRIMARY KEY (booth_id, id),
    FOREIGN KEY (booth_id) REFERENCES booths(id) ON DELETE CASCADE
);

CREATE TABLE purchase_items (
    id TEXT NOT NULL,
    booth_id TEXT NOT NULL,
    purchase_id TEXT NOT NULL,
    vendor_id TEXT NOT NULL,
    price TEXT NOT NULL,  -- Decimal as string
    purchased_at TEXT NOT NULL,
    PRIMARY KEY (booth_id, purchase_id, id),
    FOREIGN KEY (booth_id, purchase_id) REFERENCES purchases(booth_id, id) ON DELETE CASCADE,
    FOREIGN KEY (booth_id, vendor_id) REFERENCES vendors(booth_id, id)
);

-- Indexes for common queries
CREATE INDEX idx_vendors_booth ON vendors(booth_id);
CREATE INDEX idx_purchases_booth ON purchases(booth_id);
CREATE INDEX idx_purchases_date ON purchases(purchased_at);
CREATE INDEX idx_items_vendor ON purchase_items(booth_id, vendor_id);
CREATE INDEX idx_items_date ON purchase_items(purchased_at);
```

### 6.3 Vendor ID Sorting Strategy

**Requirement:** Vendors must be sortable correctly for report printing, especially when IDs are numeric.

**Problem:**
- Lexicographic sorting: "1", "10", "2", "25", "3" → incorrect order
- Numeric sorting: "1", "2", "3", "10", "25" → correct order
- Mixed IDs: Need to handle both "42" (numeric) and "V123" (alphanumeric)

**Solution: Smart Natural Sorting**

```rust
impl VendorId {
    /// Compare vendor IDs intelligently:
    /// - Pure numeric IDs (e.g., "1", "42"): Compare numerically
    /// - Alphanumeric IDs (e.g., "V123", "A5"): Compare lexicographically
    /// - Mixed: Numeric IDs always sort before alphanumeric
    pub fn compare_smart(&self, other: &VendorId) -> std::cmp::Ordering {
        match (self.0.parse::<u64>(), other.0.parse::<u64>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),                    // Both numeric: 1 < 2 < 10
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,    // Numeric before alphanumeric
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater, // Alphanumeric after numeric
            (Err(_), Err(_)) => self.0.cmp(&other.0),      // Both text: lexicographic
        }
    }
}

// Usage in VendorService
impl VendorService {
    async fn list_vendors(&self, booth_id: BoothId) -> Result<Vec<Vendor>, VendorError> {
        let mut vendors = self.repository.get_all(booth_id).await?;
        vendors.sort_by(|a, b| a.id.compare_smart(&b.id));
        Ok(vendors)
    }
}
```

**Examples:**

| Input IDs | Sorted Output | Notes |
|-----------|---------------|-------|
| "10", "2", "1", "3" | "1", "2", "3", "10" | Numeric sorting |
| "V10", "V2", "V1" | "V1", "V10", "V2" | Lexicographic (standard) |
| "1", "10", "V5", "2", "A3" | "1", "2", "10", "A3", "V5" | Numeric first, then alpha |

**Critical for:**
- Vendor report printing order
- UI vendor list display
- Multi-vendor report page order
- Export file consistency

**Database Considerations:**
- IndexedDB: Sorting done in application layer (JavaScript has no natural sort index)
- SQLite (future): Can use custom collation or application-layer sort
- Keep logic in Rust layer for consistency across storage backends

### 6.4 Data Flow

#### Write Operation (Browser-Only)
```
User Action → Component Event Handler → Service Call → Repository
→ IndexedDB Transaction → Storage Write → State Update → UI Re-render
```

#### Read Operation (Browser-Only)
```
Page Load → Service Call → Repository → IndexedDB Query
→ Deserialize → Return Entities → Update State → Render UI
```

#### Sync Operation (Hybrid Mode)
```
Browser                           Server
   │                                │
   ├─► Export data (JSON)           │
   │   with last_sync timestamp     │
   │                                │
   ├─────────── POST /sync ─────────►
   │                                │
   │                          Merge conflicts
   │                          Generate response
   │                                │
   │◄────── Response (changes) ─────┤
   │                                │
   ├─► Import changes               │
   │   Update local data            │
   │   Update last_sync             │
   │                                │
```

---

## User Interface Design

### 7.1 UI Framework: Leptos Component Architecture

#### Component Hierarchy

```
App
├── Router
│   ├── Layout
│   │   ├── Navigation
│   │   │   ├── Logo
│   │   │   ├── MenuItems
│   │   │   └── UserMenu
│   │   │
│   │   └── Content (Routes)
│   │       ├── HomePage
│   │       ├── BoothListPage
│   │       ├── BoothDetailPage
│   │       │   ├── BoothInfo
│   │       │   ├── VendorList
│   │       │   └── PurchaseHistory
│   │       │
│   │       ├── CheckoutPage
│   │       │   ├── VendorSelector
│   │       │   ├── ItemEntry
│   │       │   ├── Cart
│   │       │   └── CheckoutButton
│   │       │
│   │       ├── ReportsPage
│   │       │   ├── VendorReportForm
│   │       │   └── ReportPreview
│   │       │
│   │       └── SyncPage
│   │           ├── ExportButton
│   │           ├── ImportButton
│   │           └── ServerSyncPanel
│   │
│   └── Toasts (Global notifications)
```

### 7.2 Page Designs

#### Home Page
- Dashboard with quick stats
- Recent activity
- Quick actions (New Booth, Checkout, Reports)

#### Booth List Page
- Filterable/sortable table of booths
- Status badges (Open/Closed)
- Quick actions per booth

#### Booth Detail Page
- Booth information (editable)
- Vendor management (add/remove)
- Purchase history table
- Financial summary

#### Checkout Page
- Dynamic vendor input (enter vendor ID during checkout)
- Quick price entry (numeric keypad)
- Shopping cart with items (vendor ID + price per item)
- Automatic vendor creation if vendor doesn't exist
- Print receipt option

**Workflow:**
1. User enters vendor ID (attached to product)
2. User enters item price
3. System checks if vendor exists, creates if not
4. Item added to cart
5. Repeat for additional items
6. Complete purchase with optional receipt printing

#### Reports Page
- Vendor selection (multi-select)
- Report preview
- Print/export options

#### Sync Page
- Export data to file
- Import data from file
- Server sync status (if configured)
- Conflict resolution UI

### 7.3 Responsive Design

#### Breakpoints (Tailwind)
- **Mobile:** <640px (1 column)
- **Tablet:** 640px-1024px (2 columns)
- **Desktop:** >1024px (3+ columns)

#### Mobile-First Approach
- Touch-friendly controls (min 44x44px targets)
- Bottom navigation on mobile
- Sidebar navigation on desktop
- Collapsible sections for space efficiency

### 7.4 Accessibility

- **WCAG 2.1 AA Compliance**
- Semantic HTML5 elements
- ARIA labels for interactive elements
- Keyboard navigation support
- Focus indicators
- Screen reader friendly

### 7.5 Report Generation & Printing

#### Architecture Overview

Reports use **browser-native printing** (`window.print()`) with CSS media queries for professional output.

**Key Features:**
- Localizable report templates (see section 10.7)
- Natural vendor ID sorting (numeric IDs sorted numerically)
- Automatic page breaks between vendors
- Print-optimized CSS (clean layout, appropriate fonts)

#### Report Types

**1. Vendor Settlement Report**
- Single vendor or multiple vendors
- Item-by-item breakdown with timestamps
- Calculated totals (sales, fees, net revenue)
- Booth information (date, fees configuration)

**2. Booth Summary Report**
- All vendors for a booth
- Total sales by vendor (sorted by vendor ID)
- Overall booth statistics
- Fee calculations

#### Vendor Sorting for Reports

Reports **must** display vendors in natural sort order:
- Numeric IDs: `1, 2, 10, 100` (not `1, 10, 100, 2`)
- Mixed IDs: Numeric before alphanumeric: `1, 2, A1, A2`
- Alphanumeric IDs: Lexicographic: `A1, A2, A10, B1`

**Implementation:** Use `VendorId::compare_smart()` (defined in section 6.1) when sorting vendor lists for reports.

#### Print Layout

```css
@media print {
    /* Page setup */
    @page {
        size: A4 portrait;
        margin: 2cm;
    }
    
    /* Hide non-print elements */
    nav, .no-print, button {
        display: none !important;
    }
    
    /* Vendor page breaks */
    .vendor-report-section {
        page-break-after: always;
    }
    
    .vendor-report-section:last-child {
        page-break-after: auto; /* No blank page at end */
    }
    
    /* Typography */
    body {
        font-family: Arial, sans-serif;
        font-size: 12pt;
        color: #000;
    }
    
    h1 { font-size: 18pt; margin-bottom: 0.5cm; }
    h2 { font-size: 14pt; margin-top: 0.5cm; }
    
    /* Tables */
    table {
        width: 100%;
        border-collapse: collapse;
    }
    
    th, td {
        border: 1px solid #000;
        padding: 0.2cm;
        text-align: left;
    }
    
    th {
        background-color: #f0f0f0;
    }
}
```

#### Print Workflow

1. User selects vendors to include in report
2. System generates HTML report in hidden container
3. Vendors sorted using natural sort algorithm
4. Each vendor's data wrapped in `.vendor-report-section`
5. User clicks "Print" → triggers `window.print()`
6. Browser's print dialog opens with preview
7. User can adjust print settings, save as PDF, or print

**Benefits:**
- No external dependencies
- Works offline
- User controls paper size, margins, printer
- Can save as PDF natively
- Familiar interface for users

### 7.6 First-Time User Onboarding

When a user opens ez-vend-rs with no local data, provide a guided onboarding experience.

#### Welcome Screen

```rust
#[component]
pub fn WelcomeScreen() -> impl IntoView {
    let i18n = use_i18n();
    let (selected_locale, set_locale) = create_signal(detect_browser_locale());
    
    view! {
        <div class="welcome-container">
            <h1>{t!(i18n, welcome.title)}</h1>
            <p>{t!(i18n, welcome.description)}</p>
            
            // Language selection
            <LanguageSelector 
                selected=selected_locale
                on_change=set_locale
            />
            
            // Data import decision
            <div class="import-options">
                <h2>{t!(i18n, welcome.have_data_question)}</h2>
                
                <button on:click=move |_| navigate_to_import()>
                    {t!(i18n, welcome.import_from_browser)}
                </button>
                
                <button on:click=move |_| navigate_to_main()>
                    {t!(i18n, welcome.start_fresh)}
                </button>
            </div>
        </div>
    }
}
```

#### Onboarding Steps

**Step 1: Language Detection**
- Detect browser locale (navigator.language)
- Default to German if locale is `de-*`
- Default to English for all other locales
- Show language selector dropdown
- User can override detected language

**Step 2: Data Source Decision**

Present three options:
1. **Import from another browser** → File picker for .json export
2. **Import from Java ez-booth** → Future feature placeholder
3. **Start fresh** → Skip to dashboard with empty state

**Step 3: Import Flow (if chosen)**

```rust
#[component]
pub fn ImportFlow() -> impl IntoView {
    let (import_status, set_import_status) = create_signal(ImportStatus::SelectFile);
    
    view! {
        <div class="import-flow">
            {move || match import_status.get() {
                ImportStatus::SelectFile => view! {
                    <FileDropZone 
                        on_file_selected=|file| validate_and_preview(file)
                    />
                },
                ImportStatus::Preview(data) => view! {
                    <ImportPreview 
                        data=data
                        on_confirm=|strategy| import_data(strategy)
                        on_cancel=|| set_import_status(ImportStatus::SelectFile)
                    />
                },
                ImportStatus::Importing => view! {
                    <LoadingSpinner />
                },
                ImportStatus::Success => view! {
                    <SuccessMessage />
                    <button on:click=|_| navigate_to_dashboard()>
                        {t!(i18n, welcome.go_to_dashboard)}
                    </button>
                },
                ImportStatus::Error(msg) => view! {
                    <ErrorMessage message=msg />
                    <button on:click=|_| set_import_status(ImportStatus::SelectFile)>
                        {t!(i18n, common.retry)}
                    </button>
                },
            }}
        </div>
    }
}
```

**Step 4: First Transaction Guide**

After onboarding complete, show contextual tooltips for first-time actions:

```rust
// Show tooltip on first checkout
if is_first_checkout() {
    show_tooltip(
        target: "#vendor-id-input",
        message: t!(i18n, help.first_checkout_vendor),
        position: TooltipPosition::Below,
    );
}

// Dismiss after successful checkout
on_checkout_complete(|| {
    mark_tutorial_step_complete("first_checkout");
});
```

#### Recurring User Experience

**For users with existing data:**
- Skip welcome screen
- Load directly to dashboard
- Show last active booth (if any)

**Cross-browser awareness banner:**
```rust
#[component]
pub fn CrossBrowserBanner() -> impl IntoView {
    let (dismissed, set_dismissed) = use_local_storage("banner:cross_browser_dismissed");
    let session_count = get_session_count();
    
    // Show for first 3 sessions unless dismissed
    if session_count <= 3 && !dismissed {
        view! {
            <div class="info-banner">
                <Icon name="info" />
                <span>{t!(i18n, banner.data_browser_specific)}</span>
                <a href="/export">{t!(i18n, banner.export_data_link)}</a>
                <button on:click=move |_| set_dismissed(true)>
                    <Icon name="close" />
                </button>
            </div>
        }
    } else {
        view! { }
    }
}
```

#### Export Reminders

Encourage regular exports to prevent data loss:

```rust
pub fn should_show_export_reminder() -> bool {
    let transactions_since_export = get_transactions_since_last_export();
    let days_since_export = get_days_since_last_export();
    
    // Remind after 50 transactions or 7 days
    transactions_since_export >= 50 || days_since_export >= 7
}

#[component]
pub fn ExportReminder() -> impl IntoView {
    if should_show_export_reminder() {
        view! {
            <div class="reminder-banner">
                <Icon name="backup" />
                <span>{t!(i18n, reminder.export_suggested)}</span>
                <button on:click=|_| trigger_export()>
                    {t!(i18n, reminder.export_now)}
                </button>
                <button on:click=|_| dismiss_reminder()>
                    {t!(i18n, reminder.later)}
                </button>
            </div>
        }
    } else {
        view! { }
    }
}
```

**Auto-generated export filenames:**
- Format: `ezb-export-YYYY-MM-DD-HHMMSS.json`
- Example: `ezb-export-2026-03-19-143052.json`
- Saves to browser's Downloads folder
- No file dialog needed for quick exports

---

## Deployment Models

### 8.1 Static Hosting (Browser-Only)

**Deployment Steps:**
1. Build WASM with `trunk build --release`
2. Upload `dist/` folder to static host
3. Configure MIME types (`.wasm` → `application/wasm`)

**Hosting Options:**
- **GitHub Pages** - Free, CDN, custom domains
- **Netlify** - Free tier, automatic deployments
- **Vercel** - Free tier, edge network
- **Cloudflare Pages** - Free, global CDN
- **AWS S3 + CloudFront** - Scalable, custom domain

**Requirements:**
- HTTPS required for WASM and service workers
- Proper CORS headers for assets
- Gzip/Brotli compression enabled

### 8.2 Self-Hosted Server (Client-Server)

**Deployment Steps:**
1. Build WASM frontend: `trunk build --release`
2. Build Rust server: `cargo build --release --bin server`
3. Copy frontend assets to server `static/` directory
4. Configure reverse proxy (nginx/caddy)
5. Run server binary

**System Requirements:**
- Linux/Windows/macOS server
- 512MB RAM minimum
- 100MB disk space
- HTTPS certificate (Let's Encrypt)

**Docker Option:**
```dockerfile
FROM rust:1.76 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin server

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/server /usr/local/bin/
COPY --from=builder /app/frontend/dist /usr/local/share/ez-booth/static
CMD ["server"]
```

### 8.3 Progressive Web App (PWA)

**Manifest (`manifest.json`):**
```json
{
  "name": "ez-vend",
  "short_name": "ez-vend",
  "description": "Portable booth management system",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#ffffff",
  "theme_color": "#4F46E5",
  "icons": [
    {
      "src": "/icons/icon-192.png",
      "sizes": "192x192",
      "type": "image/png"
    },
    {
      "src": "/icons/icon-512.png",
      "sizes": "512x512",
      "type": "image/png"
    }
  ]
}
```

**Service Worker:**
- Cache WASM and assets for offline use
- Background sync when connection restored
- Push notifications for updates (optional)

---

## 9. Cross-Browser Data Portability

### 9.1 Challenge: Browser-Specific Storage

**Problem:** IndexedDB data is isolated per browser and per profile, creating data silos:

- ❌ Chrome data ≠ Firefox data
- ❌ Desktop Chrome ≠ Mobile Chrome  
- ❌ Chrome Profile A ≠ Chrome Profile B
- ❌ No automatic data transfer between browsers

**Impact:** Users cannot easily:
- Switch browsers without losing data
- Use multiple devices with the same data
- Recover data after browser reinstall
- Work across different browser profiles

### 9.2 Solution: Multi-Layer Portability Strategy

```
┌─────────────────────────────────────────────────────────┐
│              User's Device Ecosystem                     │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   Chrome     │  │   Firefox    │  │    Safari    │ │
│  │  IndexedDB   │  │  IndexedDB   │  │  IndexedDB   │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘ │
│         │                  │                  │          │
│         └──────────────────┼──────────────────┘          │
│                            ▼                             │
│              ┌─────────────────────────┐                │
│              │  Portability Solutions  │                │
│              │                         │                │
│              │  1. Export/Import (P0) │                │
│              │  2. File System (P1)   │                │
│              │  3. Cloud Sync (P2)    │                │
│              └─────────────────────────┘                │
└─────────────────────────────────────────────────────────┘
```

### 9.3 Layer 1: Manual Export/Import (P0 - Core Feature)

**Priority:** Must-have for MVP  
**Complexity:** Low  
**Implementation:** Phase 6 (Sync)

#### Export Functionality

```rust
// JSON export with integrity verification
pub struct ExportData {
    pub version: String,              // Schema version
    pub exported_at: DateTime<Utc>,   // Export timestamp
    pub client_id: String,            // Browser identifier
    pub booths: Vec<Booth>,
    pub vendors: Vec<Vendor>,
    pub purchases: Vec<Purchase>,
    pub checksum: String,             // SHA-256 integrity hash
}

impl ExportService {
    pub async fn export_as_json(&self) -> Result<String, Error> {
        let data = ExportData {
            version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: Utc::now(),
            client_id: self.get_client_id(),
            booths: self.fetch_all_booths().await?,
            vendors: self.fetch_all_vendors().await?,
            purchases: self.fetch_all_purchases().await?,
            checksum: String::new(), // Calculated after serialization
        };
        
        let json = serde_json::to_string_pretty(&data)?;
        Ok(json)
    }
    
    pub async fn download_export(&self) -> Result<(), JsValue> {
        let json = self.export_as_json().await?;
        let filename = format!(
            "ez-vend-export-{}.json",
            Utc::now().format("%Y%m%d-%H%M%S")
        );
        
        // Trigger browser download
        self.trigger_download(&json, &filename).await
    }
}
```

#### Import Functionality

```rust
pub enum MergeStrategy {
    Replace,  // Clear existing, import all
    Merge,    // Merge by timestamp (newer wins)
    Preview,  // Show changes without applying
}

impl ImportService {
    pub async fn import_from_json(
        &self,
        json: &str,
        strategy: MergeStrategy,
    ) -> Result<ImportResult, Error> {
        // 1. Parse and validate
        let data: ExportData = serde_json::from_str(json)?;
        self.verify_checksum(&data)?;
        self.validate_schema_version(&data.version)?;
        
        // 2. Apply strategy
        match strategy {
            MergeStrategy::Replace => self.import_replace(data).await?,
            MergeStrategy::Merge => self.import_merge(data).await?,
            MergeStrategy::Preview => return self.preview_changes(data).await,
        }
        
        Ok(ImportResult {
            booths_imported: data.booths.len(),
            vendors_imported: data.vendors.len(),
            purchases_imported: data.purchases.len(),
        })
    }
    
    async fn import_merge(&self, data: ExportData) -> Result<(), Error> {
        // Merge logic: newer timestamp wins
        for booth in data.booths {
            match self.db.get_booth(&booth.id).await? {
                Some(existing) if existing.updated_at > booth.updated_at => {
                    // Keep existing (newer)
                    continue;
                }
                _ => {
                    // Import (newer or new)
                    self.db.save_booth(&booth).await?;
                }
            }
        }
        // Repeat for vendors and purchases...
        Ok(())
    }
}
```

#### UI Integration

**Sync Page Component:**
```rust
#[component]
pub fn SyncPage() -> impl IntoView {
    view! {
        <div class="sync-page">
            <section class="export-section">
                <h2>"Export Data"</h2>
                <p>"Download all data as JSON file for:"</p>
                <ul>
                    <li>"✓ Switching to another browser"</li>
                    <li>"✓ Backing up your data"</li>
                    <li>"✓ Transferring to another device"</li>
                </ul>
                <button on:click=handle_export>
                    "📥 Download Export"
                </button>
            </section>
            
            <section class="import-section">
                <h2>"Import Data"</h2>
                <input 
                    type="file" 
                    accept=".json"
                    on:change=handle_import
                />
                <select name="strategy">
                    <option value="merge">"Merge with existing"</option>
                    <option value="replace">"Replace all data"</option>
                    <option value="preview">"Preview changes"</option>
                </select>
            </section>
        </div>
    }
}
```

**User Flow:**
1. User opens Chrome → "Export Data" → Downloads `ez-vend-export-20260319.json`
2. User opens Firefox → "Import Data" → Selects file → "Merge" → Done
3. Data now available in both browsers

### 9.4 User Education & Cross-Browser Awareness

Since IndexedDB data is browser-specific, users need clear guidance to avoid data loss when switching browsers or devices.

#### Challenge: Invisible Isolation

Users accustomed to cloud-synced applications expect data to follow them across devices and browsers. With IndexedDB:
- Data **does not** automatically sync between browsers
- Opening the app in Firefox shows empty state (even if Chrome has data)
- Users may think data is lost when it's actually in another browser

#### Education Strategy

**1. Persistent Cross-Browser Banner**

Display a dismissible banner for first 3 sessions:

```rust
#[component]
pub fn CrossBrowserBanner() -> impl IntoView {
    let i18n = use_i18n();
    let storage = use_local_storage();
    let session_count = storage.get_session_count();
    let dismissed = storage.get("banner:cross_browser_dismissed").unwrap_or(false);
    
    if session_count <= 3 && !dismissed {
        view! {
            <div class="info-banner warning">
                <Icon name="info-circle" />
                <div class="banner-content">
                    <strong>{t!(i18n, banner.important)}</strong>
                    <p>{t!(i18n, banner.data_browser_specific)}</p>
                    <p>{t!(i18n, banner.export_recommendation)}</p>
                </div>
                <a href="/export" class="banner-action">
                    {t!(i18n, banner.export_now)}
                </a>
                <button 
                    class="banner-dismiss"
                    on:click=move |_| storage.set("banner:cross_browser_dismissed", true)
                >
                    <Icon name="close" />
                </button>
            </div>
        }
    } else {
        view! { <></> }
    }
}
```

**Translation Keys:**
```json
{
  "banner": {
    "important": {
      "de": "Wichtig",
      "en": "Important"
    },
    "data_browser_specific": {
      "de": "Ihre Daten werden nur in diesem Browser gespeichert",
      "en": "Your data is stored in this browser only"
    },
    "export_recommendation": {
      "de": "Exportieren Sie regelmäßig Ihre Daten, um sie in anderen Browsern zu verwenden",
      "en": "Export your data regularly to use it in other browsers"
    },
    "export_now": {
      "de": "Jetzt exportieren",
      "en": "Export now"
    }
  }
}
```

**2. Prominent Export Button**

Make export easily accessible:
- Header navigation: "Export" button always visible
- Dashboard: "Export Data" card with last export date
- Booth list: Export section with one-click download

**3. Regular Export Reminders**

Trigger gentle reminders based on activity:

```rust
pub struct ExportReminderState {
    transactions_since_export: u32,
    days_since_export: u32,
}

impl ExportReminderState {
    pub fn should_show_reminder(&self) -> bool {
        self.transactions_since_export >= 50 || self.days_since_export >= 7
    }
}

#[component]
pub fn ExportReminder() -> impl IntoView {
    let state = use_export_reminder_state();
    
    if state.should_show_reminder() {
        view! {
            <div class="reminder-card">
                <Icon name="backup" size=24 />
                <div class="reminder-content">
                    <h3>{t!(i18n, reminder.backup_suggested)}</h3>
                    <p>{t!(i18n, reminder.backup_reason, 
                        transactions: state.transactions_since_export
                    )}</p>
                </div>
                <button class="primary" on:click=|_| trigger_export()>
                    {t!(i18n, reminder.export_now)}
                </button>
                <button class="secondary" on:click=|_| snooze_reminder()>
                    {t!(i18n, reminder.remind_later)}
                </button>
            </div>
        }
    } else {
        view! { <></> }
    }
}
```

**4. First-Time Import Prompt**

When opening app with empty storage, ask if user has existing data:

```rust
#[component]
pub fn EmptyStatePrompt() -> impl IntoView {
    let storage = use_storage();
    let is_first_visit = storage.is_empty();
    
    if is_first_visit {
        view! {
            <div class="empty-state-prompt">
                <h2>{t!(i18n, welcome.first_time_question)}</h2>
                <div class="option-cards">
                    <Card on:click=navigate_to_import>
                        <Icon name="download" size=48 />
                        <h3>{t!(i18n, welcome.import_existing)}</h3>
                        <p>{t!(i18n, welcome.import_description)}</p>
                    </Card>
                    <Card on:click=navigate_to_new_booth>
                        <Icon name="plus" size=48 />
                        <h3>{t!(i18n, welcome.start_fresh)}</h3>
                        <p>{t!(i18n, welcome.fresh_description)}</p>
                    </Card>
                </div>
            </div>
        }
    } else {
        view! { <></> }
    }
}
```

**5. Export Filename Auto-Generation**

Make export effortless:
- Format: `ezb-export-YYYY-MM-DD-HHMMSS.json`
- Example: `ezb-export-2026-03-19-143052.json`
- Automatic download to browser's Downloads folder
- No save dialog needed for quick exports

**6. In-App Documentation**

Provide accessible help:
- FAQ section: "How do I use my data on another device?"
- Tutorial video: "Exporting and importing your data"
- Booth list warning area: Clear explanation of browser-based storage

#### Expected User Behavior

**Ideal workflow for multi-browser users:**
1. Use ez-booth in primary browser (e.g., Chrome)
2. See banner about browser-specific storage
3. Export data to Downloads folder
4. Switch to secondary browser (e.g., Firefox)
5. Import previously exported file
6. Continue working with synced data
7. Repeat export when needed (after significant changes)

**Safety nets:**
- Banner reminds users regularly
- Export reminder triggers based on activity
- One-click export minimizes friction
- Auto-generated filenames prevent overwrites

### 9.5 Layer 2: File System Access API (P1 - Enhanced)

**Priority:** Nice-to-have for better UX  
**Complexity:** Medium  
**Implementation:** Phase 7 (Polish)

#### Automatic Cloud Folder Sync

**Feature:** Save export directly to synced folder (Dropbox, Google Drive, iCloud)

```rust
#[cfg(feature = "file-system-access")]
impl ExportService {
    pub async fn export_to_file_system(&self) -> Result<(), JsValue> {
        // Use native File System Access API
        let file_handle = self.show_save_file_picker(
            "ez-vend-data.json",
            "application/json"
        ).await?;
        
        let writable = file_handle.create_writable().await?;
        let json = self.export_as_json().await?;
        
        writable.write(&JsValue::from_str(&json)).await?;
        writable.close().await?;
        
        Ok(())
    }
}
```

**Benefits:**
- User selects save location (e.g., `~/Dropbox/ez-vend-data.json`)
- OS/cloud provider handles sync automatically
- Other devices can import from same synced file
- No manual download/upload needed

**Browser Support:**
- Chrome 86+ ✅
- Edge 86+ ✅
- Firefox: In development ⚠️
- Safari: Not supported ❌

**Fallback:** Use standard download for unsupported browsers

### 9.6 Layer 3: Optional Cloud Sync (P2 - Advanced)

**Priority:** Future enhancement  
**Complexity:** High  
**Implementation:** Phase 6+ (Optional feature)

#### Cloud Sync Architecture

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  Browser A   │         │  Cloud Sync  │         │  Browser B   │
│  (Chrome)    │◄───────►│   Service    │◄───────►│  (Firefox)   │
│  IndexedDB   │  HTTPS  │              │  HTTPS  │  IndexedDB   │
└──────────────┘         └──────────────┘         └──────────────┘
                                │
                                ▼
                         ┌──────────────┐
                         │   Storage    │
                         │ (PostgreSQL  │
                         │  or Supabase)│
                         └──────────────┘
```

#### Cloud Sync Service

```rust
pub struct CloudSyncService {
    backend_url: String,
    user_token: Option<String>,
    auto_sync_enabled: bool,
}

impl CloudSyncService {
    pub async fn sync_to_cloud(&self) -> Result<(), Error> {
        let export = ExportService::new().export_all_data().await?;
        
        // Upload to backend
        let response = reqwest::Client::new()
            .post(&format!("{}/api/sync", self.backend_url))
            .bearer_auth(self.user_token.as_ref().unwrap())
            .json(&export)
            .send()
            .await?;
        
        if response.status().is_success() {
            self.update_last_sync_timestamp().await?;
            Ok(())
        } else {
            Err(Error::SyncFailed)
        }
    }
    
    pub fn enable_auto_sync(&mut self, interval_minutes: u32) {
        self.auto_sync_enabled = true;
        
        // Background sync every N minutes
        spawn_local(async move {
            loop {
                sleep(Duration::from_secs(interval_minutes as u64 * 60)).await;
                let _ = self.sync_to_cloud().await;
            }
        });
    }
}
```

### 9.7 Data Format Specification

#### Export JSON Schema

```json
{
  "version": "0.1.0",
  "exported_at": "2026-03-19T14:31:00Z",
  "client_id": "chrome-desktop-abc123",
  "booths": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "description": "Spring Fair 2026",
      "date": "2026-03-25",
      "fees": {
        "participation_fee": "10.00",
        "sales_fee_percent": "5.0",
        "rounding_step": "0.50"
      },
      "status": { "type": "Open" },
      "created_at": "2026-03-19T10:00:00Z",
      "updated_at": "2026-03-19T14:00:00Z"
    }
  ],
  "vendors": [...],
  "purchases": [...],
  "checksum": "a3f5b8c9d2e1f4a7b6c5d8e9f2a1b4c7"
}
```

**Schema Features:**
- **Versioned:** `version` field for backward compatibility
- **Timestamped:** `exported_at` for audit trail  
- **Checksummed:** Integrity verification
- **Human-readable:** JSON for easy debugging

### 9.8 Implementation Priorities

| Priority | Feature | Phase | Effort |
|----------|---------|-------|--------|
| **P0** | Manual Export/Import | Phase 6 | 3 days |
| **P0** | JSON format with checksum | Phase 6 | 1 day |
| **P0** | Merge strategies | Phase 6 | 2 days |
| **P1** | File System Access API | Phase 7 | 2 days |
| **P2** | Cloud sync service | Phase 8+ | 2 weeks |

### 9.9 Success Criteria

**Phase 6 (MVP):**
- ✅ User can export all data as JSON
- ✅ User can import JSON in another browser
- ✅ Merge strategy preserves newer data
- ✅ Checksum verification prevents corruption
- ✅ <5 seconds for typical export/import

**Phase 7 (Enhanced):**
- ✅ File System Access API works in Chrome/Edge
- ✅ User can save to synced folder
- ✅ Graceful fallback for unsupported browsers
- ✅ Welcome screen detects empty state and prompts for import

### 9.10 User Onboarding & Browser Switch Detection

#### Problem: Silent Data Loss

When users open ez-booth in a new browser, they see an empty application with no indication that:
- They might have data in another browser
- They need to export/import to transfer data
- The application is working correctly (empty state vs. error)

#### Solution: Smart Welcome Screen

**Detection Logic:**
```rust
pub struct OnboardingState {
    pub is_first_visit: bool,
    pub has_data: bool,
    pub browser_info: BrowserInfo,
}

impl OnboardingState {
    pub async fn detect() -> Self {
        let has_data = Self::check_database_populated().await;
        let is_first_visit = Self::check_first_visit_flag().await;
        
        Self {
            is_first_visit,
            has_data,
            browser_info: Self::get_browser_info(),
        }
    }
    
    async fn check_database_populated() -> bool {
        // Check if any booths exist
        let booth_count = Database::new().count_booths().await.unwrap_or(0);
        booth_count > 0
    }
    
    async fn check_first_visit_flag() -> bool {
        // Check localStorage for first visit flag
        let window = web_sys::window().unwrap();
        let storage = window.local_storage().unwrap().unwrap();
        
        storage.get_item("ez_vend_visited").unwrap().is_none()
    }
    
    fn get_browser_info() -> BrowserInfo {
        let window = web_sys::window().unwrap();
        let navigator = window.navigator();
        
        BrowserInfo {
            user_agent: navigator.user_agent().unwrap_or_default(),
            browser_name: Self::detect_browser_name(&navigator),
        }
    }
    
    fn detect_browser_name(navigator: &web_sys::Navigator) -> String {
        let ua = navigator.user_agent().unwrap_or_default();
        
        if ua.contains("Firefox") {
            "Firefox".to_string()
        } else if ua.contains("Edg") {
            "Edge".to_string()
        } else if ua.contains("Chrome") {
            "Chrome".to_string()
        } else if ua.contains("Safari") {
            "Safari".to_string()
        } else {
            "Unknown".to_string()
        }
    }
}
```

**Welcome Screen Component:**
```rust
#[component]
pub fn WelcomeScreen() -> impl IntoView {
    let onboarding = create_resource(|| (), |_| OnboardingState::detect());
    
    view! {
        <Suspense fallback=|| view! { <div>"Loading..."</div> }>
            {move || onboarding.get().map(|state| {
                if state.is_first_visit && !state.has_data {
                    view! { <FirstTimeWelcome browser=state.browser_info.clone() /> }
                } else if state.has_data {
                    view! { <Navigate to="/booths" /> }
                } else {
                    view! { <EmptyState /> }
                }
            })}
        </Suspense>
    }
}

#[component]
pub fn FirstTimeWelcome(browser: BrowserInfo) -> impl IntoView {
    view! {
        <div class="welcome-screen max-w-2xl mx-auto p-8 text-center">
            <h1 class="text-4xl font-bold mb-4">"Welcome to ez-booth!"</h1>
            <p class="text-xl mb-8">
                "You're using " {browser.browser_name.clone()} " for the first time"
            </p>
            
            <div class="mb-8 p-6 bg-blue-50 border-2 border-blue-200 rounded-lg">
                <h2 class="text-2xl font-semibold mb-4">"🔍 Have data in another browser?"</h2>
                <p class="mb-4">
                    "If you've used ez-booth in Chrome, Firefox, or another browser before, 
                     you'll need to transfer your data:"
                </p>
                <ol class="text-left list-decimal list-inside space-y-2 mb-4">
                    <li>"Open ez-booth in your other browser"</li>
                    <li>"Open the booth list and choose Export Data"</li>
                    <li>"Download the JSON file"</li>
                    <li>"Come back here and import it"</li>
                </ol>
                <button 
                    class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                    on:click=|_| {
                        // Navigate to import page
                        use leptos_router::*;
                        let navigate = use_navigate();
                        navigate("/sync", Default::default());
                    }
                >
                    "📥 Import Existing Data"
                </button>
            </div>
            
            <div class="p-6 bg-green-50 border-2 border-green-200 rounded-lg">
                <h2 class="text-2xl font-semibold mb-4">"🆕 First time using ez-booth?"</h2>
                <p class="mb-4">"Start fresh and create your first booth"</p>
                <button 
                    class="px-6 py-3 bg-green-600 text-white rounded-lg hover:bg-green-700"
                    on:click=move |_| {
                        // Mark as visited and create first booth
                        let window = web_sys::window().unwrap();
                        let storage = window.local_storage().unwrap().unwrap();
                        let _ = storage.set_item("ez_vend_visited", "true");
                        
                        let navigate = use_navigate();
                        navigate("/booths/new", Default::default());
                    }
                >
                    "🚀 Create First Booth"
                </button>
            </div>
            
            <div class="mt-8 text-sm text-gray-600">
                <a href="/help/getting-started" class="underline">"Learn more about getting started"</a>
            </div>
        </div>
    }
}
```

#### Additional Signals & Prompts

**1. Browser Tab Title Signal**
```rust
// Change tab title when empty
if !has_data {
    document.set_title("ez-booth (Empty - Import Data?)");
} else {
    document.set_title("ez-vend");
}
```

**2. Navigation Bar Hint**
```rust
#[component]
pub fn NavBar() -> impl IntoView {
    let booth_count = create_resource(|| (), |_| async {
        Database::new().count_booths().await.unwrap_or(0)
    });
    
    view! {
        <nav class="navbar">
            {move || booth_count.get().map(|count| {
                if count == 0 {
                    view! {
                        <div class="import-hint bg-yellow-100 border-yellow-400 p-2 text-sm">
                            "💡 No data yet. "
                            <a href="/sync" class="underline">"Import from another browser?"</a>
                        </div>
                    }
                } else {
                    view! { <div></div> }
                }
            })}
            // ... rest of nav
        </nav>
    }
}
```

**3. Empty State with Import CTA**
```rust
#[component]
pub fn EmptyBoothList() -> impl IntoView {
    view! {
        <div class="empty-state text-center p-12">
            <svg class="w-24 h-24 mx-auto mb-4 text-gray-300">"..."</svg>
            <h2 class="text-2xl font-semibold mb-2">"No booths yet"</h2>
            <p class="text-gray-600 mb-6">
                "Get started by creating a new booth or importing existing data"
            </p>
            
            <div class="flex gap-4 justify-center">
                <a 
                    href="/booths/new"
                    class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                    "➕ Create New Booth"
                </a>
                
                <a 
                    href="/sync"
                    class="px-6 py-3 bg-white border-2 border-gray-300 rounded-lg hover:bg-gray-50"
                >
                    "📥 Import Data"
                </a>
            </div>
            
            <div class="mt-8 p-4 bg-blue-50 rounded-lg inline-block">
                <p class="text-sm text-blue-800">
                    "💡 <strong>Switching browsers?</strong> "
                    "Export your data from the other browser, then import it here. "
                    <a href="/help/switching-browsers" class="underline">"Learn how →"</a>
                </p>
            </div>
        </div>
    }
}
```

**4. Persistent Helper in Booth List**
```rust
#[component]
pub fn BoothListPage() -> impl IntoView {
    view! {
        <div class="booth-list-page">
            <h1>"Booths"</h1>
            
            // Always show import/export prominently
            <section class="data-portability bg-gradient-to-r from-blue-50 to-purple-50 p-6 rounded-lg mb-8">
                <h2 class="text-xl font-semibold mb-2">"📦 Data Portability"</h2>
                <p class="mb-4">"Switch browsers or devices easily"</p>
                <div class="flex gap-4">
                    <a href="/sync" class="px-4 py-2 bg-blue-600 text-white rounded">
                        "Export / Import"
                    </a>
                    <a href="/help/switching-browsers" class="px-4 py-2 border rounded">
                        "Learn More"
                    </a>
                </div>
            </section>
            
            // ... booth list content
        </div>
    }
}
```

**5. Browser Detection in Footer**
```rust
#[component]
pub fn Footer() -> impl IntoView {
    let browser = BrowserInfo::detect();
    
    view! {
        <footer class="text-center text-sm text-gray-500 p-4">
            <p>"Running in " {browser.browser_name} " | "
            <a href="/sync" class="underline">"Switch browsers?"</a>
            "</p>
        </footer>
    }
}
```

#### Help Documentation

**New Help Page:** `/help/switching-browsers`

```markdown
# Switching Browsers

## Why do I need to export/import?

ez-booth stores all data in your browser's local storage. Each browser 
(Chrome, Firefox, Safari, etc.) has its own separate storage that other 
browsers cannot access.

## Step-by-Step Guide

### From Your Old Browser:
1. Open ez-booth
2. Open the booth list and choose Export Data
3. Click "Download Export"
4. Save the `ez-vend-export-YYYYMMDD.json` file

### In Your New Browser:
1. Open ez-booth
2. Click "Import Data" on the welcome screen (or use the booth list import action)
3. Select the JSON file you downloaded
4. Choose "Merge with existing" (recommended)
5. Click Import

### Pro Tip: Use Cloud Sync
Save your export to Dropbox or Google Drive for automatic sync across 
devices!

## Troubleshooting

**Q: I don't see the welcome screen**
A: Go directly to the booth list export/import actions

**Q: Import failed with "Invalid checksum"**
A: The file may be corrupted. Try exporting again.

**Q: Some data is missing after import**
A: Check that you selected the correct JSON file and used "Merge" strategy.
```

#### Implementation Priority

| Feature | Priority | Phase | Effort |
|---------|----------|-------|--------|
| Empty state detection | P0 | Phase 6 | 1 hour |
| Welcome screen | P0 | Phase 6 | 4 hours |
| Browser detection | P1 | Phase 6 | 2 hours |
| Help documentation | P0 | Phase 6 | 2 hours |
| Navigation hints | P1 | Phase 7 | 2 hours |
| Footer browser info | P2 | Phase 7 | 1 hour |

#### Success Metrics

**User Awareness:**
- 90%+ of new browser users see welcome screen
- 80%+ understand they need to import data
- <5% support tickets about "lost data"

**Conversion Rate:**
- 70%+ of users with empty state click "Import Data"
- 60%+ successfully complete import process
- 40%+ set up cloud folder sync (Phase 7)

---

## 10. Internationalization (i18n)

### 10.1 Overview

**Primary Language:** German (de)  
**Fallback Language:** English (en)  
**Future:** Extensible to additional languages

The Java version has full German localization (172 translation keys). This must be preserved in the Rust version.

### 10.2 Technology

**Library:** `leptos_i18n 0.3+`

**Features:**
- Compile-time key validation
- Type-safe translations
- Browser locale detection
- Reactive language switching
- Small footprint (~20KB)

### 10.3 File Structure

```
locales/
├── de.json          # German (primary, 172 keys)
├── en.json          # English (fallback)
└── translations.json # Config
```

### 10.4 Implementation

```rust
use leptos_i18n::*;

// Browser locale detection
pub fn init_i18n() -> Locale {
    let browser_lang = window()
        .navigator()
        .language()
        .unwrap_or_default();
    
    match browser_lang.split('-').next() {
        Some("de") => Locale::De,
        Some("en") => Locale::En,
        _ => Locale::De, // Default to German
    }
}

// Usage in components
#[component]
pub fn BoothForm() -> impl IntoView {
    let i18n = use_i18n();
    
    view! {
        <h2>{t!(i18n, booth.title)}</h2>
        <label>{t!(i18n, booth.description.label)}</label>
        <button>{t!(i18n, common.save)}</button>
    }
}
```

### 10.5 Format Helpers

Locale-aware formatting for:
- **Currency:** EUR vs USD formatting
- **Dates:** dd.MM.yyyy (de) vs MM/dd/yyyy (en)
- **Decimals:** Comma vs period separators

### 10.6 Translation Categories

| Category | Keys | Examples |
|----------|------|----------|
| App Layout | 3 | Title, tooltips |
| Booth Management | 35 | Forms, validation, status |
| Checkout | 25 | Keypad, confirmation, receipts |
| Vendor Reports | 20 | Lists, statistics, printing |
| Report Templates | 15 | Print headers, totals, labels |
| Export/Import | 30 | File operations, notifications |
| Generic | 14 | Buttons, errors, common text |

**Total:** ~142 translation keys (expandable to match Java's 172)

### 10.7 Report Template Localization

#### Current Problem
Java version uses hardcoded German strings in Thymeleaf templates:
- `VendorReport.template.html`: "Verkäufer-Quittung", "Gesamtsumme", "Zeitraum"
- Reports cannot be generated in other languages
- No locale-aware date/number formatting in templates

#### Proposed Solution
**Template Rendering with i18n:**
```rust
pub fn render_vendor_report(
    vendor: &Vendor,
    items: &[PurchaseItem],
    locale: Locale,
) -> String {
    let i18n = get_translations(locale);
    
    format!(r#"
        <html>
        <head>
            <title>{}</title>
            <style>/* Print-friendly CSS */</style>
        </head>
        <body>
            <h1>{}</h1>
            <table>
                <tr>
                    <th>{}</th>
                    <th>{}</th>
                    <th>{}</th>
                </tr>
                {items_html}
            </table>
            <div class="total">
                {}: {total}
            </div>
        </body>
        </html>
    "#,
        i18n.report.vendor_receipt,
        i18n.report.vendor_receipt,
        i18n.report.date,
        i18n.report.item,
        i18n.report.amount,
        items_html = render_items(items, locale),
        i18n.report.total,
        total = format_currency(calculate_total(items), locale)
    )
}
```

**Key Translation Keys for Reports:**
```json
{
  "report": {
    "vendor_receipt": {
      "de": "Verkäufer-Quittung",
      "en": "Vendor Receipt"
    },
    "total": {
      "de": "Gesamtsumme",
      "en": "Total"
    },
    "period": {
      "de": "Zeitraum",
      "en": "Period"
    },
    "date": {
      "de": "Datum",
      "en": "Date"
    },
    "item": {
      "de": "Artikel",
      "en": "Item"
    },
    "amount": {
      "de": "Betrag",
      "en": "Amount"
    },
    "quantity": {
      "de": "Anzahl",
      "en": "Quantity"
    }
  }
}
```

**Locale-Aware Formatting:**
- **Currency:** `12,50 €` (DE) vs `€12.50` (EN)
- **Date:** `19.03.2026` (DE) vs `03/19/2026` (EN)
- **Numbers:** `1.234,56` (DE) vs `1,234.56` (EN)

**Print CSS remains language-agnostic** - page breaks, margins, fonts work for all languages.

#### Multi-Vendor Report Pagination

**Requirement:** When printing reports for multiple vendors, each vendor's report should start on a new page for easy separation and distribution.

**Implementation Strategy:**

```css
/* Print-specific styles for vendor reports */
@media print {
    .vendor-report-page {
        page-break-after: always; /* Force new page after each vendor */
    }
    
    .vendor-report-page:last-child {
        page-break-after: auto; /* Don't add blank page at end */
    }
    
    @page {
        margin: 2cm;
        size: A4 portrait;
    }
}
```

**Template Structure for Bulk Printing:**
```rust
pub fn render_multi_vendor_report(
    vendors: &[Vendor],
    items_by_vendor: &HashMap<VendorId, Vec<PurchaseItem>>,
    locale: Locale,
) -> String {
    let vendor_pages: Vec<String> = vendors
        .iter()
        .map(|vendor| {
            let items = items_by_vendor.get(&vendor.id).unwrap_or(&vec![]);
            format!(
                r#"<div class="vendor-report-page">
                    {}
                </div>"#,
                render_single_vendor_report(vendor, items, locale)
            )
        })
        .collect();
    
    format!(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <meta charset="utf-8">
            <title>{}</title>
            <style>
                /* Base report styles */
                body {{ margin: 0; padding: 0; font-family: Arial, sans-serif; }}
                
                @media print {{
                    .vendor-report-page {{
                        page-break-after: always;
                        padding: 20px;
                    }}
                    .vendor-report-page:last-child {{
                        page-break-after: auto;
                    }}
                    @page {{
                        margin: 2cm;
                        size: A4 portrait;
                    }}
                }}
                
                @media screen {{
                    .vendor-report-page {{
                        margin: 20px auto;
                        max-width: 21cm;
                        padding: 20px;
                        border: 1px solid #ccc;
                        box-shadow: 0 2px 4px rgba(0,0,0,0.1);
                    }}
                }}
                
                table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
                th, td {{ padding: 8px; text-align: left; border-bottom: 1px solid #ddd; }}
                th {{ background-color: #f5f5f5; font-weight: bold; }}
                .total {{ margin-top: 20px; font-weight: bold; text-align: right; font-size: 1.2em; }}
                h1 {{ margin-top: 0; }}
            </style>
        </head>
        <body>
            {}
        </body>
        </html>"#,
        get_translations(locale).report.all_vendor_receipts,
        vendor_pages.join("\n")
    )
}
```

**User Interface Features:**

1. **Preview Mode:** On-screen display shows each vendor as a separate "page" with visual separation
2. **Print All Button (Default):** Primary action - single click prints all vendors with automatic page breaks
3. **Individual Print (Optional):** Secondary option to print single vendor from dropdown
4. **Batch Selection (Optional):** Advanced feature - checkboxes to select specific vendors for batch printing

**Translation Keys:**
```json
{
  "report": {
    "all_vendor_receipts": {
      "de": "Alle Verkäufer-Quittungen",
      "en": "All Vendor Receipts"
    },
    "print_all_vendors": {
      "de": "Alle Verkäufer drucken",
      "en": "Print All Vendors"
    },
    "print_selected": {
      "de": "Ausgewählte drucken",
      "en": "Print Selected"
    }
  }
}
```

**Benefits:**
- Clean separation for distributing printed reports to individual vendors
- Preview shows exactly what will print
- Efficient bulk printing (single print job for all vendors)
- Works consistently across all browsers and operating systems

### 10.8 Implementation Timeline

**Phase 1 (Week 1):** Setup i18n infrastructure + report templates (10 hours)  
**Phase 2 (Ongoing):** Replace hardcoded strings (4 hours)  
**Phase 4 (Week 1):** Testing and validation (2 hours)

**Total Additional Effort:** 16 hours (~2 days)

### 10.9 Success Metrics

| Metric | Target |
|--------|--------|
| Translation coverage | 100% (UI + reports) |
| Browser locale detection | 95%+ accuracy |
| Language switch latency | <50ms |
| Bundle size impact | <20KB |
| Report language accuracy | 100% |

**For detailed implementation, see:** `04_IMPLEMENTATION.md` and the current i18n implementation in `crates/ez-vend-ui`.

---

## 11. Data Migration from ez-booth

### 11.1 Overview

ez-vend-rs includes integrated migration functionality to preserve user data when transitioning from the legacy ez-booth (Java/Spring Boot) application. This allows users to import their historical booth data, vendors, purchases, and reports without data loss.

### 11.2 Migration Approach

**Strategy:** Browser-based SQLite import

The migration is implemented as a WASM component within the main application that:
1. Accepts user upload of ez-booth's SQLite database file (`booth.db`)
2. Parses the SQLite database using `sql.js` (SQLite compiled to WASM)
3. Transforms data to match ez-vend-rs domain models
4. Imports directly into IndexedDB storage

**Benefits:**
- **Integrated Experience** - No separate tools required
- **Privacy-Preserving** - All processing happens locally in browser
- **User-Friendly** - Single-step upload and import process
- **Seamless Transition** - Original ez-booth database remains intact

### 11.3 Data Mapping

The migration transforms ez-booth's SQLite schema to ez-vend-rs's JSON format:

| ez-booth Entity | ez-vend-rs Entity | Key Transformations |
|-----------------|-------------------|---------------------|
| `booth` table | `Booth` | `description` → `name`, `closed` → `status` enum |
| `vendor` table | `Vendor` | Direct mapping with synthetic `created_at` |
| `purchase` + `purchase_item` | `Transaction` | Denormalize items into transaction |
| Numeric vendor IDs | Smart sorting | Ensure natural sort (1, 2, 10 not 1, 10, 2) |

### 11.4 User Interface

**Migration Wizard:**
1. **Welcome Screen** - "Import from ez-vend" option (prominently displayed if no data exists)
2. **Instructions** - Show the default database location for the current platform and offer a copy-to-clipboard action
3. **Upload** - File picker for `booth.db` (manual selection only; browsers cannot preselect the folder)
4. **Processing** - Progress bar with validation and transformation steps
5. **Preview** - Display booth/vendor/transaction counts before final import
6. **Confirmation** - Complete import or cancel with detailed error reporting

**Discoverability:**
- If no booth data exists yet, the empty booth list should show a secondary migration call-to-action.
- That call-to-action should deep-link to `Settings` with the migration tab preselected via `?tab=migration`.

**Error Handling:**
- Schema version detection and validation
- Referential integrity checks
- Partial import support (skip corrupt records)
- Detailed error export for support troubleshooting

### 11.5 Technical Implementation

**Module:** `crates/ez-vend-migration/`

**Dependencies:**
- `sql.js` (via JS interop) - SQLite parsing
- `web-sys::File` - Browser file handling
- `serde_json` - Data transformation
- Core domain models for validation

**Key Functions:**
```rust
pub async fn migrate_from_sqlite(
    file: web_sys::File
) -> Result<MigrationResult, MigrationError>;

pub struct MigrationResult {
    pub booths_imported: usize,
    pub vendors_imported: usize,
    pub transactions_imported: usize,
    pub warnings: Vec<MigrationWarning>,
}
```

### 11.6 Testing Strategy

- Unit tests with sample SQLite databases
- Edge cases: empty DB, large DB (>100MB), corrupt data
- Integration tests verifying data integrity post-migration
- Manual testing with real production databases

### 11.7 Migration Timeline

**Not required for MVP** - Migration is a convenience feature for existing ez-booth users. New users start fresh.

**Recommended Phase:** Phase 3 (Post-MVP enhancement)
- Allows focus on core functionality first
- Can gather user feedback on must-have migration features
- Reduces initial complexity

**For current migration status, see:** `05_STATUS.md` and `../COMPARISON_TO_ORIGINAL.md`.

---

## 12. Security & Privacy

### 12.1 Data Protection

#### Browser-Only Mode
- **Encryption at Rest:** Not required (local-only data)
- **Privacy:** All data stays in browser, never transmitted
- **Clear Data:** User can clear browser storage to delete all data

#### Client-Server Mode
- **HTTPS Only:** Enforce TLS for all communication
- **Authentication:** Optional JWT-based auth
- **Authorization:** Role-based access control (RBAC)
- **Data Encryption:** Encrypt sensitive fields in database

### 10.2 Input Validation

- **Frontend:** Immediate validation feedback
- **Backend:** Re-validate all inputs (never trust client)
- **Type Safety:** Rust types prevent injection attacks
- **Sanitization:** Escape user input in reports/exports

### 10.3 CSRF Protection

- **SameSite Cookies:** Use `SameSite=Strict` for session cookies
- **CSRF Tokens:** For state-changing operations
- **Origin Validation:** Check `Origin` header on server

---

## 13. Error Handling & User Support

### 12.1 Overview

Comprehensive error handling and support infrastructure to minimize user frustration and support burden.

**Key Features:**
1. User-friendly error messages (localized)
2. Automatic error recovery mechanisms
3. Built-in diagnostic tools
4. Self-service help system
5. One-click support bundle export

### 12.2 Error Hierarchy

```rust
pub enum AppError {
    // User-recoverable (show friendly message + action)
    Validation(ValidationError),
    NotFound(EntityType, String),
    Conflict(ConflictError),
    
    // System errors (show recovery options)
    Storage(StorageError),
    Network(NetworkError),
    Sync(SyncError),
    
    // Fatal errors (show diagnostic export)
    Corruption(CorruptionError),
    BrowserCompatibility(CompatError),
    OutOfMemory,
}
```

### 12.3 Error Display Strategy

**User-Recoverable:**
```
⚠️ Vendor name required
Please enter a name for this vendor.
[Go back] [Learn more]
```

**System Errors:**
```
❌ Failed to save changes
Could not write to local storage.

Try these steps:
• Refresh the page and try again
• Check browser storage settings
• Export your data as backup

[Retry] [Export Backup] [Get Help]
```

**Fatal Errors:**
```
🔴 Critical Error
The application encountered a problem it cannot 
recover from automatically.

Your data is safe. Download a support bundle to 
share with our team.

[Download Support Bundle] [Contact Support] [Reload App]
```

### 12.4 Diagnostic Tools

**System Health Check** (dedicated diagnostics view):
```
System Health
─────────────────────────────────────────
✅ Browser: Chrome 120.0.6099 (supported)
✅ Storage: 45.2 MB / 500 MB available
✅ WASM: Loaded successfully
✅ Database: 3 tables, 156 records
⚠️  IndexedDB version: 2 (upgrade available)
✅ Last backup: 2026-03-18 14:32

[Run Full Diagnostic] [Export Diagnostic Report]
```

**Diagnostic Report Export:**
- System information (browser, platform, versions)
- Error logs (last 7 days)
- Performance metrics
- Storage statistics
- Feature flags
- **NO sensitive data** (no vendor names, purchase data)

### 12.5 Self-Service Support

**In-App Help System:**
- Searchable help articles
- Context-sensitive tooltips
- Guided tours for new users
- FAQ database
- Troubleshooting guides

**Categories:**
- Getting Started
- Data Management (export/import/backup)
- Reports & Printing
- Settings & Configuration
- Troubleshooting

### 12.6 Error Recovery

**Automatic Recovery:**
- Transaction rollback on failures
- Optimistic retry with exponential backoff
- Graceful degradation for missing features

**Manual Recovery:**
- Data repair wizard (orphaned records, invalid data)
- Database rebuild from backup
- Conflict resolution UI

### 12.7 Support Bundle

**One-Click Export:**
User clicks "Download Support Bundle" → generates ZIP with:
- `diagnostic-report.json` - System health
- `error-log.json` - Recent errors
- `performance-metrics.json` - Performance data
- `system-info.json` - Browser & platform
- `README.txt` - Instructions for support

**Privacy Guarantee:** Bundle contains NO sensitive data (verified by automated tests).

### 12.8 Success Metrics

| Metric | Current (Java) | Target (Rust) | Improvement |
|--------|----------------|---------------|-------------|
| Support tickets (errors) | 10/month | 2/month | **80% reduction** |
| Time to diagnose issue | 30 min | 5 min | **6x faster** |
| Self-service resolution | 20% | 70% | **3.5x higher** |
| User error understanding | 30% | 90% | **3x better** |

### 12.9 Implementation Priority

| Feature | Priority | Phase | Effort |
|---------|----------|-------|--------|
| User-friendly error messages | P0 | 2 | 8h |
| Error recovery (retry) | P0 | 2 | 4h |
| In-app help system | P1 | 4 | 12h |
| Diagnostic report export | P1 | 4 | 6h |
| Support bundle generation | P1 | 4 | 4h |
| Error log viewer | P2 | 5 | 4h |
| Data repair wizard | P2 | 6 | 8h |
| Guided tours | P2 | 6 | 8h |

**Total Effort:** ~54 hours (1.5 weeks)

**For the current validation and operator-support direction, see:** `05_STATUS.md`, `../validation/VALIDATION_WORKFLOW.md`, and `../planning/PHASE2_PROPOSAL.md`.

---

## 14. Performance Targets

### 13.1 Bundle Size

| Asset | Target | Rationale |
|-------|--------|-----------|
| WASM | <3MB | Gzipped to ~800KB over network |
| JavaScript | <500KB | Wasm-bindgen glue + Leptos runtime |
| CSS | <50KB | Tailwind purged of unused classes |
| **Total** | **<3.5MB** | Initial load, cached thereafter |

### 13.2 Runtime Performance

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to Interactive | <500ms | Lighthouse audit |
| First Contentful Paint | <300ms | Lighthouse audit |
| Largest Contentful Paint | <1s | Lighthouse audit |
| Memory Usage | <50MB | Browser DevTools |
| Checkout Transaction | <100ms | Lighthouse audit |

### 13.3 Storage Limits

| Storage | Limit | Notes |
|---------|-------|-------|
| IndexedDB | ~50% of disk space | Browser-dependent quota |
| LocalStorage | ~10MB | Settings only |
| Cache API | ~50% of disk space | Service worker cache |

**Estimated Capacity:**
- ~10,000 purchases per booth (10MB typical)
- ~100 booths per installation
- ~1 million purchase items total

---

## Appendices

### A. Technology Comparison Matrix

| Feature | Leptos | Yew | Dioxus | Svelte* |
|---------|--------|-----|--------|---------|
| Bundle Size | ~150KB | ~200KB | ~250KB | ~50KB |
| Reactivity | Signals | Virtual DOM | Virtual DOM | Compiler |
| Performance | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| Maturity | Medium | High | Medium | High |
| Rust Native | ✅ | ✅ | ✅ | ❌ |
| SSR Support | ✅ | ❌ | ✅ | ✅ |

*Svelte requires JavaScript toolchain

**Decision:** Leptos for optimal balance of size, performance, and DX

### B. Browser Compatibility Matrix

| Browser | Minimum Version | WASM | IndexedDB | Service Worker |
|---------|----------------|------|-----------|----------------|
| Chrome | 90+ (2021) | ✅ | ✅ | ✅ |
| Firefox | 88+ (2021) | ✅ | ✅ | ✅ |
| Safari | 14+ (2020) | ✅ | ✅ | ✅ |
| Edge | 90+ (2021) | ✅ | ✅ | ✅ |

**Coverage:** ~95% of users (2024+ data)

### C. Development Timeline Estimate

| Phase | Duration | Deliverables |
|-------|----------|--------------|
| Phase 1: Core MVP | 4 weeks | Entities, services, storage, basic UI, booths, vendors, checkout |
| Phase 2: Reports & Export | 3 weeks | Report generation, printing, **export/import, cross-browser portability** |
| Phase 3: Polish & i18n | 3 weeks | Responsive design, DE/EN localization, PWA, accessibility, File System API |
| Phase 4: Testing & Launch | 2 weeks | E2E tests, browser testing, performance optimization |
| **Total** | **12 weeks** | **MVP Release** |

**Accelerated Timeline:** Consolidated phases reduce MVP delivery from 18 to 12 weeks by combining related work and deferring advanced features (CRDT sync, plugin system, cloud features) to post-MVP.

---

**Document Status:** Ready for Review  
**Next Steps:** 
1. Review and approval of architecture design
2. Proceed to Implementation Details specification
3. Set up initial project structure
