use async_trait::async_trait;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::models::{Booth, BoothId, Purchase, PurchaseId, Vendor, VendorId};
use crate::repositories::{BoothRepository, PurchaseRepository, VendorRepository};
use crate::{BoothRunningTotals, DomainResult, PaginatedPurchases};

// ─── MockBoothRepository ──────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MockBoothRepository {
    booths: Arc<Mutex<HashMap<BoothId, Booth>>>,
}

impl MockBoothRepository {
    pub fn new() -> Self {
        Self {
            booths: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add(&self, booth: Booth) {
        self.booths.lock().unwrap().insert(booth.id, booth);
    }
}

#[async_trait(?Send)]
impl BoothRepository for MockBoothRepository {
    async fn save(&self, booth: &Booth) -> DomainResult<()> {
        self.booths.lock().unwrap().insert(booth.id, booth.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &BoothId) -> DomainResult<Option<Booth>> {
        Ok(self.booths.lock().unwrap().get(id).cloned())
    }

    async fn find_all(&self) -> DomainResult<Vec<Booth>> {
        Ok(self.booths.lock().unwrap().values().cloned().collect())
    }

    async fn find_active(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .booths
            .lock()
            .unwrap()
            .values()
            .filter(|b| !b.is_archived())
            .cloned()
            .collect())
    }

    async fn find_archived(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .booths
            .lock()
            .unwrap()
            .values()
            .filter(|b| b.is_archived())
            .cloned()
            .collect())
    }

    async fn find_by_description_and_date(
        &self,
        description: &str,
        date: &NaiveDate,
    ) -> DomainResult<Option<Booth>> {
        Ok(self
            .booths
            .lock()
            .unwrap()
            .values()
            .find(|b| b.date == *date && b.description.trim() == description.trim())
            .cloned())
    }

    async fn find_all_by_description_and_date(
        &self,
        description: &str,
        date: &NaiveDate,
    ) -> DomainResult<Vec<Booth>> {
        Ok(self
            .booths
            .lock()
            .unwrap()
            .values()
            .filter(|b| b.date == *date && b.description.trim() == description.trim())
            .cloned()
            .collect())
    }

    async fn find_duplicate_groups(&self) -> DomainResult<Vec<Vec<Booth>>> {
        let booths = self.booths.lock().unwrap();
        let mut groups: HashMap<String, Vec<Booth>> = HashMap::new();
        for booth in booths.values().filter(|b| !b.is_archived()) {
            let key = format!("{}|{}", booth.description.trim(), booth.date);
            groups.entry(key).or_default().push(booth.clone());
        }
        Ok(groups.into_values().filter(|g| g.len() >= 2).collect())
    }

    async fn delete(&self, id: &BoothId) -> DomainResult<()> {
        self.booths.lock().unwrap().remove(id);
        Ok(())
    }
}

// ─── MockVendorRepository ─────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MockVendorRepository {
    vendors: Arc<Mutex<HashMap<(BoothId, VendorId), Vendor>>>,
}

