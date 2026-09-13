wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use domain::repositories::{
    BoothRepository, BoothRunningTotals, PaginatedPurchases, PurchaseRepository, VendorRepository,
};
use domain::{
    Booth, BoothId, DomainResult, FeeConfig, Purchase, PurchaseId, PurchaseItem, Vendor, VendorId,
};
use ez_vend_storage::migration::{MigrationIssueStrategy, ValidationIssue};
use ez_vend_storage::{MigrationError, MigrationService, SqliteParser};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

const REAL_DB: &[u8] = include_bytes!("migration/fixtures/booth.db");

fn test_booth(description: &str) -> Booth {
    Booth::new(
        description.to_string(),
        NaiveDate::from_ymd_opt(2026, 4, 6).unwrap(),
        FeeConfig {
            participation_fee: dec!(10.00),
            sales_fee_percent: dec!(15.00),
            rounding_step: dec!(0.50),
        },
    )
    .unwrap()
}

fn test_vendor(booth_id: BoothId, vendor_id: &str) -> Vendor {
    Vendor::new(VendorId::new(vendor_id.to_string()), booth_id)
}

fn test_purchase(booth_id: BoothId, purchase_id: PurchaseId, item_id: domain::ItemId) -> Purchase {
    Purchase {
        id: purchase_id,
        booth_id,
        items: vec![PurchaseItem {
            id: item_id,
            amount: dec!(25.00),
            vendor_id: VendorId::from("1"),
        }],
        timestamp: Utc::now(),
        note: None,
    }
}

#[derive(Clone, Default)]
struct MockBoothRepository {
    booths: Arc<Mutex<Vec<Booth>>>,
}

#[async_trait(?Send)]
impl BoothRepository for MockBoothRepository {
    async fn save(&self, booth: &Booth) -> DomainResult<()> {
        let mut booths = self.booths.lock().unwrap();
        if let Some(existing) = booths.iter_mut().find(|existing| existing.id == booth.id) {
            *existing = booth.clone();
        } else {
            booths.push(booth.clone());
        }
        Ok(())
    }

    async fn find_by_id(&self, id: &BoothId) -> DomainResult<Option<Booth>> {
        Ok(self
            .booths
            .lock()
            .unwrap()
            .iter()
            .find(|booth| booth.id == *id)
            .cloned())
    }

    async fn find_all(&self) -> DomainResult<Vec<Booth>> {
        Ok(self.booths.lock().unwrap().clone())
    }

