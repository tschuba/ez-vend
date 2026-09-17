---
title: Implementation Specification
nav_order: 6
parent: Redesign
---

# ez-vend Implementation Specification

---
**Document Status:** Active Guide
**Last Updated:** 2026-04-04
**Purpose:** Detailed implementation guide for the Rust/WASM redesign, including historical implementation planning that may now be partly superseded by current docs.

---

**Document Version:** 1.0  
**Date:** March 19, 2026  
**Status:** Design Phase  
**Related Documents:** [02_ARCHITECTURE.md](02_ARCHITECTURE.md), [03_IMPROVEMENTS.md](03_IMPROVEMENTS.md), [REDESIGN_SUMMARY.md](REDESIGN_SUMMARY.md)

---

## Table of Contents

1. [Project Structure](#1-project-structure)
2. [Core Domain Implementation](#2-core-domain-implementation)
3. [Storage Layer Specification](#3-storage-layer-specification)
4. [Frontend Implementation](#4-frontend-implementation)
5. [Backend Implementation (Optional)](#5-backend-implementation-optional)
6. [Cross-Browser Data Portability & Synchronization](#6-cross-browser-data-portability--synchronization)
7. [Data Migration from ez-booth](#7-data-migration-from-ez-booth)
8. [Build & Deployment](#8-build--deployment)
9. [Testing Strategy](#9-testing-strategy)
10. [Performance Optimization](#10-performance-optimization)
11. [Error Handling Implementation](#11-error-handling-implementation)
12. [Security Implementation](#12-security-implementation)

---

## 1. Project Structure

### 1.1 Workspace Configuration

**File:** `Cargo.toml` (workspace root)

```toml
[workspace]
members = [
    "crates/core",
    "crates/storage",
    "crates/frontend",
    "crates/server",
    "crates/shared",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["ez-vend contributors"]
license = "PolyForm-Noncommercial-1.0.0"
repository = "https://github.com/tschuba/ez-vend"

[workspace.dependencies]
# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# UUID
uuid = { version = "1.7", features = ["v4", "serde"] }

# Decimal (for currency)
rust_decimal = { version = "1.34", features = ["serde"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Async runtime
tokio = { version = "1.36", features = ["full"] }
futures = "0.3"

# Validation
validator = { version = "0.18", features = ["derive"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[profile.release]
opt-level = "z"     # Optimize for size
lto = true          # Enable Link Time Optimization
codegen-units = 1   # Better optimization, slower compile
panic = "abort"     # Smaller binary
strip = true        # Remove symbols
```

### 1.2 Directory Structure

```
ez-vend/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── README.md
├── LICENSE
│
├── crates/
│   ├── core/                     # Domain logic (no dependencies)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── entities/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── booth.rs      # Booth entity
│   │   │   │   ├── vendor.rs     # Vendor entity
│   │   │   │   ├── purchase.rs   # Purchase + PurchaseItem
│   │   │   │   └── ids.rs        # Type-safe IDs
│   │   │   ├── services/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── booth_service.rs
│   │   │   │   ├── vendor_service.rs
│   │   │   │   ├── purchase_service.rs
│   │   │   │   ├── charging_service.rs
│   │   │   │   └── reporting_service.rs
│   │   │   ├── validation/
│   │   │   │   ├── mod.rs
│   │   │   │   └── rules.rs
│   │   │   └── error.rs
│   │   └── tests/
│   │
│   ├── storage/                  # Storage abstraction
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── repository/       # Repository traits
│   │   │   │   ├── mod.rs
│   │   │   │   ├── booth_repo.rs
│   │   │   │   ├── vendor_repo.rs
│   │   │   │   └── purchase_repo.rs
│   │   │   ├── indexeddb/        # IndexedDB implementation
│   │   │   │   ├── mod.rs
│   │   │   │   ├── database.rs
│   │   │   │   ├── booth_repo.rs
│   │   │   │   ├── vendor_repo.rs
│   │   │   │   └── purchase_repo.rs
│   │   │   ├── sql/              # SQL implementation (server)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── migrations/
│   │   │   │   └── repos.rs
│   │   │   └── memory/           # In-memory (testing)
│   │   │       └── mod.rs
│   │   └── tests/
│   │
│   ├── frontend/                 # WASM UI (Leptos)
│   │   ├── Cargo.toml
│   │   ├── Trunk.toml
│   │   ├── index.html
│   │   ├── style/
│   │   │   ├── main.css
│   │   │   └── tailwind.css
│   │   ├── assets/
│   │   │   ├── icons/
│   │   │   └── manifest.json
│   │   ├── locales/              # i18n translations (NEW)
│   │   │   ├── de.json           # German translations
│   │   │   ├── en.json           # English translations
│   │   │   └── translations.json # i18n config
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── app.rs            # Root component
│   │   │   ├── router.rs         # Route definitions
│   │   │   ├── pages/            # Page components
│   │   │   │   ├── mod.rs
│   │   │   │   ├── home.rs
│   │   │   │   ├── booth_list.rs
│   │   │   │   ├── booth_detail.rs
│   │   │   │   ├── checkout.rs
│   │   │   │   ├── reports.rs
│   │   │   │   └── sync.rs
│   │   │   ├── components/       # Reusable components
│   │   │   │   ├── mod.rs
│   │   │   │   ├── button.rs
│   │   │   │   ├── table.rs
│   │   │   │   ├── form.rs
│   │   │   │   └── modal.rs
│   │   │   ├── i18n/             # i18n setup & formatters (NEW)
│   │   │   │   ├── mod.rs
│   │   │   │   ├── locale.rs      # Locale detection & switching
│   │   │   │   └── formatters.rs  # Currency, date, number formatting
│   │   │   ├── state/            # Global state
│   │   │   │   ├── mod.rs
│   │   │   │   ├── app_state.rs
│   │   │   │   └── storage_state.rs
│   │   │   ├── hooks/            # Custom hooks
│   │   │   │   └── mod.rs
│   │   │   └── utils/
│   │   │       ├── mod.rs
│   │   │       └── format.rs
│   │   └── tests/
│   │
│   ├── server/                   # Optional backend
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── config.rs
│   │   │   ├── handlers/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── booth.rs
│   │   │   │   ├── vendor.rs
│   │   │   │   ├── purchase.rs
│   │   │   │   └── sync.rs
│   │   │   ├── middleware/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── auth.rs
│   │   │   │   └── logging.rs
│   │   │   └── sync/
│   │   │       ├── mod.rs
│   │   │       └── protocol.rs
│   │   └── tests/
│   │
│   └── shared/                   # Client-server shared types
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── dto/              # Data transfer objects
│           │   └── mod.rs
│           └── protocol/         # Sync protocol
│               └── mod.rs
│
├── docs/                         # Documentation
│   ├── SPEC.md
│   ├── ANALYSIS.md
│   ├── ARCHITECTURE.md
│   ├── IMPROVEMENTS.md
│   ├── IMPLEMENTATION.md         # This file
│   └── api/                      # API documentation
│
├── tests/                        # Integration tests
│   ├── integration/
│   └── e2e/
│
├── scripts/                      # Build/deploy scripts
│   ├── build.sh
│   ├── test.sh
│   └── deploy.sh
│
└── .github/                      # CI/CD
    └── workflows/
        └── ci.yml
```

---

## 2. Core Domain Implementation

### 2.1 Entity Definitions

**File:** `crates/core/src/entities/ids.rs`

```rust
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            pub fn as_str(&self) -> String {
                self.0.to_string()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

define_id!(BoothId);
define_id!(PurchaseId);
define_id!(ItemId);

// VendorId is special - user-provided string, not UUID
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VendorId(String);

impl VendorId {
    pub fn new(id: String) -> Self {
        Self(id)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Smart sorting: numeric IDs sorted numerically, alphanumeric sorted lexicographically
    pub fn compare_smart(&self, other: &VendorId) -> std::cmp::Ordering {
        match (self.0.parse::<u64>(), other.0.parse::<u64>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),                    // Both numeric: 1 < 2 < 10
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,    // Numeric before alphanumeric
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater, // Alphanumeric after numeric
            (Err(_), Err(_)) => self.0.cmp(&other.0),       // Both text: lexicographic
        }
    }
}

impl fmt::Display for VendorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for VendorId {
    fn from(s: String) -> Self {
        Self(s)
    }
}
```

**File:** `crates/core/src/entities/booth.rs`

```rust
use crate::entities::ids::BoothId;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct Booth {
    pub id: BoothId,
    
    #[validate(length(min = 1, max = 200))]
    pub description: String,
    
    pub date: NaiveDate,
    
    #[validate]
    pub fees: FeeConfig,
    
    pub status: BoothStatus,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct FeeConfig {
    #[validate(range(min = 0.0))]
    pub participation_fee: Decimal,
    
    #[validate(range(min = 0.0, max = 100.0))]
    pub sales_fee_percent: Decimal,
    
    #[validate(range(min = 0.0))]
    pub rounding_step: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum BoothStatus {
    Open,
    Closed { closed_at: DateTime<Utc> },
}

impl Booth {
    pub fn new(description: String, date: NaiveDate, fees: FeeConfig) -> Self {
        let now = Utc::now();
        Self {
            id: BoothId::new(),
            description,
            date,
            fees,
            status: BoothStatus::Open,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn close(&mut self) {
        let now = Utc::now();
        self.status = BoothStatus::Closed { closed_at: now };
        self.updated_at = now;
    }

    pub fn is_closed(&self) -> bool {
        matches!(self.status, BoothStatus::Closed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    #[test]
    fn test_booth_creation() {
        let fees = FeeConfig {
            participation_fee: dec!(10.00),
            sales_fee_percent: dec!(5.0),
            rounding_step: dec!(0.50),
        };
        
        let booth = Booth::new(
            "Test Booth".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 19).unwrap(),
            fees,
        );
        
        assert!(!booth.is_closed());
        assert_eq!(booth.description, "Test Booth");
    }

    #[test]
    fn test_booth_closing() {
        let fees = FeeConfig {
            participation_fee: dec!(10.00),
            sales_fee_percent: dec!(5.0),
            rounding_step: dec!(0.50),
        };
        
        let mut booth = Booth::new(
            "Test Booth".to_string(),
            NaiveDate::from_ymd_opt(2026, 3, 19).unwrap(),
            fees,
        );
        
        booth.close();
        assert!(booth.is_closed());
    }
}
```

**File:** `crates/core/src/entities/vendor.rs`

```rust
use crate::entities::ids::{BoothId, VendorId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vendor {
    pub id: VendorId,
    pub booth_id: BoothId,
    pub created_at: DateTime<Utc>,
}

impl Vendor {
    /// Create new vendor with user-provided ID (entered during checkout)
    pub fn new(id: String, booth_id: BoothId) -> Self {
        Self {
            id: VendorId::new(id),
            booth_id,
            created_at: Utc::now(),
        }
    }
}
```

**File:** `crates/core/src/entities/purchase.rs`

```rust
use crate::entities::ids::{BoothId, ItemId, PurchaseId, VendorId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct Purchase {
    pub id: PurchaseId,
    pub booth_id: BoothId,
    
    #[validate]
    pub items: Vec<PurchaseItem>,
    
    pub total: Decimal,
    pub purchased_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct PurchaseItem {
    pub id: ItemId,
    pub vendor_id: VendorId,
    
    #[validate(range(min = 0.0))]
    pub price: Decimal,
    
    pub purchased_at: DateTime<Utc>,
}

impl Purchase {
    pub fn new(booth_id: BoothId, items: Vec<PurchaseItem>) -> Self {
        let total = items.iter().map(|item| item.price).sum();
        
        Self {
            id: PurchaseId::new(),
            booth_id,
            items,
            total,
            purchased_at: Utc::now(),
        }
    }

    pub fn recalculate_total(&mut self) {
        self.total = self.items.iter().map(|item| item.price).sum();
    }
}

impl PurchaseItem {
    pub fn new(vendor_id: VendorId, price: Decimal) -> Self {
        Self {
            id: ItemId::new(),
            vendor_id,
            price,
            purchased_at: Utc::now(),
        }
    }
}
```

### 2.2 Service Implementations

**File:** `crates/core/src/services/charging_service.rs`

```rust
use crate::entities::booth::FeeConfig;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeCalculation {
    pub total_sales: Decimal,
    pub participation_fee: Decimal,
    pub sales_fee: Decimal,
    pub total_fees: Decimal,
    pub net_revenue: Decimal,
}

pub struct ChargingService;

impl ChargingService {
    pub fn calculate_fees(
        total_sales: Decimal,
        config: &FeeConfig,
    ) -> FeeCalculation {
        let participation = config.participation_fee;
        
        // Calculate sales fee as percentage
        let sales_fee_raw = total_sales * config.sales_fee_percent 
            / Decimal::from(100);
        
        // Apply rounding
        let sales_fee = Self::round_to_step(sales_fee_raw, config.rounding_step);
        
        let total_fees = participation + sales_fee;
        let net_revenue = total_sales - total_fees;
        
        FeeCalculation {
            total_sales,
            participation_fee: participation,
            sales_fee,
            total_fees,
            net_revenue,
        }
    }
    
    fn round_to_step(value: Decimal, step: Decimal) -> Decimal {
        if step.is_zero() {
            value
        } else {
            // Round to nearest step
            let steps = (value / step).round();
            steps * step
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::booth::FeeConfig;
    use rust_decimal_macros::dec;

    #[test]
    fn test_fee_calculation() {
        let config = FeeConfig {
            participation_fee: dec!(10.00),
            sales_fee_percent: dec!(5.0),
            rounding_step: dec!(0.50),
        };
        
        let result = ChargingService::calculate_fees(dec!(100.00), &config);
        
        assert_eq!(result.total_sales, dec!(100.00));
        assert_eq!(result.participation_fee, dec!(10.00));
        assert_eq!(result.sales_fee, dec!(5.00)); // 5% of 100
        assert_eq!(result.total_fees, dec!(15.00));
        assert_eq!(result.net_revenue, dec!(85.00));
    }

    #[test]
    fn test_rounding() {
        let config = FeeConfig {
            participation_fee: dec!(0.00),
            sales_fee_percent: dec!(5.0),
            rounding_step: dec!(0.50),
        };
        
        // 5% of 107 = 5.35, rounded to 5.50
        let result = ChargingService::calculate_fees(dec!(107.00), &config);
        assert_eq!(result.sales_fee, dec!(5.50));
    }
}
```

### 2.3 Repository Traits

**File:** `crates/core/src/services/booth_service.rs`

```rust
use crate::entities::booth::{Booth, FeeConfig};
use crate::entities::ids::BoothId;
use crate::error::CoreError;
use async_trait::async_trait;
use chrono::NaiveDate;

#[async_trait(?Send)] // ?Send for WASM compatibility
pub trait BoothRepository {
    async fn save(&self, booth: &Booth) -> Result<(), CoreError>;
    async fn find_by_id(&self, id: BoothId) -> Result<Option<Booth>, CoreError>;
    async fn find_all(&self) -> Result<Vec<Booth>, CoreError>;
    async fn delete(&self, id: BoothId) -> Result<(), CoreError>;
}

pub struct BoothService<R: BoothRepository> {
    repository: R,
}

impl<R: BoothRepository> BoothService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_booth(
        &self,
        description: String,
        date: NaiveDate,
        fees: FeeConfig,
    ) -> Result<Booth, CoreError> {
        let booth = Booth::new(description, date, fees);
        booth.validate()?;
        
        self.repository.save(&booth).await?;
        Ok(booth)
    }

    pub async fn get_booth(&self, id: BoothId) -> Result<Booth, CoreError> {
        self.repository
            .find_by_id(id)
            .await?
            .ok_or(CoreError::NotFound("Booth not found".to_string()))
    }

    pub async fn list_booths(&self) -> Result<Vec<Booth>, CoreError> {
        self.repository.find_all().await
    }

    pub async fn update_booth(&self, booth: Booth) -> Result<(), CoreError> {
        booth.validate()?;
        self.repository.save(&booth).await
    }

    pub async fn close_booth(&self, id: BoothId) -> Result<Booth, CoreError> {
        let mut booth = self.get_booth(id).await?;
        booth.close();
        self.repository.save(&booth).await?;
        Ok(booth)
    }

    pub async fn delete_booth(&self, id: BoothId) -> Result<(), CoreError> {
        self.repository.delete(id).await
    }
}
```

**File:** `crates/core/src/services/vendor_service.rs`

```rust
use crate::entities::vendor::Vendor;
use crate::entities::ids::{BoothId, VendorId};
use crate::error::CoreError;
use async_trait::async_trait;

#[async_trait(?Send)] // ?Send for WASM compatibility
pub trait VendorRepository {
    async fn save(&self, vendor: &Vendor) -> Result<(), CoreError>;
    async fn find_by_id(&self, booth_id: BoothId, vendor_id: &VendorId) -> Result<Option<Vendor>, CoreError>;
    async fn find_all(&self, booth_id: BoothId) -> Result<Vec<Vendor>, CoreError>;
    async fn delete(&self, booth_id: BoothId, vendor_id: &VendorId) -> Result<(), CoreError>;
}

pub struct VendorService<R: VendorRepository> {
    repository: R,
}

impl<R: VendorRepository> VendorService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Get or create vendor by ID (auto-created during checkout)
    pub async fn get_or_create(
        &self,
        booth_id: BoothId,
        vendor_id: String,
    ) -> Result<Vendor, CoreError> {
        let id = VendorId::new(vendor_id.clone());
        
        if let Some(vendor) = self.repository.find_by_id(booth_id, &id).await? {
            Ok(vendor)
        } else {
            let vendor = Vendor::new(vendor_id, booth_id);
            self.repository.save(&vendor).await?;
            Ok(vendor)
        }
    }

    /// List all vendors with smart sorting.
    /// Numeric IDs (e.g., "1", "42") sorted numerically: 1, 2, 10, 42
    /// Alphanumeric IDs sorted lexicographically after numeric IDs.
    /// Critical for correct print order in vendor reports.
    pub async fn list_vendors(&self, booth_id: BoothId) -> Result<Vec<Vendor>, CoreError> {
        let mut vendors = self.repository.find_all(booth_id).await?;
        vendors.sort_by(|a, b| a.id.compare_smart(&b.id));
        Ok(vendors)
    }

    pub async fn get_vendor(
        &self,
        booth_id: BoothId,
        vendor_id: String,
    ) -> Result<Vendor, CoreError> {
        let id = VendorId::new(vendor_id);
        self.repository
            .find_by_id(booth_id, &id)
            .await?
            .ok_or(CoreError::NotFound("Vendor not found".to_string()))
    }

    pub async fn delete_vendor(
        &self,
        booth_id: BoothId,
        vendor_id: String,
    ) -> Result<(), CoreError> {
        let id = VendorId::new(vendor_id);
        self.repository.delete(booth_id, &id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::ids::VendorId;

    #[test]
    fn test_vendor_id_sorting() {
        let ids = vec![
            VendorId::new("10".to_string()),
            VendorId::new("2".to_string()),
            VendorId::new("1".to_string()),
            VendorId::new("V5".to_string()),
            VendorId::new("25".to_string()),
            VendorId::new("A3".to_string()),
        ];
        
        let mut sorted = ids.clone();
        sorted.sort_by(|a, b| a.compare_smart(b));
        
        // Expected: numeric IDs first (1, 2, 10, 25), then alphanumeric (A3, V5)
        assert_eq!(sorted[0].as_str(), "1");
        assert_eq!(sorted[1].as_str(), "2");
        assert_eq!(sorted[2].as_str(), "10");
        assert_eq!(sorted[3].as_str(), "25");
        assert_eq!(sorted[4].as_str(), "A3");
        assert_eq!(sorted[5].as_str(), "V5");
    }
}
```

---

## 3. Storage Layer Specification

### 3.1 IndexedDB Implementation

**File:** `crates/storage/Cargo.toml`

```toml
[package]
name = "ez-vend-storage"
version.workspace = true
edition.workspace = true

[dependencies]
ez-vend-core = { path = "../core" }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }
bincode = { workspace = true }

# WASM
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "Window",
    "IdbFactory",
    "IdbDatabase",
    "IdbObjectStore",
    "IdbTransaction",
    "IdbRequest",
    "IdbOpenDbRequest",
    "IdbCursor",
    "IdbKeyRange",
] }

# IndexedDB wrapper
rexie = "0.6"

# Error handling
thiserror = { workspace = true }
anyhow = { workspace = true }

# Async
futures = { workspace = true }

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

**File:** `crates/storage/src/indexeddb/database.rs`

```rust
use rexie::{Rexie, TransactionMode};
use wasm_bindgen::prelude::*;

const DB_NAME: &str = "ez_vend_v1";
const DB_VERSION: u32 = 1;

pub struct Database {
    db: Rexie,
}

impl Database {
    pub async fn new() -> Result<Self, JsValue> {
        let rexie = Rexie::builder(DB_NAME)
            .version(DB_VERSION)
            .add_object_store(
                rexie::ObjectStore::new("booths")
                    .key_path("id")
                    .add_index(rexie::Index::new("date", "date"))
                    .add_index(rexie::Index::new("status", "status.type"))
            )
            .add_object_store(
                rexie::ObjectStore::new("vendors")
                    .key_path_array(&["booth_id", "id"])
                    .add_index(rexie::Index::new("booth_id", "booth_id"))
            )
            .add_object_store(
                rexie::ObjectStore::new("purchases")
                    .key_path_array(&["booth_id", "id"])
                    .add_index(rexie::Index::new("booth_id", "booth_id"))
                    .add_index(rexie::Index::new("purchased_at", "purchased_at"))
            )
            .add_object_store(
                rexie::ObjectStore::new("purchase_items")
                    .key_path_array(&["booth_id", "purchase_id", "id"])
                    .add_index(rexie::Index::new("booth_id", "booth_id"))
                    .add_index(rexie::Index::new("vendor_id", "vendor_id"))
                    .add_index(rexie::Index::new("purchased_at", "purchased_at"))
            )
            .build()
            .await?;
        
        Ok(Self { db: rexie })
    }

    pub fn transaction(
        &self,
        stores: &[&str],
        mode: TransactionMode,
    ) -> Result<rexie::Transaction, JsValue> {
        self.db.transaction(stores, mode)
    }
}
```

**File:** `crates/storage/src/indexeddb/booth_repo.rs`

```rust
use crate::indexeddb::database::Database;
use async_trait::async_trait;
use ez_vend_core::entities::booth::Booth;
use ez_vend_core::entities::ids::BoothId;
use ez_vend_core::error::CoreError;
use ez_vend_core::services::booth_service::BoothRepository;
use rexie::TransactionMode;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsValue;

pub struct IndexedDbBoothRepository {
    db: Database,
}

impl IndexedDbBoothRepository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait(?Send)]
impl BoothRepository for IndexedDbBoothRepository {
    async fn save(&self, booth: &Booth) -> Result<(), CoreError> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadWrite)
            .map_err(|e| CoreError::Storage(format!("Transaction error: {:?}", e)))?;
        
        let store = transaction
            .store("booths")
            .map_err(|e| CoreError::Storage(format!("Store error: {:?}", e)))?;
        
        let value = to_value(booth)
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        
        store
            .put(&value, None)
            .await
            .map_err(|e| CoreError::Storage(format!("Put error: {:?}", e)))?;
        
        transaction
            .done()
            .await
            .map_err(|e| CoreError::Storage(format!("Commit error: {:?}", e)))?;
        
        Ok(())
    }

    async fn find_by_id(&self, id: BoothId) -> Result<Option<Booth>, CoreError> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadOnly)
            .map_err(|e| CoreError::Storage(format!("Transaction error: {:?}", e)))?;
        
        let store = transaction
            .store("booths")
            .map_err(|e| CoreError::Storage(format!("Store error: {:?}", e)))?;
        
        let key = to_value(&id.as_str())
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        
        let result = store
            .get(&key)
            .await
            .map_err(|e| CoreError::Storage(format!("Get error: {:?}", e)))?;
        
        match result {
            Some(value) => {
                let booth: Booth = from_value(value)
                    .map_err(|e| CoreError::Deserialization(e.to_string()))?;
                Ok(Some(booth))
            }
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<Booth>, CoreError> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadOnly)
            .map_err(|e| CoreError::Storage(format!("Transaction error: {:?}", e)))?;
        
        let store = transaction
            .store("booths")
            .map_err(|e| CoreError::Storage(format!("Store error: {:?}", e)))?;
        
        let values = store
            .get_all(None, None)
            .await
            .map_err(|e| CoreError::Storage(format!("GetAll error: {:?}", e)))?;
        
        let booths: Vec<Booth> = values
            .into_iter()
            .map(|v| from_value(v).map_err(|e| CoreError::Deserialization(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        
        Ok(booths)
    }

    async fn delete(&self, id: BoothId) -> Result<(), CoreError> {
        let transaction = self
            .db
            .transaction(&["booths"], TransactionMode::ReadWrite)
            .map_err(|e| CoreError::Storage(format!("Transaction error: {:?}", e)))?;
        
        let store = transaction
            .store("booths")
            .map_err(|e| CoreError::Storage(format!("Store error: {:?}", e)))?;
        
        let key = to_value(&id.as_str())
            .map_err(|e| CoreError::Serialization(e.to_string()))?;
        
        store
            .delete(&key)
            .await
            .map_err(|e| CoreError::Storage(format!("Delete error: {:?}", e)))?;
        
        transaction
            .done()
            .await
            .map_err(|e| CoreError::Storage(format!("Commit error: {:?}", e)))?;
        
        Ok(())
    }
}
```

---

## 4. Frontend Implementation

### 4.1 Leptos Configuration

**File:** `crates/frontend/Cargo.toml`

```toml
[package]
name = "ez-vend-frontend"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
ez-vend-core = { path = "../core" }
ez-vend-storage = { path = "../storage" }

# Leptos
leptos = { version = "0.6", features = ["csr", "nightly"] }
leptos_meta = { version = "0.6", features = ["csr"] }
leptos_router = { version = "0.6", features = ["csr"] }
leptos_i18n = { version = "0.3", features = ["csr"] }

# WASM
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = { version = "0.3", features = ["Window", "Document", "HtmlElement", "Navigator"] }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Date/Time
chrono = { workspace = true, features = ["wasmbind"] }

# Logging
console_error_panic_hook = "0.1"
tracing = { workspace = true }
tracing-wasm = "0.2"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

**File:** `crates/frontend/Trunk.toml`

```toml
[build]
target = "index.html"
release = true
dist = "dist"
public_url = "/"

[watch]
ignore = ["dist", "target"]

[serve]
address = "127.0.0.1"
port = 8080
open = false

[clean]
dist = "dist"
cargo = true
```

### 4.2 Main Application

**File:** `crates/frontend/src/main.rs`

```rust
use leptos::*;
use ez_vend_frontend::app::App;

fn main() {
    // Set up logging
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();

    // Mount the app
    mount_to_body(|| view! { <App/> })
}
```

**File:** `crates/frontend/src/app.rs`

```rust
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::pages::{
    home::HomePage,
    booth_list::BoothListPage,
    booth_detail::BoothDetailPage,
    checkout::CheckoutPage,
    reports::ReportsPage,
    sync::SyncPage,
};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="leptos" href="/pkg/ez-vend-frontend.css"/>
        <Title text="ez-vend"/>
        <Meta name="description" content="Portable booth management system"/>
        
        <Router>
            <nav class="navbar">
                <A href="/">"Home"</A>
                <A href="/booths">"Booths"</A>
                <A href="/checkout">"Checkout"</A>
                <A href="/reports">"Reports"</A>
                <A href="/sync">"Sync"</A>
            </nav>
            
            <main>
                <Routes>
                    <Route path="/" view=HomePage/>
                    <Route path="/booths" view=BoothListPage/>
                    <Route path="/booths/:id" view=BoothDetailPage/>
                    <Route path="/checkout" view=CheckoutPage/>
                    <Route path="/reports" view=ReportsPage/>
                    <Route path="/sync" view=SyncPage/>
                </Routes>
            </main>
        </Router>
    }
}
```

### 4.3 State Management

**File:** `crates/frontend/src/state/app_state.rs`

```rust
use ez_vend_core::entities::booth::Booth;
use ez_vend_core::entities::ids::BoothId;
use ez_vend_storage::indexeddb::database::Database;
use leptos::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct AppState {
    pub booths: RwSignal<HashMap<BoothId, Booth>>,
    pub selected_booth: RwSignal<Option<BoothId>>,
    pub database: Database,
}

impl AppState {
    pub async fn new() -> Result<Self, String> {
        let database = Database::new()
            .await
            .map_err(|e| format!("Database init error: {:?}", e))?;
        
        Ok(Self {
            booths: create_rw_signal(HashMap::new()),
            selected_booth: create_rw_signal(None),
            database,
        })
    }

    pub async fn load_booths(&self) -> Result<(), String> {
        // Implementation using repository
        Ok(())
    }

    pub fn select_booth(&self, id: BoothId) {
        self.selected_booth.set(Some(id));
    }

    pub fn get_selected_booth(&self) -> Option<Booth> {
        self.selected_booth.get()
            .and_then(|id| self.booths.get().get(&id).cloned())
    }
}
```

### 4.4 Internationalization (i18n) Implementation

#### 4.4.1 Translation Files

**File:** `crates/frontend/locales/de.json` (German - Primary)

```json
{
  "common": {
    "save": "Speichern",
    "cancel": "Abbrechen",
    "delete": "Löschen",
    "edit": "Bearbeiten",
    "close": "Schließen",
    "yes": "Ja",
    "no": "Nein"
  },
  "booth": {
    "title": "Stand",
    "name_label": "Name",
    "description_label": "Beschreibung",
    "list_title": "Stände",
    "create": "Stand erstellen",
    "edit": "Stand bearbeiten"
  },
  "checkout": {
    "title": "Kasse",
    "total": "Gesamt",
    "confirm": "Bestätigen",
    "amount": "Betrag"
  },
  "report": {
    "vendor_receipt": "Verkäufer-Quittung",
    "total": "Gesamtsumme",
    "period": "Zeitraum",
    "date": "Datum",
    "item": "Artikel",
    "amount": "Betrag",
    "quantity": "Anzahl",
    "export": "Exportieren",
    "print": "Drucken"
  },
  "sync": {
    "title": "Synchronisierung",
    "status": "Status",
    "last_sync": "Letzte Synchronisierung"
  },
  "export": {
    "title": "Daten exportieren",
    "format": "Format",
    "download": "Herunterladen"
  },
  "import": {
    "title": "Daten importieren",
    "select_file": "Datei auswählen",
    "upload": "Hochladen"
  }
}
```

**File:** `crates/frontend/locales/en.json` (English - Fallback)

```json
{
  "common": {
    "save": "Save",
    "cancel": "Cancel",
    "delete": "Delete",
    "edit": "Edit",
    "close": "Close",
    "yes": "Yes",
    "no": "No"
  },
  "booth": {
    "title": "Booth",
    "name_label": "Name",
    "description_label": "Description",
    "list_title": "Booths",
    "create": "Create Booth",
    "edit": "Edit Booth"
  },
  "checkout": {
    "title": "Checkout",
    "total": "Total",
    "confirm": "Confirm",
    "amount": "Amount"
  },
  "report": {
    "vendor_receipt": "Vendor Receipt",
    "total": "Total",
    "period": "Period",
    "date": "Date",
    "item": "Item",
    "amount": "Amount",
    "quantity": "Quantity",
    "export": "Export",
    "print": "Print"
  },
  "sync": {
    "title": "Synchronization",
    "status": "Status",
    "last_sync": "Last Sync"
  },
  "export": {
    "title": "Export Data",
    "format": "Format",
    "download": "Download"
  },
  "import": {
    "title": "Import Data",
    "select_file": "Select File",
    "upload": "Upload"
  }
}
```

**File:** `crates/frontend/locales/translations.json` (Config)

```json
{
  "default": "de",
  "locales": ["de", "en"]
}
```

#### 4.4.2 i18n Module Implementation

**File:** `crates/frontend/src/i18n/mod.rs`

```rust
pub mod locale;
pub mod formatters;

pub use locale::{init_i18n, get_locale, set_locale, Locale};
pub use formatters::{format_currency, format_date, format_number};
```

**File:** `crates/frontend/src/i18n/locale.rs`

```rust
use leptos::*;
use leptos_i18n::*;
use web_sys::window;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locale {
    De,
    En,
}

impl Locale {
    pub fn from_str(s: &str) -> Self {
        match s.split('-').next() {
            Some("de") => Locale::De,
            Some("en") => Locale::En,
            _ => Locale::De, // Default to German
        }
    }
    
    pub fn to_str(&self) -> &'static str {
        match self {
            Locale::De => "de",
            Locale::En => "en",
        }
    }
}

/// Initialize i18n with browser locale detection
pub fn init_i18n() -> Locale {
    // Check localStorage for user preference
    if let Ok(Some(storage)) = window()
        .and_then(|w| w.local_storage().ok().flatten())
    {
        if let Ok(Some(saved_locale)) = storage.get_item("ez_vend_locale") {
            return Locale::from_str(&saved_locale);
        }
    }
    
    // Detect browser language
    if let Some(window) = window() {
        if let Some(lang) = window.navigator().language() {
            return Locale::from_str(&lang);
        }
    }
    
    // Default to German
    Locale::De
}

/// Get current locale from context
pub fn get_locale() -> Locale {
    use_context::<RwSignal<Locale>>()
        .map(|signal| signal.get())
        .unwrap_or(Locale::De)
}

/// Set locale and persist to localStorage
pub fn set_locale(locale: Locale) {
    if let Some(signal) = use_context::<RwSignal<Locale>>() {
        signal.set(locale);
        
        // Persist to localStorage
        if let Ok(Some(storage)) = window()
            .and_then(|w| w.local_storage().ok().flatten())
        {
            let _ = storage.set_item("ez_vend_locale", locale.to_str());
        }
    }
}
```

**File:** `crates/frontend/src/i18n/formatters.rs`

```rust
use super::Locale;
use chrono::{DateTime, Local};
use rust_decimal::Decimal;

/// Format currency according to locale
pub fn format_currency(amount: Decimal, locale: Locale) -> String {
    let value = amount.to_string();
    
    match locale {
        Locale::De => {
            // German: 12,50 €
            format!("{} €", value.replace('.', ','))
        }
        Locale::En => {
            // English: €12.50
            format!("€{}", value)
        }
    }
}

/// Format date according to locale
pub fn format_date(date: DateTime<Local>, locale: Locale) -> String {
    match locale {
        Locale::De => date.format("%d.%m.%Y").to_string(), // 19.03.2026
        Locale::En => date.format("%m/%d/%Y").to_string(), // 03/19/2026
    }
}

/// Format number according to locale
pub fn format_number(num: f64, locale: Locale) -> String {
    match locale {
        Locale::De => {
            // German: 1.234,56
            let formatted = format!("{:.2}", num);
            formatted
                .replace('.', ",")
                .replace(',', "X")
                .replace('.', ",")
                .replace('X', ".")
        }
        Locale::En => {
            // English: 1,234.56
            format!("{:.2}", num)
        }
    }
}
```

#### 4.4.3 App Integration

Update `crates/frontend/src/app.rs` to provide i18n context:

```rust
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use crate::i18n::{init_i18n, Locale};

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    
    // Initialize i18n
    let locale = create_rw_signal(init_i18n());
    provide_context(locale);

    view! {
        <Stylesheet id="leptos" href="/pkg/ez-vend-frontend.css"/>
        <Title text="ez-vend"/>
        <Meta name="description" content="Portable booth management system"/>
        
        <Router>
            <nav class="navbar">
                <A href="/">{move || t!(locale, "nav.home")}</A>
                <A href="/booths">{move || t!(locale, "booth.list_title")}</A>
                <A href="/checkout">{move || t!(locale, "checkout.title")}</A>
                <A href="/reports">{move || t!(locale, "report.title")}</A>
                <A href="/sync">{move || t!(locale, "sync.title")}</A>
                
                {/* Language switcher */}
                <select 
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        let new_locale = if value == "de" { Locale::De } else { Locale::En };
                        set_locale(new_locale);
                    }
                >
                    <option value="de" selected=move || locale.get() == Locale::De>"Deutsch"</option>
                    <option value="en" selected=move || locale.get() == Locale::En>"English"</option>
                </select>
            </nav>
            
            <main>
                <Routes>
                    {/* Routes... */}
                </Routes>
            </main>
        </Router>
    }
}
```

#### 4.4.4 Report Template Localization

**File:** `crates/frontend/src/pages/reports.rs`

```rust
use crate::i18n::{get_locale, format_currency, format_date, Locale};

pub fn render_vendor_report(
    vendor: &Vendor,
    items: &[PurchaseItem],
) -> String {
    let locale = get_locale();
    let t = get_translations(locale);
    
    let items_html = items.iter()
        .map(|item| format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
            format_date(item.date, locale),
            item.description,
            format_currency(item.amount, locale)
        ))
        .collect::<String>();
    
    let total = items.iter()
        .map(|i| i.amount)
        .sum::<Decimal>();
    
    format!(r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="utf-8">
            <title>{}</title>
            <style>
                @media print {{
                    body {{ margin: 0; padding: 20px; }}
                }}
                table {{ width: 100%; border-collapse: collapse; }}
                th, td {{ padding: 8px; text-align: left; border-bottom: 1px solid #ddd; }}
                .total {{ margin-top: 20px; font-weight: bold; text-align: right; }}
            </style>
        </head>
        <body>
            <h1>{}</h1>
            <table>
                <thead>
                    <tr>
                        <th>{}</th>
                        <th>{}</th>
                        <th>{}</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
            <div class="total">{}: {}</div>
        </body>
        </html>
    "#,
        t.report.vendor_receipt,
        t.report.vendor_receipt,
        t.report.date,
        t.report.item,
        t.report.amount,
        items_html,
        t.report.total,
        format_currency(total, locale)
    )
}

// Multi-vendor batch report with page breaks
pub fn render_multi_vendor_report(
    vendors: &[Vendor],
    items_by_vendor: &HashMap<VendorId, Vec<PurchaseItem>>,
) -> String {
    let locale = get_locale();
    let t = get_translations(locale);
    
    let vendor_pages: Vec<String> = vendors
        .iter()
        .map(|vendor| {
            let items = items_by_vendor
                .get(&vendor.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            
            format!(
                r#"<div class="vendor-report-page">
                    <h1>{} - {}</h1>
                    <table>
                        <thead>
                            <tr>
                                <th>{}</th>
                                <th>{}</th>
                                <th>{}</th>
                            </tr>
                        </thead>
                        <tbody>{}</tbody>
                    </table>
                    <div class="total">{}: {}</div>
                </div>"#,
                t.report.vendor_receipt,
                vendor.vendor_id,
                t.report.date,
                t.report.item,
                t.report.amount,
                render_items(items, locale),
                t.report.total,
                format_currency(calculate_total(items), locale)
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
        t.report.all_vendor_receipts,
        vendor_pages.join("\n")
    )
}
```

**Implementation Notes:**
- Each vendor report wrapped in `.vendor-report-page` div with `page-break-after: always`
- Last vendor doesn't force page break to avoid blank page
- Screen preview shows separated "cards" with borders for visual verification
- Print output creates clean page breaks for easy distribution

---

## 5. Backend Implementation (Optional)

### 5.1 Server Configuration

**File:** `crates/server/Cargo.toml`

```toml
[package]
name = "ez-vend-server"
version.workspace = true
edition.workspace = true

[[bin]]
name = "server"
path = "src/main.rs"

[dependencies]
ez-vend-core = { path = "../core" }
ez-vend-storage = { path = "../storage" }
ez-vend-shared = { path = "../shared" }

# Web framework
axum = { version = "0.7", features = ["macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }

# Async runtime
tokio = { workspace = true }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite", "chrono", "uuid"] }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Error handling
thiserror = { workspace = true }
anyhow = { workspace = true }

# Logging
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

# Config
config = "0.14"
```

**File:** `crates/server/src/main.rs`

```rust
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod handlers;
mod middleware;
mod sync;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = config::Config::load()?;

    // Build application routes
    let api_routes = Router::new()
        .route("/booths", get(handlers::booth::list_booths))
        .route("/booths", post(handlers::booth::create_booth))
        .route("/sync", post(handlers::sync::sync_data));

    let app = Router::new()
        .nest("/api", api_routes)
        .nest_service("/", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Server listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## 6. Cross-Browser Data Portability & Synchronization

### 6.1 Export/Import Service (Core Feature - Phase 6)

**File:** `crates/frontend/src/services/export_service.rs`

```rust
use ez_vend_core::entities::{booth::Booth, vendor::Vendor, purchase::Purchase};
use ez_vend_storage::indexeddb::database::Database;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use wasm_bindgen::JsValue;
use web_sys::{Blob, Url};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportData {
    pub version: String,              // Schema version for compatibility
    pub exported_at: DateTime<Utc>,   // Export timestamp
    pub client_id: String,            // Browser/device identifier
    pub booths: Vec<Booth>,
    pub vendors: Vec<Vendor>,
    pub purchases: Vec<Purchase>,
    pub checksum: String,             // Integrity verification
}

pub struct ExportService {
    db: Database,
}

impl ExportService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    pub async fn export_all_data(&self) -> Result<ExportData, JsValue> {
        // 1. Fetch all data from IndexedDB
        let booths = self.fetch_all_booths().await?;
        let vendors = self.fetch_all_vendors().await?;
        let purchases = self.fetch_all_purchases().await?;
        
        // 2. Calculate checksum for integrity
        let checksum = Self::calculate_checksum(&booths, &vendors, &purchases);
        
        // 3. Create export structure
        let export = ExportData {
            version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: Utc::now(),
            client_id: Self::get_client_id(),
            booths,
            vendors,
            purchases,
            checksum,
        };
        
        Ok(export)
    }
    
    pub async fn download_as_json(&self) -> Result<(), JsValue> {
        let export = self.export_all_data().await?;
        let json = serde_json::to_string_pretty(&export)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        let filename = format!(
            "ez-vend-export-{}.json",
            Utc::now().format("%Y%m%d-%H%M%S")
        );
        
        // Create blob and trigger download
        let blob = Blob::new_with_str_sequence(&js_sys::Array::from_iter([
            JsValue::from_str(&json)
        ]))?;
        
        let url = Url::create_object_url_with_blob(&blob)?;
        
        // Trigger browser download
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let a = document.create_element("a")?;
        a.set_attribute("href", &url)?;
        a.set_attribute("download", &filename)?;
        a.dyn_ref::<web_sys::HtmlElement>().unwrap().click();
        
        Url::revoke_object_url(&url)?;
        Ok(())
    }
    
    fn calculate_checksum(
        booths: &[Booth],
        vendors: &[Vendor],
        purchases: &[Purchase],
    ) -> String {
        let mut hasher = DefaultHasher::new();
        booths.hash(&mut hasher);
        vendors.hash(&mut hasher);
        purchases.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    fn get_client_id() -> String {
        // Generate stable client ID from browser info
        format!(
            "{}-{}",
            web_sys::window().unwrap().navigator().user_agent().unwrap_or_default(),
            uuid::Uuid::new_v4()
        )
    }
    
    async fn fetch_all_booths(&self) -> Result<Vec<Booth>, JsValue> {
        // Implementation using BoothRepository
        todo!()
    }
    
    async fn fetch_all_vendors(&self) -> Result<Vec<Vendor>, JsValue> {
        // Implementation using VendorRepository
        todo!()
    }
    
    async fn fetch_all_purchases(&self) -> Result<Vec<Purchase>, JsValue> {
        // Implementation using PurchaseRepository
        todo!()
    }
}
```

### 6.2 Import Service with Merge Strategies

**File:** `crates/frontend/src/services/import_service.rs`

```rust
use ez_vend_core::entities::{booth::Booth, vendor::Vendor, purchase::Purchase};
use ez_vend_storage::indexeddb::database::Database;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use super::export_service::{ExportData, ExportService};

#[derive(Clone, Copy, Debug)]
pub enum MergeStrategy {
    Replace,  // Clear existing data and import all
    Merge,    // Merge by timestamp (newer wins)
    Preview,  // Show changes without applying
}

pub struct ImportResult {
    pub booths_imported: usize,
    pub vendors_imported: usize,
    pub purchases_imported: usize,
    pub conflicts: Vec<String>,
}

pub struct ImportService {
    db: Database,
}

impl ImportService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    
    pub async fn import_from_file(
        &self,
        file: web_sys::File,
        strategy: MergeStrategy,
    ) -> Result<ImportResult, JsValue> {
        // 1. Read file as text
        let text = Self::read_file_as_text(&file).await?;
        
        // 2. Parse JSON
        let export: ExportData = serde_json::from_str(&text)
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;
        
        // 3. Verify integrity
        self.verify_checksum(&export)?;
        
        // 4. Check version compatibility
        self.validate_schema_version(&export.version)?;
        
        // 5. Apply import strategy
        match strategy {
            MergeStrategy::Replace => self.import_replace(export).await,
            MergeStrategy::Merge => self.import_merge(export).await,
            MergeStrategy::Preview => self.preview_changes(export).await,
        }
    }
    
    async fn read_file_as_text(file: &web_sys::File) -> Result<String, JsValue> {
        let promise = file.text();
        let js_value = JsFuture::from(promise).await?;
        Ok(js_value.as_string().unwrap())
    }
    
    fn verify_checksum(&self, export: &ExportData) -> Result<(), JsValue> {
        let calculated = ExportService::calculate_checksum(
            &export.booths,
            &export.vendors,
            &export.purchases,
        );
        
        if calculated != export.checksum {
            return Err(JsValue::from_str(
                "Data integrity check failed - file may be corrupted"
            ));
        }
        
        Ok(())
    }
    
    fn validate_schema_version(&self, version: &str) -> Result<(), JsValue> {
        // Check if import version is compatible
        let current = env!("CARGO_PKG_VERSION");
        
        // Simple version check - can be enhanced with semver crate
        if version > current {
            return Err(JsValue::from_str(&format!(
                "Export from newer version ({}) - please update ez-vend",
                version
            )));
        }
        
        Ok(())
    }
    
    async fn import_replace(&self, export: ExportData) -> Result<ImportResult, JsValue> {
        // 1. Clear all existing data
        self.clear_all_data().await?;
        
        // 2. Import all data
        for booth in &export.booths {
            self.db.save_booth(booth).await?;
        }
        for vendor in &export.vendors {
            self.db.save_vendor(vendor).await?;
        }
        for purchase in &export.purchases {
            self.db.save_purchase(purchase).await?;
        }
        
        Ok(ImportResult {
            booths_imported: export.booths.len(),
            vendors_imported: export.vendors.len(),
            purchases_imported: export.purchases.len(),
            conflicts: vec![],
        })
    }
    
    async fn import_merge(&self, export: ExportData) -> Result<ImportResult, JsValue> {
        let mut conflicts = Vec::new();
        
        // Merge booths (newer timestamp wins)
        for booth in export.booths {
            match self.db.get_booth(&booth.id).await? {
                Some(existing) => {
                    if booth.updated_at > existing.updated_at {
                        self.db.save_booth(&booth).await?;
                    } else {
                        conflicts.push(format!(
                            "Booth {} kept existing (newer)",
                            booth.id
                        ));
                    }
                }
                None => {
                    self.db.save_booth(&booth).await?;
                }
            }
        }
        
        // Merge vendors (newer timestamp wins)
        for vendor in export.vendors {
            match self.db.get_vendor(&vendor.id).await? {
                Some(existing) => {
                    if vendor.created_at > existing.created_at {
                        self.db.save_vendor(&vendor).await?;
                    }
                }
                None => {
                    self.db.save_vendor(&vendor).await?;
                }
            }
        }
        
        // Merge purchases (newer timestamp wins)
        for purchase in export.purchases {
            match self.db.get_purchase(&purchase.id).await? {
                Some(existing) => {
                    if purchase.purchased_at > existing.purchased_at {
                        self.db.save_purchase(&purchase).await?;
                    }
                }
                None => {
                    self.db.save_purchase(&purchase).await?;
                }
            }
        }
        
        Ok(ImportResult {
            booths_imported: export.booths.len(),
            vendors_imported: export.vendors.len(),
            purchases_imported: export.purchases.len(),
            conflicts,
        })
    }
    
    async fn preview_changes(&self, export: ExportData) -> Result<ImportResult, JsValue> {
        // Calculate what would change without applying
        let mut conflicts = Vec::new();
        
        for booth in &export.booths {
            if let Some(existing) = self.db.get_booth(&booth.id).await? {
                if booth.updated_at != existing.updated_at {
                    conflicts.push(format!(
                        "Booth {} would be updated",
                        booth.id
                    ));
                }
            }
        }
        
        Ok(ImportResult {
            booths_imported: 0,
            vendors_imported: 0,
            purchases_imported: 0,
            conflicts,
        })
    }
    
    async fn clear_all_data(&self) -> Result<(), JsValue> {
        // Clear all object stores
        self.db.clear_booths().await?;
        self.db.clear_vendors().await?;
        self.db.clear_purchases().await?;
        Ok(())
    }
}
```

### 6.3 UI Integration - Sync Page

**File:** `crates/frontend/src/pages/sync.rs`

```rust
use leptos::*;
use wasm_bindgen::JsCast;
use super::services::{ExportService, ImportService, MergeStrategy};

#[component]
pub fn SyncPage() -> impl IntoView {
    let (export_status, set_export_status) = create_signal(None::<String>);
    let (import_status, set_import_status) = create_signal(None::<String>);
    let (merge_strategy, set_merge_strategy) = create_signal(MergeStrategy::Merge);
    
    let handle_export = move |_| {
        spawn_local(async move {
            set_export_status(Some("Exporting...".to_string()));
            
            let export_service = ExportService::new(/* db */);
            match export_service.download_as_json().await {
                Ok(_) => {
                    set_export_status(Some("✓ Export downloaded successfully".to_string()));
                }
                Err(e) => {
                    set_export_status(Some(format!("✗ Export failed: {:?}", e)));
                }
            }
        });
    };
    
    let handle_import = move |ev: web_sys::Event| {
        let input = ev.target().unwrap()
            .dyn_into::<web_sys::HtmlInputElement>().unwrap();
        
        if let Some(files) = input.files() {
            if files.length() > 0 {
                let file = files.get(0).unwrap();
                
                spawn_local(async move {
                    set_import_status(Some("Importing...".to_string()));
                    
                    let import_service = ImportService::new(/* db */);
                    match import_service.import_from_file(file, merge_strategy.get()).await {
                        Ok(result) => {
                            set_import_status(Some(format!(
                                "✓ Imported {} booths, {} vendors, {} purchases",
                                result.booths_imported,
                                result.vendors_imported,
                                result.purchases_imported
                            )));
                        }
                        Err(e) => {
                            set_import_status(Some(format!("✗ Import failed: {:?}", e)));
                        }
                    }
                });
            }
        }
    };
    
    let handle_strategy_change = move |ev: web_sys::Event| {
        let select = ev.target().unwrap()
            .dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        
        let strategy = match select.value().as_str() {
            "replace" => MergeStrategy::Replace,
            "merge" => MergeStrategy::Merge,
            "preview" => MergeStrategy::Preview,
            _ => MergeStrategy::Merge,
        };
        
        set_merge_strategy(strategy);
    };
    
    view! {
        <div class="sync-page max-w-4xl mx-auto p-6">
            <h1 class="text-3xl font-bold mb-6">"Cross-Browser Data Portability"</h1>
            
            <section class="export-section mb-8 p-6 bg-white rounded-lg shadow">
                <h2 class="text-2xl font-semibold mb-4">"Export Data"</h2>
                <p class="mb-4">"Download all your data as a JSON file for:"</p>
                <ul class="list-disc list-inside mb-4 space-y-2">
                    <li>"✓ Switching to another browser (Chrome → Firefox)"</li>
                    <li>"✓ Backing up your data safely"</li>
                    <li>"✓ Transferring to another device"</li>
                    <li>"✓ Sharing with team members"</li>
                </ul>
                <button 
                    on:click=handle_export
                    class="px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                >
                    "📥 Download Export"
                </button>
                {move || export_status.get().map(|status| view! {
                    <p class="mt-4 text-sm">{status}</p>
                })}
            </section>
            
            <section class="import-section p-6 bg-white rounded-lg shadow">
                <h2 class="text-2xl font-semibold mb-4">"Import Data"</h2>
                <p class="mb-4">"Upload a previously exported JSON file"</p>
                
                <div class="mb-4">
                    <label class="block mb-2 font-medium">"Import Strategy:"</label>
                    <select 
                        on:change=handle_strategy_change
                        class="w-full px-4 py-2 border rounded-lg"
                    >
                        <option value="merge" selected>"Merge with existing (newer wins)"</option>
                        <option value="replace">"Replace all data"</option>
                        <option value="preview">"Preview changes only"</option>
                    </select>
                </div>
                
                <input 
                    type="file" 
                    accept=".json"
                    on:change=handle_import
                    class="block w-full px-4 py-2 border rounded-lg cursor-pointer"
                />
                
                {move || import_status.get().map(|status| view! {
                    <p class="mt-4 text-sm">{status}</p>
                })}
                
                <div class="mt-6 p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
                    <p class="text-sm text-yellow-800">
                        "⚠️ Tip: Save exports to a cloud folder (Dropbox, Google Drive) for automatic sync across devices"
                    </p>
                </div>
            </section>
        </div>
    }
}
```

### 6.4 User Onboarding & Browser Switch Detection

**File:** `crates/frontend/src/components/welcome_screen.rs`

```rust
use leptos::*;
use leptos_router::*;
use wasm_bindgen::JsCast;
use web_sys;

#[derive(Clone, Debug)]
pub struct OnboardingState {
    pub is_first_visit: bool,
    pub has_data: bool,
    pub browser_name: String,
}

impl OnboardingState {
    pub async fn detect() -> Self {
        let has_data = Self::check_database_populated().await;
        let is_first_visit = Self::check_first_visit_flag();
        let browser_name = Self::detect_browser_name();
        
        Self {
            is_first_visit,
            has_data,
            browser_name,
        }
    }
    
    async fn check_database_populated() -> bool {
        // Check if any booths exist
        let db = Database::new().await.unwrap();
        let booth_count = db.count_booths().await.unwrap_or(0);
        booth_count > 0
    }
    
    fn check_first_visit_flag() -> bool {
        let window = web_sys::window().unwrap();
        let storage = window.local_storage().unwrap().unwrap();
        storage.get_item("ez_vend_visited").unwrap().is_none()
    }
    
    fn detect_browser_name() -> String {
        let window = web_sys::window().unwrap();
        let ua = window.navigator().user_agent().unwrap_or_default();
        
        if ua.contains("Firefox") {
            "Firefox".to_string()
        } else if ua.contains("Edg") {
            "Edge".to_string()
        } else if ua.contains("Chrome") {
            "Chrome".to_string()
        } else if ua.contains("Safari") {
            "Safari".to_string()
        } else {
            "your browser".to_string()
        }
    }
    
    pub fn mark_visited() {
        let window = web_sys::window().unwrap();
        let storage = window.local_storage().unwrap().unwrap();
        let _ = storage.set_item("ez_vend_visited", "true");
    }
}

#[component]
pub fn WelcomeScreen() -> impl IntoView {
    let onboarding = create_resource(|| (), |_| OnboardingState::detect());
    
    view! {
        <Suspense fallback=|| view! { 
            <div class="flex items-center justify-center h-screen">
                <div class="animate-spin rounded-full h-32 w-32 border-b-2 border-blue-600"></div>
            </div>
        }>
            {move || onboarding.get().map(|state| {
                if state.is_first_visit && !state.has_data {
                    view! { <FirstTimeWelcome state=state /> }.into_view()
                } else if state.has_data {
                    view! { <Navigate path="/booths" /> }.into_view()
                } else {
                    view! { <EmptyStatePrompt /> }.into_view()
                }
            })}
        </Suspense>
    }
}

#[component]
pub fn FirstTimeWelcome(state: OnboardingState) -> impl IntoView {
    let navigate = use_navigate();
    
    let handle_import = move |_| {
        OnboardingState::mark_visited();
        navigate("/sync", Default::default());
    };
    
    let handle_new = move |_| {
        OnboardingState::mark_visited();
        navigate("/booths/new", Default::default());
    };
    
    view! {
        <div class="welcome-screen min-h-screen bg-gradient-to-b from-blue-50 to-white p-8">
            <div class="max-w-3xl mx-auto">
                <div class="text-center mb-12">
                    <h1 class="text-5xl font-bold mb-4 text-gray-900">
                        "👋 Welcome to ez-booth!"
                    </h1>
                    <p class="text-xl text-gray-600">
                        "First time using " {state.browser_name.clone()} "?"
                    </p>
                </div>
                
                <div class="grid md:grid-cols-2 gap-6 mb-8">
                    // Import existing data card
                    <div class="bg-white p-8 rounded-xl shadow-lg border-2 border-blue-200 hover:border-blue-400 transition-all">
                        <div class="text-5xl mb-4">"📥"</div>
                        <h2 class="text-2xl font-semibold mb-4 text-gray-900">
                            "Have data in another browser?"
                        </h2>
                        <p class="text-gray-600 mb-6">
                            "If you've used ez-booth in Chrome, Firefox, or another browser, 
                             transfer your data here:"
                        </p>
                        <ol class="text-sm text-gray-700 space-y-2 mb-6 pl-5">
                            <li class="list-decimal">"Open ez-booth in your other browser"</li>
                            <li class="list-decimal">"Open the booth list and choose Export Data"</li>
                            <li class="list-decimal">"Download the JSON file"</li>
                            <li class="list-decimal">"Come back here and import it"</li>
                        </ol>
                        <button 
                            on:click=handle_import
                            class="w-full px-6 py-4 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition-colors shadow-md hover:shadow-lg"
                        >
                            "📥 Import Existing Data"
                        </button>
                    </div>
                    
                    // Start fresh card
                    <div class="bg-white p-8 rounded-xl shadow-lg border-2 border-green-200 hover:border-green-400 transition-all">
                        <div class="text-5xl mb-4">"🆕"</div>
                        <h2 class="text-2xl font-semibold mb-4 text-gray-900">
                            "First time using ez-booth?"
                        </h2>
                        <p class="text-gray-600 mb-6">
                            "Start fresh and create your first booth event. 
                             You can always import data later if needed."
                        </p>
                        <div class="text-sm text-gray-700 space-y-2 mb-6 pl-5">
                            <p class="flex items-start">
                                <span class="mr-2">"✓"</span>
                                "No setup required"
                            </p>
                            <p class="flex items-start">
                                <span class="mr-2">"✓"</span>
                                "Works completely offline"
                            </p>
                            <p class="flex items-start">
                                <span class="mr-2">"✓"</span>
                                "Data stays in your browser"
                            </p>
                        </div>
                        <button 
                            on:click=handle_new
                            class="w-full px-6 py-4 bg-green-600 text-white font-semibold rounded-lg hover:bg-green-700 transition-colors shadow-md hover:shadow-lg"
                        >
                            "🚀 Create First Booth"
                        </button>
                    </div>
                </div>
                
                <div class="bg-yellow-50 border-2 border-yellow-200 rounded-xl p-6 text-center">
                    <p class="text-sm text-yellow-900">
                        "💡 <strong>Pro Tip:</strong> "
                        "Need to use multiple browsers or devices? "
                        "Export your data to a cloud folder (Dropbox, Google Drive) for easy sync. "
                        <a href="/help/switching-browsers" class="underline font-semibold">"Learn more"</a>
                    </p>
                </div>
                
                <div class="mt-8 text-center">
                    <a 
                        href="/help/getting-started" 
                        class="text-blue-600 hover:text-blue-800 underline"
                    >
                        "📚 Read the Getting Started Guide"
                    </a>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn EmptyStatePrompt() -> impl IntoView {
    view! {
        <div class="empty-state min-h-screen flex items-center justify-center p-8">
            <div class="max-w-2xl text-center">
                <svg 
                    class="w-32 h-32 mx-auto mb-6 text-gray-300" 
                    fill="none" 
                    stroke="currentColor" 
                    viewBox="0 0 24 24"
                >
                    <path 
                        stroke-linecap="round" 
                        stroke-linejoin="round" 
                        stroke-width="2" 
                        d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
                    />
                </svg>
                
                <h2 class="text-3xl font-bold mb-4 text-gray-900">
                    "No booths yet"
                </h2>
                <p class="text-lg text-gray-600 mb-8">
                    "Get started by creating a new booth or importing existing data"
                </p>
                
                <div class="flex flex-col sm:flex-row gap-4 justify-center mb-8">
                    <a 
                        href="/booths/new"
                        class="px-8 py-4 bg-blue-600 text-white font-semibold rounded-lg hover:bg-blue-700 transition-colors shadow-md"
                    >
                        "➕ Create New Booth"
                    </a>
                    
                    <a 
                        href="/sync"
                        class="px-8 py-4 bg-white border-2 border-gray-300 font-semibold rounded-lg hover:bg-gray-50 transition-colors"
                    >
                        "📥 Import Data"
                    </a>
                </div>
                
                <div class="bg-blue-50 border-2 border-blue-200 rounded-lg p-6 inline-block">
                    <p class="text-sm text-blue-900">
                        "💡 <strong>Switching browsers?</strong> "
                        "Export your data from the other browser, then import it here. "
                        <a href="/help/switching-browsers" class="underline font-semibold">"Learn how →"</a>
                    </p>
                </div>
            </div>
        </div>
    }
}
```

**File:** `crates/frontend/src/components/navbar.rs`

```rust
use leptos::*;
use ez_vend_storage::indexeddb::database::Database;

#[component]
pub fn NavBar() -> impl IntoView {
    let booth_count = create_resource(
        || (),
        |_| async {
            Database::new()
                .await
                .ok()
                .and_then(|db| db.count_booths().await.ok())
                .unwrap_or(0)
        }
    );
    
    view! {
        <nav class="navbar bg-white shadow-md">
            <div class="container mx-auto px-4 py-3">
                {move || booth_count.get().map(|count| {
                    if count == 0 {
                        view! {
                            <div class="import-hint bg-yellow-50 border-l-4 border-yellow-400 p-3 mb-2 rounded">
                                <p class="text-sm text-yellow-800">
                                    "💡 No data yet. "
                                    <a href="/sync" class="underline font-semibold">"Import from another browser?"</a>
                                    " or "
                                    <a href="/booths/new" class="underline font-semibold">"create your first booth"</a>
                                </p>
                            </div>
                        }.into_view()
                    } else {
                        view! { <div></div> }.into_view()
                    }
                })}
                
                <div class="flex items-center justify-between">
                    <a href="/" class="text-2xl font-bold text-blue-600">"ez-vend"</a>
                    
                    <div class="flex gap-4">
                        <a href="/booths" class="nav-link">"Booths"</a>
                        <a href="/checkout" class="nav-link">"Checkout"</a>
                        <a href="/reports" class="nav-link">"Reports"</a>
                        <a href="/sync" class="nav-link">"Sync"</a>
                    </div>
                </div>
            </div>
        </nav>
    }
}
```

### 6.5 Data Synchronization Protocol (Server - Optional)

**File:** `crates/shared/src/protocol/mod.rs`

```rust
use chrono::{DateTime, Utc};
use ez_vend_core::entities::{booth::Booth, vendor::Vendor, purchase::Purchase};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncRequest {
    pub client_id: String,
    pub last_sync: Option<DateTime<Utc>>,
    pub changes: ChangeSet,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncResponse {
    pub server_time: DateTime<Utc>,
    pub changes: ChangeSet,
    pub conflicts: Vec<Conflict>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChangeSet {
    pub booths: Vec<Change<Booth>>,
    pub vendors: Vec<Change<Vendor>>,
    pub purchases: Vec<Change<Purchase>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change<T> {
    pub operation: Operation,
    pub entity: T,
    pub timestamp: DateTime<Utc>,
    pub version: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    Create,
    Update,
    Delete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Conflict {
    pub entity_type: String,
    pub entity_id: String,
    pub client_version: u64,
    pub server_version: u64,
    pub resolution: ConflictResolution,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConflictResolution {
    ServerWins,
    ClientWins,
    Manual,
}
```

---

## 7. Data Migration from ez-booth

### 7.1 Overview

**Module:** `crates/ez-vend-migration/`  
**Priority:** Phase 3 (Post-MVP)  
**Target Users:** Existing ez-booth users transitioning to ez-vend

### 7.2 Module Structure

```
crates/ez-vend-migration/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API
│   ├── parser.rs           # SQLite parsing via sql.js
│   ├── transformer.rs      # Data transformation
│   ├── validator.rs        # Data validation
│   └── error.rs            # Error types
└── tests/
    ├── fixtures/           # Sample booth.db files
    └── integration_tests.rs
```

### 7.3 Dependencies

```toml
[dependencies]
# Core
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }

# WASM/Browser
wasm-bindgen = "0.2"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "File",
    "FileReader",
    "Blob",
] }

# Domain models
ez-vend-core = { path = "../core" }
ez-vend-storage = { path = "../storage" }
```

### 7.4 Implementation

**File:** `src/lib.rs`

```rust
use wasm_bindgen::prelude::*;
use web_sys::File;

#[wasm_bindgen]
pub struct MigrationService {
    // SQL.js will be accessed via JS interop
}

#[wasm_bindgen]
impl MigrationService {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {}
    }
    
    /// Main entry point for migration
    pub async fn migrate_from_file(&self, file: File) -> Result<JsValue, JsValue> {
        let result = self.migrate_internal(file).await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        
        Ok(serde_wasm_bindgen::to_value(&result)?)
    }
}

impl MigrationService {
    async fn migrate_internal(&self, file: File) -> Result<MigrationResult> {
        // 1. Read file
        let db_bytes = read_file_bytes(&file).await?;
        
        // 2. Parse SQLite via sql.js
        let db_data = parse_sqlite_database(&db_bytes)?;
        
        // 3. Validate schema
        validate_schema(&db_data)?;
        
        // 4. Transform data
        let transformed = transform_data(db_data)?;
        
        // 5. Validate transformed data
        validate_transformed_data(&transformed)?;
        
        Ok(MigrationResult {
            booths: transformed.booths.len(),
            vendors: transformed.vendors.len(),
            transactions: transformed.transactions.len(),
            warnings: transformed.warnings,
        })
    }
}

#[derive(Serialize)]
pub struct MigrationResult {
    pub booths: usize,
    pub vendors: usize,
    pub transactions: usize,
    pub warnings: Vec<String>,
}
```

### 7.5 SQL.js Integration

**File:** `frontend/index.html`

```html
<!-- Load sql.js before WASM module -->
<script src="https://cdn.jsdelivr.net/npm/sql.js@1.8.0/dist/sql-wasm.js"></script>
<script>
  // Initialize sql.js
  window.initSqlJs = initSqlJs;
</script>
```

**JavaScript Bridge:** `src/parser.rs`

```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn initSqlJs() -> js_sys::Promise;
    
    #[wasm_bindgen]
    type Database;
    
    #[wasm_bindgen(method)]
    fn exec(this: &Database, sql: &str) -> js_sys::Array;
}

pub async fn parse_sqlite_database(bytes: &[u8]) -> Result<DatabaseData> {
    // Initialize sql.js
    let sql_js = JsFuture::from(initSqlJs()).await?;
    
    // Load database
    let db = create_database(sql_js, bytes)?;
    
    // Query tables
    let booths = query_booths(&db)?;
    let vendors = query_vendors(&db)?;
    let purchases = query_purchases(&db)?;
    let items = query_purchase_items(&db)?;
    
    Ok(DatabaseData {
        booths,
        vendors,
        purchases,
        items,
    })
}
```

### 7.6 Data Transformation

**File:** `src/transformer.rs`

```rust
pub fn transform_data(db: DatabaseData) -> Result<TransformedData> {
    let booths = transform_booths(db.booths)?;
    let vendors = transform_vendors(db.vendors, &db.items)?;
    let transactions = transform_transactions(db.purchases, db.items)?;
    
    Ok(TransformedData {
        booths,
        vendors,
        transactions,
        warnings: vec![],
    })
}

fn transform_booths(booths: Vec<SqliteBooth>) -> Result<Vec<Booth>> {
    booths.into_iter().map(|b| {
        Ok(Booth {
            id: BoothId::new(b.booth_id),
            name: b.description,
            date: NaiveDate::parse_from_str(&b.date, "%Y-%m-%d")?,
            participation_fee: Money::from_f64(b.participation_fee)?,
            sales_fee_percent: b.sales_fee,
            fee_rounding_step: Money::from_f64(b.fees_rounding_step)?,
            status: if b.closed { BoothStatus::Closed } else { BoothStatus::Open },
            closed_at: b.closed_on.map(parse_timestamp).transpose()?,
            created_at: /* derive from first transaction or date */,
            updated_at: /* derive from last transaction or closed_on */,
        })
    }).collect()
}
```

### 7.7 UI Integration

**Component:** `frontend/src/components/MigrationWizard.rs`

```rust
#[component]
pub fn MigrationWizard() -> impl IntoView {
    let (step, set_step) = create_signal(MigrationStep::Welcome);
    let (file, set_file) = create_signal(None::<File>);
    let (result, set_result) = create_signal(None::<MigrationResult>);
    
    let on_file_select = move |ev: Event| {
        let input: HtmlInputElement = event_target(&ev);
        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                set_file(Some(file));
                set_step(MigrationStep::Processing);
            }
        }
    };
    
    view! {
        <div class="migration-wizard">
            {move || match step.get() {
                MigrationStep::Welcome => view! {
                    <WelcomeStep />
                    <button on:click=move |_| set_step(MigrationStep::Upload)>
                        {t!(i18n, migration.start)}
                    </button>
                },
                MigrationStep::Upload => view! {
                    <UploadStep />
                    <input 
                        type="file" 
                        accept=".db" 
                        on:change=on_file_select 
                    />
                },
                MigrationStep::Processing => view! {
                    <ProcessingStep file=file.get() on_complete=set_result />
                },
                MigrationStep::Preview => view! {
                    <PreviewStep result=result.get() />
                    <button on:click=move |_| confirm_import()>
                        {t!(i18n, migration.confirm)}
                    </button>
                },
                MigrationStep::Complete => view! {
                    <CompleteStep result=result.get() />
                },
            }}
        </div>
    }
}
```

### 7.8 Testing

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_booth_transformation() {
        let sqlite_booth = SqliteBooth {
            booth_id: "2024-spring".to_string(),
            description: "Frühjahrsbasar".to_string(),
            date: "2024-03-15".to_string(),
            // ...
        };
        
        let booth = transform_booth(sqlite_booth).unwrap();
        assert_eq!(booth.name, "Frühjahrsbasar");
        assert_eq!(booth.date.to_string(), "2024-03-15");
    }
}
```

**Integration Tests:**
```rust
#[cfg(test)]
mod integration_tests {
    use wasm_bindgen_test::*;
    
    #[wasm_bindgen_test]
    async fn test_full_migration() {
        let test_db = include_bytes!("../tests/fixtures/sample.db");
        let file = create_test_file(test_db);
        
        let service = MigrationService::new();
        let result = service.migrate_from_file(file).await.unwrap();
        
        assert_eq!(result.booths, 2);
        assert_eq!(result.vendors, 10);
        assert_eq!(result.transactions, 50);
    }
}
```

### 7.9 Error Handling

**Error Types:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Invalid database format")]
    InvalidFormat,
    
    #[error("Schema version {0} not supported")]
    UnsupportedSchema(String),
    
    #[error("Data validation failed: {0}")]
    ValidationError(String),
    
    #[error("File read error: {0}")]
    FileReadError(String),
}
```

### 7.10 Documentation

Users will need clear guidance:
- Default database location: `~/Documents/tschuba/ez-booth/booth.db`
- Step-by-step wizard with screenshots
- What data gets migrated
- What to do if migration fails
- How to verify migration success

**See:** `05_STATUS.md` for the current migration status and `../COMPARISON_TO_ORIGINAL.md` for the current user-facing summary.

---

## 8. Build & Deployment

### 7.1 Build Scripts

**File:** `scripts/build.sh`

```bash
#!/bin/bash
set -e

echo "Building ez-vend..."

# Build frontend (WASM)
echo "Building frontend..."
cd crates/frontend
trunk build --release
cd ../..

# Optimize WASM
echo "Optimizing WASM..."
wasm-opt -Oz -o crates/frontend/dist/optimized.wasm \
    crates/frontend/dist/*.wasm

# Build server (optional)
if [ "$BUILD_SERVER" = "true" ]; then
    echo "Building server..."
    cargo build --release --bin server
fi

echo "Build complete!"
echo "Frontend: crates/frontend/dist/"
echo "Server: target/release/server"
```

### 7.2 GitHub Actions CI

**File:** `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      
      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
      
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      
      - name: Run tests
        run: cargo test --all-features
      
      - name: Run wasm tests
        run: wasm-pack test --headless --firefox crates/frontend

  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      
      - name: Check formatting
        run: cargo fmt --all -- --check
      
      - name: Run clippy
        run: cargo clippy --all-features -- -D warnings

  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      
      - name: Install Trunk
        run: wget -qO- https://github.com/trunk-rs/trunk/releases/download/v0.19.0/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf-
      
      - name: Build frontend
        run: |
          cd crates/frontend
          ../../trunk build --release
      
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: frontend-dist
          path: crates/frontend/dist/
```

---

## 9. Testing Strategy

### 8.1 Unit Tests

```rust
// Example: crates/core/src/services/charging_service.rs

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_basic_fee_calculation() {
        let config = FeeConfig {
            participation_fee: dec!(10.00),
            sales_fee_percent: dec!(5.0),
            rounding_step: dec!(0.01),
        };
        
        let result = ChargingService::calculate_fees(dec!(100.00), &config);
        
        assert_eq!(result.participation_fee, dec!(10.00));
        assert_eq!(result.sales_fee, dec!(5.00));
        assert_eq!(result.total_fees, dec!(15.00));
    }
}
```

### 8.2 WASM Browser Tests

```rust
// Example: crates/frontend/tests/integration.rs

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use ez_vend_frontend::app::App;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_app_renders() {
    // Test that app component mounts without errors
    leptos::mount_to_body(App);
}
```

---

## 10. Performance Optimization

### 9.1 WASM Optimization

```toml
# Cargo.toml
[profile.release]
opt-level = "z"           # Optimize for size
lto = true                # Link-time optimization
codegen-units = 1         # Single codegen unit
panic = "abort"           # Smaller binary
strip = true              # Remove debug symbols

[profile.release.package."*"]
opt-level = "z"
```

### 9.2 Bundle Size Optimization

```bash
# Post-build optimization
wasm-opt -Oz input.wasm -o output.wasm
wasm-strip output.wasm
gzip -9 output.wasm
```

---

---

## 11. Error Handling Implementation

### 10.1 Error Type Hierarchy

**File:** `crates/core/src/error.rs`

```rust
use thiserror::Error;
use serde::{Serialize, Deserialize};

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum AppError {
    // User-recoverable errors
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("{entity_type} not found: {id}")]
    NotFound {
        entity_type: EntityType,
        id: String,
    },
    
    #[error("Conflict: {0}")]
    Conflict(ConflictError),
    
    // System errors
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Sync error: {0}")]
    Sync(SyncError),
    
    // Fatal errors
    #[error("Data corruption detected: {0}")]
    Corruption(CorruptionError),
    
    #[error("Browser not compatible: {0}")]
    BrowserCompatibility(String),
    
    #[error("Out of memory")]
    OutOfMemory,
    
    // Fallback
    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message_key: String,
    pub suggestion_key: Option<String>,
}

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("Storage operation failed: {operation}")]
pub struct StorageError {
    pub operation: StorageOp,
    pub cause: String,
    pub recoverable: bool,
    pub recovery_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageOp {
    Read,
    Write,
    Delete,
    Transaction,
}

impl AppError {
    /// Get user-friendly message key for localization
    pub fn message_key(&self) -> String {
        match self {
            AppError::Validation(e) => e.message_key.clone(),
            AppError::NotFound { entity_type, .. } => 
                format!("errors.not_found.{}", entity_type.as_str()),
            AppError::Storage(e) if e.recoverable => 
                "errors.storage.save_failed".to_string(),
            AppError::Storage(_) => 
                "errors.storage.fatal".to_string(),
            AppError::Corruption(_) => 
                "errors.corruption.detected".to_string(),
            AppError::BrowserCompatibility(_) => 
                "errors.browser.not_supported".to_string(),
            _ => "errors.unknown".to_string(),
        }
    }
    
    /// Get recovery actions for user
    pub fn recovery_actions(&self) -> Vec<RecoveryAction> {
        match self {
            AppError::Storage(e) if e.recoverable => vec![
                RecoveryAction::Retry,
                RecoveryAction::ExportBackup,
                RecoveryAction::ContactSupport,
            ],
            AppError::Corruption(_) => vec![
                RecoveryAction::DownloadSupportBundle,
                RecoveryAction::ContactSupport,
            ],
            AppError::Validation(_) => vec![
                RecoveryAction::GoBack,
            ],
            _ => vec![
                RecoveryAction::Retry,
                RecoveryAction::ContactSupport,
            ],
        }
    }
    
    /// Check if error is transient (can retry)
    pub fn is_transient(&self) -> bool {
        matches!(self, 
            AppError::Storage(e) if e.recoverable || 
            AppError::Network(_)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryAction {
    Retry,
    GoBack,
    ExportBackup,
    DownloadSupportBundle,
    ContactSupport,
    Reload,
}
```

### 10.2 Automatic Error Recovery

**File:** `crates/storage/src/resilient.rs`

```rust
use crate::error::StorageError;
use std::time::Duration;
use gloo_timers::future::sleep;

pub struct ResilientStorage<S> {
    inner: S,
    max_retries: u8,
    initial_backoff_ms: u64,
}

impl<S> ResilientStorage<S> {
    pub fn new(storage: S) -> Self {
        Self {
            inner: storage,
            max_retries: 3,
            initial_backoff_ms: 100,
        }
    }
    
    pub async fn resilient_save<T>(
        &self,
        entity: &T,
    ) -> Result<(), StorageError>
    where
        S: Storage<T>,
    {
        let mut attempts = 0;
        let mut backoff_ms = self.initial_backoff_ms;
        
        loop {
            match self.inner.save(entity).await {
                Ok(()) => return Ok(()),
                Err(e) if attempts < self.max_retries && e.is_transient() => {
                    attempts += 1;
                    log::warn!(
                        "Save failed (attempt {}/{}): {}. Retrying...",
                        attempts,
                        self.max_retries,
                        e
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms *= 2; // Exponential backoff
                }
                Err(e) => return Err(e),
            }
        }
    }
}
```

### 10.3 Error Display Component

**File:** `crates/frontend/src/components/error_display.rs`

```rust
use leptos::*;
use ez_vend_core::error::*;
use crate::i18n::*;

#[component]
pub fn ErrorDisplay(
    error: AppError,
    on_action: impl Fn(RecoveryAction) + 'static,
) -> impl IntoView {
    let i18n = use_i18n();
    let message_key = error.message_key();
    let message = move || i18n.get(&message_key);
    let actions = error.recovery_actions();
    
    let severity_class = match error {
        AppError::Validation(_) => "warning",
        AppError::Corruption(_) | 
        AppError::BrowserCompatibility(_) => "critical",
        _ => "error",
    };
    
    view! {
        <div class=format!("error-display error-{}", severity_class)>
            <div class="error-icon">
                {match severity_class {
                    "warning" => "⚠️",
                    "critical" => "🔴",
                    _ => "❌",
                }}
            </div>
            <div class="error-content">
                <h3>{message}</h3>
                {error.details().map(|details| view! {
                    <p class="error-details">{details}</p>
                })}
            </div>
            <div class="error-actions">
                <For
                    each=move || actions.clone()
                    key=|action| format!("{:?}", action)
                    children=move |action| {
                        let action_label = action.label(&i18n);
                        let on_click = {
                            let action = action.clone();
                            move |_| on_action(action.clone())
                        };
                        view! {
                            <button 
                                class="error-action"
                                on:click=on_click
                            >
                                {action_label}
                            </button>
                        }
                    }
                />
            </div>
        </div>
    }
}
```

### 10.4 Diagnostic Report Generation

**File:** `crates/frontend/src/diagnostics.rs`

```rust
use serde::{Serialize, Deserialize};
use web_sys::window;

#[derive(Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub diagnostic_version: String,
    pub timestamp: String,
    pub app_version: String,
    pub browser: BrowserInfo,
    pub storage: StorageInfo,
    pub data_summary: DataSummary,
    pub errors: Vec<ErrorLog>,
    pub performance: PerformanceMetrics,
    pub features: FeatureFlags,
}

#[derive(Serialize, Deserialize)]
pub struct BrowserInfo {
    pub name: String,
    pub version: String,
    pub user_agent: String,
    pub language: String,
    pub platform: String,
}

pub async fn generate_diagnostic_report() -> DiagnosticReport {
    let window = window().expect("window");
    let navigator = window.navigator();
    
    DiagnosticReport {
        diagnostic_version: "1.0".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        browser: BrowserInfo {
            name: detect_browser_name(),
            version: detect_browser_version(),
            user_agent: navigator.user_agent().unwrap_or_default(),
            language: navigator.language().unwrap_or_default(),
            platform: navigator.platform().unwrap_or_default(),
        },
        storage: collect_storage_info().await,
        data_summary: collect_data_summary().await,
        errors: collect_error_log().await,
        performance: collect_performance_metrics(),
        features: detect_feature_flags(),
    }
}

pub async fn export_diagnostic_bundle() -> Result<Vec<u8>, AppError> {
    let report = generate_diagnostic_report().await;
    
    // Create ZIP bundle
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    
    // Add diagnostic report
    zip.start_file("diagnostic-report.json", FileOptions::default())?;
    zip.write_all(serde_json::to_string_pretty(&report)?.as_bytes())?;
    
    // Add error log
    zip.start_file("error-log.json", FileOptions::default())?;
    zip.write_all(serde_json::to_string_pretty(&report.errors)?.as_bytes())?;
    
    // Add README
    zip.start_file("README.txt", FileOptions::default())?;
    zip.write_all(SUPPORT_BUNDLE_README.as_bytes())?;
    
    Ok(zip.finish()?.into_inner())
}

const SUPPORT_BUNDLE_README: &str = r#"
ez-booth Support Bundle
Generated: {timestamp}
Version: {version}

This bundle contains diagnostic information to help
troubleshoot issues with ez-booth.

Privacy Notice:
NO sensitive data is included (no vendor names, sales
data, or personal information).

To get support:
1. Email this bundle to support@ez-booth.com
2. Or create a GitHub issue and attach this file
3. Include a description of your problem

Response time: Usually within 24 hours
"#;
```

### 10.5 In-App Help System

**File:** `crates/frontend/src/components/help_panel.rs`

```rust
use leptos::*;
use crate::i18n::*;

#[component]
pub fn HelpPanel(
    show: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let (search_query, set_search_query) = create_signal(String::new());
    let (selected_category, set_selected_category) = create_signal(HelpCategory::GettingStarted);
    
    let filtered_articles = create_memo(move |_| {
        let query = search_query.get().to_lowercase();
        HELP_ARTICLES
            .iter()
            .filter(|article| {
                article.matches_query(&query) &&
                (selected_category.get() == HelpCategory::All || 
                 article.category == selected_category.get())
            })
            .collect::<Vec<_>>()
    });
    
    view! {
        <div 
            class="help-panel"
            class:show=move || show.get()
        >
            <div class="help-header">
                <h2>{i18n.get("help.title")}</h2>
                <button on:click=move |_| set_show.set(false)>"×"</button>
            </div>
            
            <div class="help-search">
                <input 
                    type="text"
                    placeholder={i18n.get("help.search_placeholder")}
                    on:input=move |ev| {
                        set_search_query.set(event_target_value(&ev))
                    }
                />
            </div>
            
            <div class="help-categories">
                <For
                    each=|| HelpCategory::all()
                    key=|cat| format!("{:?}", cat)
                    children=move |category| {
                        view! {
                            <button
                                class="category-btn"
                                class:active=move || selected_category.get() == category
                                on:click=move |_| set_selected_category.set(category)
                            >
                                {category.icon()} {" "} {category.label(&i18n)}
                            </button>
                        }
                    }
                />
            </div>
            
            <div class="help-articles">
                <For
                    each=move || filtered_articles.get()
                    key=|article| article.id
                    children=move |article| {
                        view! {
                            <HelpArticle article=article />
                        }
                    }
                />
            </div>
            
            <div class="help-footer">
                <button on:click=move |_| open_support_page()>
                    {i18n.get("help.contact_support")}
                </button>
            </div>
        </div>
    }
}
```

### 10.6 Error Logging

**File:** `crates/frontend/src/error_log.rs`

```rust
use std::collections::VecDeque;
use std::sync::Mutex;

static ERROR_LOG: Mutex<VecDeque<ErrorLogEntry>> = Mutex::new(VecDeque::new());

#[derive(Clone, Serialize, Deserialize)]
pub struct ErrorLogEntry {
    pub timestamp: String,
    pub error_type: String,
    pub message: String,
    pub stack_trace: Option<String>,
    pub user_action: Option<String>,
    pub recovered: bool,
}

pub fn log_error(error: &AppError, user_action: Option<String>, recovered: bool) {
    let entry = ErrorLogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        error_type: format!("{:?}", error),
        message: error.to_string(),
        stack_trace: None, // Could use backtrace in dev mode
        user_action,
        recovered,
    };
    
    let mut log = ERROR_LOG.lock().unwrap();
    log.push_back(entry);
    
    // Keep only last 100 entries
    if log.len() > 100 {
        log.pop_front();
    }
}

pub fn get_error_log() -> Vec<ErrorLogEntry> {
    ERROR_LOG.lock().unwrap().iter().cloned().collect()
}

pub fn clear_error_log() {
    ERROR_LOG.lock().unwrap().clear();
}
```

### 10.7 Testing Error Scenarios

**File:** `crates/frontend/tests/error_handling_test.rs`

```rust
#[wasm_bindgen_test]
async fn test_storage_failure_recovery() {
    let storage = create_failing_storage(2); // Fail twice, then succeed
    let resilient = ResilientStorage::new(storage);
    
    let result = resilient.resilient_save(&test_vendor()).await;
    
    assert!(result.is_ok(), "Should recover after retries");
    assert_eq!(storage.attempt_count(), 3, "Should retry twice");
}

#[wasm_bindgen_test]
async fn test_user_error_message_localization() {
    let error = AppError::Validation(ValidationError {
        field: "vendor_id".to_string(),
        message_key: "errors.validation.vendor_id_required".to_string(),
        suggestion_key: Some("errors.validation.enter_valid_vendor_id".to_string()),
    });
    
    let i18n = I18n::new(Locale::De);
    let message = i18n.get(&error.message_key());
    
    assert!(message.contains("Verkäufer-ID"), "Should be in German");
}

#[wasm_bindgen_test]
async fn test_diagnostic_export_no_sensitive_data() {
    // Create test data with vendor IDs
    create_test_vendors().await;
    
    let diagnostic = generate_diagnostic_report().await;
    let json = serde_json::to_string(&diagnostic).unwrap();
    
    // Verify no sensitive data leaked
    assert!(!json.contains("TEST-VENDOR-ID"), "Should not contain raw vendor identifiers");
    assert!(!json.contains("test@email.com"), "Should not contain emails");
    
    // Verify required data present
    assert!(diagnostic.browser.name.len() > 0);
    assert!(diagnostic.storage.used_mb > 0.0);
}
```

### 10.8 Localized Error Messages

**File:** `locales/de.json` (error messages section)

```json
{
  "errors": {
    "validation": {
      "vendor_id_required": "Bitte geben Sie eine Verkäufer-ID ein.",
      "commission_rate_invalid": "Provision muss zwischen 0% und 100% liegen.",
      "purchase_amount_required": "Bitte geben Sie einen Betrag ein.",
      "date_invalid": "Ungültiges Datum."
    },
    "storage": {
      "save_failed": "Änderungen konnten nicht gespeichert werden.",
      "database_locked": "Datenbank ist gesperrt. Bitte versuchen Sie es erneut.",
      "quota_exceeded": "Speicherplatz voll. Bitte alte Daten archivieren.",
      "fatal": "Schwerwiegender Speicherfehler."
    },
    "not_found": {
      "vendor": "Händler nicht gefunden.",
      "booth": "Stand nicht gefunden.",
      "purchase": "Kauf nicht gefunden."
    },
    "corruption": {
      "detected": "Datenkorruption erkannt. Bitte Support kontaktieren."
    },
    "browser": {
      "not_supported": "Ihr Browser wird nicht unterstützt. Bitte verwenden Sie Chrome, Firefox, Edge oder Safari."
    },
    "recovery": {
      "retry": "Erneut versuchen",
      "go_back": "Zurück",
      "export_backup": "Backup exportieren",
      "download_support_bundle": "Support-Bundle herunterladen",
      "contact_support": "Support kontaktieren",
      "reload": "Anwendung neu laden"
    },
    "unknown": "Ein unbekannter Fehler ist aufgetreten."
  },
  "help": {
    "title": "Hilfe",
    "search_placeholder": "Hilfe durchsuchen...",
    "getting_started": "Erste Schritte",
    "data_management": "Datenverwaltung",
    "reports": "Berichte",
    "settings": "Einstellungen",
    "troubleshooting": "Problembehandlung",
    "contact_support": "Support kontaktieren"
  }
}
```

### 10.9 Implementation Priority

| Component | Phase | Effort | Dependencies |
|-----------|-------|--------|--------------|
| Error types & hierarchy | 2 | 4h | Core domain |
| Automatic retry mechanism | 2 | 4h | Storage layer |
| Error display component | 2 | 4h | Frontend UI |
| Error message localization | 3 | 4h | i18n system |
| Diagnostic report generation | 4 | 6h | Storage access |
| Support bundle export | 4 | 4h | Diagnostics |
| In-app help system | 4 | 12h | Content creation |
| Error log viewer | 5 | 4h | Error logging |
| Data repair wizard | 6 | 8h | Storage layer |
| Guided tours | 6 | 8h | UI components |

**Total Effort:** ~58 hours

**For current validation and operator-support expectations, see:** `05_STATUS.md`, `../validation/VALIDATION_WORKFLOW.md`, and `../planning/PHASE2_PROPOSAL.md`.

---

## 12. Security Implementation

### 10.1 Input Validation

```rust
use validator::Validate;

#[derive(Validate, Serialize, Deserialize)]
pub struct CreateBoothRequest {
    #[validate(length(min = 1, max = 200))]
    pub description: String,
    
    #[validate(custom = "validate_date")]
    pub date: NaiveDate,
    
    #[validate]
    pub fees: FeeConfig,
}

fn validate_date(date: &NaiveDate) -> Result<(), ValidationError> {
    let min_date = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
    let max_date = NaiveDate::from_ymd_opt(2100, 12, 31).unwrap();
    
    if date < &min_date || date > &max_date {
        return Err(ValidationError::new("date_out_of_range"));
    }
    
    Ok(())
}
```

### 11.2 Content Security Policy

```html
<!-- index.html -->
<meta http-equiv="Content-Security-Policy" 
      content="default-src 'self'; 
               script-src 'self' 'wasm-unsafe-eval'; 
               style-src 'self' 'unsafe-inline';">
```

---

**Document Status:** Ready for Implementation  
**Next Steps:**
1. Set up project structure
2. Implement Phase 1 (Core domain)
3. Begin iterative development

**Estimated Implementation Timeline:** 18 weeks to MVP