impl MockVendorRepository {
    pub fn new() -> Self {
        Self {
            vendors: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add(&self, vendor: Vendor) {
        self.vendors
            .lock()
            .unwrap()
            .insert((vendor.booth_id, vendor.vendor_id.clone()), vendor);
    }
}

#[async_trait(?Send)]
impl VendorRepository for MockVendorRepository {
    async fn save(&self, vendor: &Vendor) -> DomainResult<()> {
        self.vendors
            .lock()
            .unwrap()
            .insert((vendor.booth_id, vendor.vendor_id.clone()), vendor.clone());
        Ok(())
    }

    async fn find_by_id(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Option<Vendor>> {
        Ok(self
            .vendors
            .lock()
            .unwrap()
            .get(&(*booth_id, vendor_id.clone()))
            .cloned())
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Vendor>> {
        Ok(self
            .vendors
            .lock()
            .unwrap()
            .iter()
            .filter(|((bid, _), _)| bid == booth_id)
            .map(|(_, v)| v.clone())
            .collect())
    }

    async fn find_all(&self) -> DomainResult<Vec<Vendor>> {
        Ok(self.vendors.lock().unwrap().values().cloned().collect())
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        let mut vendors = self.vendors.lock().unwrap();
        let before = vendors.len();
        vendors.retain(|(bid, _), _| bid != booth_id);
        Ok(before.saturating_sub(vendors.len()))
    }

    async fn delete(&self, booth_id: &BoothId, vendor_id: &VendorId) -> DomainResult<()> {
        self.vendors
            .lock()
            .unwrap()
            .remove(&(*booth_id, vendor_id.clone()));
        Ok(())
    }

    async fn delete_from_booth(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<()> {
        self.vendors
            .lock()
            .unwrap()
            .remove(&(*booth_id, vendor_id.clone()));
        Ok(())
    }
}

// ─── MockPurchaseRepository ───────────────────────────────────────────────────

#[derive(Clone)]
pub struct MockPurchaseRepository {
    purchases: Arc<Mutex<HashMap<PurchaseId, Purchase>>>,
}

impl MockPurchaseRepository {
    pub fn new() -> Self {
        Self {
            purchases: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add(&self, purchase: Purchase) {
        self.purchases.lock().unwrap().insert(purchase.id, purchase);
    }
}

#[async_trait(?Send)]
impl PurchaseRepository for MockPurchaseRepository {
    async fn save(&self, purchase: &Purchase) -> DomainResult<()> {
        self.purchases
            .lock()
            .unwrap()
            .insert(purchase.id, purchase.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: &PurchaseId) -> DomainResult<Option<Purchase>> {
        Ok(self.purchases.lock().unwrap().get(id).cloned())
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Purchase>> {
        Ok(self
            .purchases
            .lock()
            .unwrap()
            .values()
            .filter(|p| &p.booth_id == booth_id)
            .cloned()
            .collect())
    }

    async fn find_by_booth_paginated(
        &self,
        booth_id: &BoothId,
        offset: usize,
        limit: usize,
    ) -> DomainResult<PaginatedPurchases> {
        let mut purchases = self.find_by_booth(booth_id).await?;
        purchases.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        let total_count = purchases.len();
        let items = purchases.into_iter().skip(offset).take(limit).collect();
        Ok(PaginatedPurchases { items, total_count })
    }

    async fn get_running_totals(&self, booth_id: &BoothId) -> DomainResult<BoothRunningTotals> {
        let purchases = self.find_by_booth(booth_id).await?;
        let total_sales: Decimal = purchases.iter().map(|p| p.total_amount()).sum();
        let total_items: usize = purchases.iter().map(|p| p.items.len()).sum();
        let total_checkouts = purchases.len();
        Ok(BoothRunningTotals {
            total_sales,
            total_items,
            total_checkouts,
        })
    }

    async fn find_by_vendor(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Vec<Purchase>> {
        Ok(self
            .purchases
            .lock()
            .unwrap()
            .values()
            .filter(|p| {
                &p.booth_id == booth_id && p.items.iter().any(|item| &item.vendor_id == vendor_id)
            })
            .cloned()
            .collect())
    }

    async fn find_all(&self) -> DomainResult<Vec<Purchase>> {
        Ok(self.purchases.lock().unwrap().values().cloned().collect())
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        let mut purchases = self.purchases.lock().unwrap();
        let before = purchases.len();
        purchases.retain(|_, p| &p.booth_id != booth_id);
        Ok(before.saturating_sub(purchases.len()))
    }

    async fn delete(&self, id: &PurchaseId) -> DomainResult<()> {
        self.purchases.lock().unwrap().remove(id);
        Ok(())
    }

    async fn delete_from_booth(&self, _booth_id: &BoothId, id: &PurchaseId) -> DomainResult<()> {
        self.purchases.lock().unwrap().remove(id);
        Ok(())
    }
}