    async fn find_active(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .booths
            .lock()
            .unwrap()
            .iter()
            .filter(|booth| !booth.is_archived())
            .cloned()
            .collect())
    }

    async fn find_archived(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .booths
            .lock()
            .unwrap()
            .iter()
            .filter(|booth| booth.is_archived())
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
            .iter()
            .find(|booth| booth.description == description && booth.date == *date)
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
            .iter()
            .filter(|booth| booth.description.trim() == description.trim() && booth.date == *date)
            .cloned()
            .collect())
    }

    async fn find_duplicate_groups(&self) -> DomainResult<Vec<Vec<Booth>>> {
        let booths = self.booths.lock().unwrap();
        let mut groups: std::collections::HashMap<String, Vec<Booth>> =
            std::collections::HashMap::new();
        for booth in booths.iter().filter(|b| !b.is_archived()) {
            let key = format!("{}|{}", booth.description.trim(), booth.date);
            groups.entry(key).or_default().push(booth.clone());
        }
        Ok(groups.into_values().filter(|g| g.len() >= 2).collect())
    }

    async fn delete(&self, id: &BoothId) -> DomainResult<()> {
        self.booths.lock().unwrap().retain(|booth| booth.id != *id);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MockVendorRepository {
    vendors: Arc<Mutex<Vec<Vendor>>>,
}

#[async_trait(?Send)]
impl VendorRepository for MockVendorRepository {
    async fn save(&self, vendor: &Vendor) -> DomainResult<()> {
        let mut vendors = self.vendors.lock().unwrap();
        if let Some(existing) = vendors.iter_mut().find(|existing| {
            existing.booth_id == vendor.booth_id && existing.vendor_id == vendor.vendor_id
        }) {
            *existing = vendor.clone();
        } else {
            vendors.push(vendor.clone());
        }
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
            .iter()
            .find(|vendor| vendor.booth_id == *booth_id && vendor.vendor_id == *vendor_id)
            .cloned())
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Vendor>> {
        Ok(self
            .vendors
            .lock()
            .unwrap()
            .iter()
            .filter(|vendor| vendor.booth_id == *booth_id)
            .cloned()
            .collect())
    }

    async fn find_all(&self) -> DomainResult<Vec<Vendor>> {
        Ok(self.vendors.lock().unwrap().clone())
    }

    async fn delete(&self, booth_id: &BoothId, vendor_id: &VendorId) -> DomainResult<()> {
        self.vendors
            .lock()
            .unwrap()
            .retain(|vendor| !(vendor.booth_id == *booth_id && vendor.vendor_id == *vendor_id));
        Ok(())
    }

    async fn delete_from_booth(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<()> {
        self.delete(booth_id, vendor_id).await
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        let mut vendors = self.vendors.lock().unwrap();
        let original_len = vendors.len();
        vendors.retain(|vendor| vendor.booth_id != *booth_id);
        Ok(original_len - vendors.len())
    }
}

#[derive(Clone, Default)]
struct MockPurchaseRepository {
    purchases: Arc<Mutex<Vec<Purchase>>>,
}

#[async_trait(?Send)]
impl PurchaseRepository for MockPurchaseRepository {
    async fn save(&self, purchase: &Purchase) -> DomainResult<()> {
        let mut purchases = self.purchases.lock().unwrap();
        if let Some(existing) = purchases
            .iter_mut()
            .find(|existing| existing.id == purchase.id)
        {
            *existing = purchase.clone();
        } else {
            purchases.push(purchase.clone());
        }
        Ok(())
    }

    async fn find_by_id(&self, id: &PurchaseId) -> DomainResult<Option<Purchase>> {
        Ok(self
            .purchases
            .lock()
            .unwrap()
            .iter()
            .find(|purchase| purchase.id == *id)
            .cloned())
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Purchase>> {
        Ok(self
            .purchases
            .lock()
            .unwrap()
            .iter()
            .filter(|purchase| purchase.booth_id == *booth_id)
            .cloned()
            .collect())
    }

    async fn find_by_booth_paginated(
        &self,
        booth_id: &BoothId,
        offset: usize,
        limit: usize,
    ) -> DomainResult<PaginatedPurchases> {
        let all = self.find_by_booth(booth_id).await?;
        let total_count = all.len();
        let items = all.into_iter().skip(offset).take(limit).collect();
        Ok(PaginatedPurchases { items, total_count })
    }

    async fn get_running_totals(&self, booth_id: &BoothId) -> DomainResult<BoothRunningTotals> {
        let purchases = self.find_by_booth(booth_id).await?;
        Ok(BoothRunningTotals {
            total_sales: purchases.iter().map(Purchase::total_amount).sum(),
            total_items: purchases.iter().map(|purchase| purchase.items.len()).sum(),
            total_checkouts: purchases.len(),
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
            .iter()
            .filter(|purchase| {
                purchase.booth_id == *booth_id
                    && purchase
                        .items
                        .iter()
                        .any(|item| item.vendor_id == *vendor_id)
            })
            .cloned()
            .collect())
    }

    async fn find_all(&self) -> DomainResult<Vec<Purchase>> {
        Ok(self.purchases.lock().unwrap().clone())
    }

    async fn delete(&self, id: &PurchaseId) -> DomainResult<()> {
        self.purchases
            .lock()
            .unwrap()
            .retain(|purchase| purchase.id != *id);
        Ok(())
    }

    async fn delete_from_booth(&self, booth_id: &BoothId, id: &PurchaseId) -> DomainResult<()> {
        self.purchases
            .lock()
            .unwrap()
            .retain(|purchase| !(purchase.booth_id == *booth_id && purchase.id == *id));
        Ok(())
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        let mut purchases = self.purchases.lock().unwrap();
        let original_len = purchases.len();
        purchases.retain(|purchase| purchase.booth_id != *booth_id);
        Ok(original_len - purchases.len())
    }
}

#[test]
fn opens_real_sqlite_fixture() {
    let parser = SqliteParser::open_database(REAL_DB.to_vec()).unwrap();
    let booths = parser.parse_booths().unwrap();

    assert_eq!(booths.len(), 4);
}

#[test]
fn parses_real_fixture_counts() {
    let parser = SqliteParser::open_database(REAL_DB.to_vec()).unwrap();

    assert_eq!(parser.parse_booths().unwrap().len(), 4);
    assert_eq!(parser.parse_vendors().unwrap().len(), 106);
    assert_eq!(parser.parse_purchases().unwrap().len(), 45);
}

#[test]
fn parses_purchases_with_items() {
    let parser = SqliteParser::open_database(REAL_DB.to_vec()).unwrap();
    let purchases = parser.parse_purchases().unwrap();

    assert!(purchases.iter().all(|purchase| !purchase.items.is_empty()));
}

#[test]
fn rejects_invalid_database_bytes() {
    let result = SqliteParser::open_database(vec![0_u8; 64]);

    assert!(result.is_err());
}

#[test]
fn parse_and_validate_real_fixture_returns_expected_summary() {
    let service = MigrationService::new(
        Arc::new(MockBoothRepository::default()),
        Arc::new(MockVendorRepository::default()),
        Arc::new(MockPurchaseRepository::default()),
    );

    let summary = service.parse_and_validate(REAL_DB.to_vec()).unwrap();

    assert_eq!(summary.validation.booth_count, 4);
    assert_eq!(summary.validation.vendor_count, 106);
    assert_eq!(summary.validation.purchase_count, 45);
    assert_eq!(summary.booths.len(), 4);
    assert_eq!(summary.vendors.len(), 106);
    assert_eq!(summary.purchases.len(), 45);
    assert_eq!(summary.validation.issues.len(), 9);
    assert!(summary
        .validation
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::VendorMissingBooth { .. })));
    assert!(summary
        .validation
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::PurchaseMissingBooth { .. })));
}

