//! Repository trait definitions for data persistence
//!
//! These traits define the interface between the domain logic and storage implementations.
//! They use async_trait with ?Send to support WASM environments.

use crate::error::DomainResult;
use crate::models::{Booth, BoothId, Purchase, PurchaseId, Vendor, VendorId};
use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;

/// Repository trait for booth persistence operations
#[async_trait(?Send)]
pub trait BoothRepository {
    /// Save a booth (insert or update)
    async fn save(&self, booth: &Booth) -> DomainResult<()>;

    /// Find a booth by its ID
    async fn find_by_id(&self, id: &BoothId) -> DomainResult<Option<Booth>>;

    /// Find all booths
    async fn find_all(&self) -> DomainResult<Vec<Booth>>;

    /// Find all active booths
    async fn find_active(&self) -> DomainResult<Vec<Booth>>;

    /// Find all archived booths
    async fn find_archived(&self) -> DomainResult<Vec<Booth>>;

    /// Find a booth by description and date (returns first match)
    async fn find_by_description_and_date(
        &self,
        description: &str,
        date: &NaiveDate,
    ) -> DomainResult<Option<Booth>>;

    /// Find all booths matching description and date (active and archived)
    async fn find_all_by_description_and_date(
        &self,
        description: &str,
        date: &NaiveDate,
    ) -> DomainResult<Vec<Booth>>;

    /// Find groups of active booths sharing the same (description, date) key but different UUIDs
    async fn find_duplicate_groups(&self) -> DomainResult<Vec<Vec<Booth>>>;

    /// Delete a booth by ID
    async fn delete(&self, id: &BoothId) -> DomainResult<()>;
}

/// Repository trait for vendor persistence operations
#[async_trait(?Send)]
pub trait VendorRepository {
    /// Save a vendor (insert or update)
    async fn save(&self, vendor: &Vendor) -> DomainResult<()>;

    /// Find a vendor by booth ID and vendor ID
    async fn find_by_id(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Option<Vendor>>;

    /// Find all vendors for a specific booth
    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Vendor>>;

    /// Find all vendors across all booths
    async fn find_all(&self) -> DomainResult<Vec<Vendor>>;

    /// Delete all vendors for a booth and return the deleted count
    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize>;

    /// Delete a vendor
    async fn delete(&self, booth_id: &BoothId, vendor_id: &VendorId) -> DomainResult<()>;

    /// Delete a vendor by booth ID and vendor ID (for compound key stores)
    async fn delete_from_booth(&self, booth_id: &BoothId, vendor_id: &VendorId)
        -> DomainResult<()>;
}

/// Paginated query result for purchases
#[derive(Debug, Clone)]
pub struct PaginatedPurchases {
    /// The purchases for the current page
    pub items: Vec<Purchase>,
    /// Total number of purchases matching the query
    pub total_count: usize,
}

/// Running totals for a booth
#[derive(Debug, Clone)]
pub struct BoothRunningTotals {
    /// Total sales amount across all purchases
    pub total_sales: Decimal,
    /// Total number of items across all purchases
    pub total_items: usize,
    /// Total number of checkouts (purchases)
    pub total_checkouts: usize,
}

/// Repository trait for purchase persistence operations
#[async_trait(?Send)]
pub trait PurchaseRepository {
    /// Save a purchase (insert or update)
    async fn save(&self, purchase: &Purchase) -> DomainResult<()>;

    /// Find a purchase by its ID
    async fn find_by_id(&self, id: &PurchaseId) -> DomainResult<Option<Purchase>>;

    /// Find all purchases for a specific booth
    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Purchase>>;

    /// Find purchases for a specific booth with pagination
    /// Returns paginated results sorted by timestamp descending (newest first)
    async fn find_by_booth_paginated(
        &self,
        booth_id: &BoothId,
        offset: usize,
        limit: usize,
    ) -> DomainResult<PaginatedPurchases>;

    /// Get running totals for a specific booth
    async fn get_running_totals(&self, booth_id: &BoothId) -> DomainResult<BoothRunningTotals>;

    /// Find all purchases for a specific vendor in a booth
    async fn find_by_vendor(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Vec<Purchase>>;

    /// Find all purchases
    async fn find_all(&self) -> DomainResult<Vec<Purchase>>;

    /// Delete all purchases for a booth and return the deleted count
    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize>;

    /// Delete a purchase by ID
    async fn delete(&self, id: &PurchaseId) -> DomainResult<()>;

    /// Delete a purchase by booth ID and purchase ID (for compound key stores)
    async fn delete_from_booth(&self, booth_id: &BoothId, id: &PurchaseId) -> DomainResult<()>;
}