#[test]
fn prepare_import_cancel_rejects_validation_issues() {
    let service = MigrationService::new(
        Arc::new(MockBoothRepository::default()),
        Arc::new(MockVendorRepository::default()),
        Arc::new(MockPurchaseRepository::default()),
    );

    let summary = service.parse_and_validate(REAL_DB.to_vec()).unwrap();
    let result = service.prepare_import(summary, MigrationIssueStrategy::Cancel);

    assert!(matches!(result, Err(MigrationError::ReplaceFailed(_))));
}

#[test]
fn prepare_import_skip_invalid_filters_orphaned_records() {
    let service = MigrationService::new(
        Arc::new(MockBoothRepository::default()),
        Arc::new(MockVendorRepository::default()),
        Arc::new(MockPurchaseRepository::default()),
    );

    let summary = service.parse_and_validate(REAL_DB.to_vec()).unwrap();
    let prepared = service
        .prepare_import(summary, MigrationIssueStrategy::SkipInvalid)
        .unwrap();

    assert_eq!(prepared.booths.len(), 4);
    assert_eq!(prepared.vendors.len(), 101);
    assert_eq!(prepared.purchases.len(), 41);
    assert!(prepared.validation.issues.is_empty());
}

#[test]
fn replace_all_removes_existing_records_before_saving_new_ones() {
    let booth_repo = Arc::new(MockBoothRepository::default());
    let vendor_repo = Arc::new(MockVendorRepository::default());
    let purchase_repo = Arc::new(MockPurchaseRepository::default());
    let service = MigrationService::new(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
    );

    let old_booth = test_booth("Old Booth");
    let old_vendor = test_vendor(old_booth.id, "1");
    let old_purchase = test_purchase(old_booth.id, PurchaseId::new(), domain::ItemId::new());
    futures::executor::block_on(async {
        booth_repo.save(&old_booth).await.unwrap();
        vendor_repo.save(&old_vendor).await.unwrap();
        purchase_repo.save(&old_purchase).await.unwrap();
    });

    let new_booth = test_booth("New Booth");
    let new_vendor = test_vendor(new_booth.id, "9");
    let new_purchase = Purchase {
        id: PurchaseId::new(),
        booth_id: new_booth.id,
        items: vec![PurchaseItem {
            id: domain::ItemId::new(),
            amount: dec!(42.00),
            vendor_id: VendorId::from("9"),
        }],
        timestamp: Utc::now(),
        note: None,
    };

    let result = futures::executor::block_on(service.replace_all(
        vec![new_booth.clone()],
        vec![new_vendor.clone()],
        vec![new_purchase.clone()],
    ))
    .unwrap();

    assert_eq!(result.booths_migrated, 1);
    assert_eq!(result.vendors_migrated, 1);
    assert_eq!(result.purchases_migrated, 1);

    futures::executor::block_on(async {
        assert_eq!(booth_repo.find_all().await.unwrap(), vec![new_booth]);
        assert_eq!(vendor_repo.find_all().await.unwrap(), vec![new_vendor]);
        assert_eq!(purchase_repo.find_all().await.unwrap(), vec![new_purchase]);
    });
}

#[test]
fn parse_and_validate_reports_purchase_total_mismatch() {
    let mut bytes = REAL_DB.to_vec();
    let parser = SqliteParser::open_database(bytes.clone()).unwrap();
    let mut purchases = parser.parse_purchases().unwrap();
    purchases[0].total_value += dec!(1.00);

    let result = ez_vend_storage::migration::validate_dataset(
        &ez_vend_storage::migration::map_booths(&parser.parse_booths().unwrap()).unwrap(),
        &ez_vend_storage::migration::map_vendors(&parser.parse_vendors().unwrap()).unwrap(),
        &ez_vend_storage::migration::map_purchases(&purchases).unwrap(),
        &purchases,
    );

    assert!(result
        .issues
        .iter()
        .any(|issue| matches!(issue, ValidationIssue::PurchaseTotalMismatch { .. })));

    bytes.clear();
}

#[test]
fn parse_and_validate_reports_invalid_item_uuid() {
    let parser = SqliteParser::open_database(REAL_DB.to_vec()).unwrap();
    let booths = parser.parse_booths().unwrap();
    let vendors = parser.parse_vendors().unwrap();
    let mut purchases = parser.parse_purchases().unwrap();
    purchases[0].items[0].item_id = "not-a-uuid".to_string();

    let booths = ez_vend_storage::migration::map_booths(&booths).unwrap();
    let vendors = ez_vend_storage::migration::map_vendors(&vendors).unwrap();
    let result = ez_vend_storage::migration::map_purchases(&purchases);

    assert_eq!(booths.len(), 4);
    assert_eq!(vendors.len(), 106);
    assert!(matches!(
        result,
        Err(MigrationError::InvalidUuid {
            field: "item_id",
            ..
        })
    ));
}

#[test]
fn decimal_fixture_totals_remain_two_decimal_places() {
    let parser = SqliteParser::open_database(REAL_DB.to_vec()).unwrap();
    let purchases = parser.parse_purchases().unwrap();

    assert!(purchases
        .iter()
        .all(|purchase| purchase.total_value.scale() <= 2));
    assert!(purchases
        .iter()
        .flat_map(|purchase| purchase.items.iter())
        .all(|item| item.price.scale() <= 2));
    let total_sales: Decimal = purchases.iter().map(|purchase| purchase.total_value).sum();
    assert!(total_sales >= Decimal::ZERO);
}
