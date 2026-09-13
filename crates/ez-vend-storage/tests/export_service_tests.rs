#![allow(clippy::arc_with_non_send_sync)]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use std::sync::Arc;

use async_trait::async_trait;
use chrono::NaiveDate;
use domain::repositories::{
    BoothRepository, BoothRunningTotals, PaginatedPurchases, PurchaseRepository, VendorRepository,
};
use domain::{
    Booth, BoothId, DomainResult, FeeConfig, Purchase, PurchaseId, PurchaseItem, Vendor, VendorId,
};
use ez_vend_storage::export::{
    compute_backup_checksum, compute_booth_checksum, generate_booth_backup_filename,
    generate_full_backup_filename, ExportError, ExportService, BACKUP_FORMAT_VERSION,
};
use ez_vend_storage::indexeddb::Database;
use ez_vend_storage::repositories::{
    IndexedDbBoothRepository, IndexedDbPurchaseRepository, IndexedDbVendorRepository,
};
use rust_decimal_macros::dec;
use wasm_bindgen_test::*;

async fn create_test_db() -> Database {
    let db_name = format!("test_export_db_{}", js_sys::Math::random());
    Database::new_with_name(&db_name)
        .await
        .expect("Failed to create test database")
}

fn create_test_booth(description: &str) -> Booth {
    Booth::new(
        description.to_string(),
        NaiveDate::from_ymd_opt(2026, 3, 29).unwrap(),
        FeeConfig {
            participation_fee: dec!(10.00),
            sales_fee_percent: dec!(15.00),
            rounding_step: dec!(0.50),
        },
    )
    .unwrap()
}

fn create_test_purchase(booth: &Booth, vendor_id: &VendorId) -> Purchase {
    Purchase::new(
        booth.id,
        vec![PurchaseItem::new(dec!(42.00), vendor_id.clone()).unwrap()],
    )
    .unwrap()
}

fn create_test_purchase_with_items(
    booth_id: BoothId,
    items: Vec<(&str, rust_decimal::Decimal)>,
) -> Purchase {
    let items = items
        .into_iter()
        .map(|(vendor_id, amount)| {
            PurchaseItem::new(amount, VendorId::new(vendor_id.to_string())).unwrap()
        })
        .collect();

    Purchase::new(booth_id, items).unwrap()
}

#[derive(Clone)]
struct MockBoothRepository {
    booths: Vec<Booth>,
}

#[async_trait(?Send)]
impl BoothRepository for MockBoothRepository {
    async fn save(&self, _booth: &Booth) -> DomainResult<()> {
        Ok(())
    }

    async fn find_by_id(&self, id: &BoothId) -> DomainResult<Option<Booth>> {
        Ok(self.booths.iter().find(|booth| booth.id == *id).cloned())
    }

    async fn find_all(&self) -> DomainResult<Vec<Booth>> {
        Ok(self.booths.clone())
    }

    async fn find_active(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .booths
            .iter()
            .filter(|booth| !booth.is_archived())
            .cloned()
            .collect())
    }

    async fn find_archived(&self) -> DomainResult<Vec<Booth>> {
        Ok(self
            .booths
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
            .iter()
            .filter(|booth| booth.description.trim() == description.trim() && booth.date == *date)
            .cloned()
            .collect())
    }

    async fn find_duplicate_groups(&self) -> DomainResult<Vec<Vec<Booth>>> {
        let mut groups: std::collections::HashMap<String, Vec<Booth>> =
            std::collections::HashMap::new();
        for booth in self.booths.iter().filter(|b| !b.is_archived()) {
            let key = format!("{}|{}", booth.description.trim(), booth.date);
            groups.entry(key).or_default().push(booth.clone());
        }
        Ok(groups.into_values().filter(|g| g.len() >= 2).collect())
    }

    async fn delete(&self, _id: &BoothId) -> DomainResult<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct MockVendorRepository {
    vendors: Vec<Vendor>,
}

#[async_trait(?Send)]
impl VendorRepository for MockVendorRepository {
    async fn save(&self, _vendor: &Vendor) -> DomainResult<()> {
        Ok(())
    }

    async fn find_by_id(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Option<Vendor>> {
        Ok(self
            .vendors
            .iter()
            .find(|vendor| vendor.booth_id == *booth_id && vendor.vendor_id == *vendor_id)
            .cloned())
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Vendor>> {
        Ok(self
            .vendors
            .iter()
            .filter(|vendor| vendor.booth_id == *booth_id)
            .cloned()
            .collect())
    }

    async fn find_all(&self) -> DomainResult<Vec<Vendor>> {
        Ok(self.vendors.clone())
    }

    async fn delete(&self, _booth_id: &BoothId, _vendor_id: &VendorId) -> DomainResult<()> {
        Ok(())
    }

    async fn delete_from_booth(
        &self,
        _booth_id: &BoothId,
        _vendor_id: &VendorId,
    ) -> DomainResult<()> {
        Ok(())
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        Ok(self
            .vendors
            .iter()
            .filter(|vendor| vendor.booth_id == *booth_id)
            .count())
    }
}

#[derive(Clone)]
struct MockPurchaseRepository {
    purchases: Vec<Purchase>,
}

#[async_trait(?Send)]
impl PurchaseRepository for MockPurchaseRepository {
    async fn save(&self, _purchase: &Purchase) -> DomainResult<()> {
        Ok(())
    }

    async fn find_by_id(&self, id: &PurchaseId) -> DomainResult<Option<Purchase>> {
        Ok(self
            .purchases
            .iter()
            .find(|purchase| purchase.id == *id)
            .cloned())
    }

    async fn find_by_booth(&self, booth_id: &BoothId) -> DomainResult<Vec<Purchase>> {
        Ok(self
            .purchases
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
        let items: Vec<Purchase> = self
            .purchases
            .iter()
            .filter(|purchase| purchase.booth_id == *booth_id)
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();

        let total_count = self
            .purchases
            .iter()
            .filter(|purchase| purchase.booth_id == *booth_id)
            .count();

        Ok(PaginatedPurchases { items, total_count })
    }

    async fn get_running_totals(&self, booth_id: &BoothId) -> DomainResult<BoothRunningTotals> {
        let matching: Vec<&Purchase> = self
            .purchases
            .iter()
            .filter(|purchase| purchase.booth_id == *booth_id)
            .collect();

        Ok(BoothRunningTotals {
            total_sales: matching
                .iter()
                .map(|purchase| purchase.total_amount())
                .sum(),
            total_items: matching.iter().map(|purchase| purchase.items.len()).sum(),
            total_checkouts: matching.len(),
        })
    }

    async fn find_by_vendor(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Vec<Purchase>> {
        Ok(self
            .purchases
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
        Ok(self.purchases.clone())
    }

    async fn delete(&self, _id: &PurchaseId) -> DomainResult<()> {
        Ok(())
    }

    async fn delete_from_booth(&self, _booth_id: &BoothId, _id: &PurchaseId) -> DomainResult<()> {
        Ok(())
    }

    async fn delete_by_booth(&self, booth_id: &BoothId) -> DomainResult<usize> {
        Ok(self
            .purchases
            .iter()
            .filter(|purchase| purchase.booth_id == *booth_id)
            .count())
    }
}

fn mock_service(
    booths: Vec<Booth>,
    vendors: Vec<Vendor>,
    purchases: Vec<Purchase>,
) -> ExportService {
    ExportService::with_app_version(
        Arc::new(MockBoothRepository { booths }),
        Arc::new(MockVendorRepository { vendors }),
        Arc::new(MockPurchaseRepository { purchases }),
        None,
        "test-version",
    )
}

#[wasm_bindgen_test]
async fn test_export_all_collects_all_records_and_serializes() {
    let db = Arc::new(create_test_db().await);
    let booth_repo: Arc<dyn BoothRepository> = Arc::new(IndexedDbBoothRepository::new(db.clone()));
    let vendor_repo: Arc<dyn VendorRepository> =
        Arc::new(IndexedDbVendorRepository::new(db.clone()));
    let purchase_repo: Arc<dyn PurchaseRepository> =
        Arc::new(IndexedDbPurchaseRepository::new(db.clone()));

    let booth = create_test_booth("Spring Market 2026");
    let vendor = Vendor::new(VendorId::new("12".to_string()), booth.id);
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let service = ExportService::with_app_version(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
        None,
        "test-version",
    );

    let backup = service.export_all().await.unwrap();
    assert_eq!(backup.version, BACKUP_FORMAT_VERSION);
    assert_eq!(backup.app_version, "test-version");
    assert_eq!(backup.booths.len(), 1);
    assert_eq!(backup.vendors.len(), 1);
    assert_eq!(backup.purchases.len(), 1);
    assert!(backup.metadata.is_empty());

    let serialized = service.serialize_full_backup(&backup).unwrap();
    assert_eq!(
        serialized.file_name,
        generate_full_backup_filename(backup.created_at)
    );
    assert!(serialized.json.contains("\n  \"booths\""));
    assert!(serialized.json.contains("Spring Market 2026"));

    let parsed: serde_json::Value = serde_json::from_str(&serialized.json).unwrap();
    let checksum = parsed
        .get("checksum")
        .and_then(|value| value.as_str())
        .unwrap();
    assert_eq!(checksum, compute_backup_checksum(&backup).unwrap());
}

#[wasm_bindgen_test]
async fn test_export_booth_filters_to_requested_booth() {
    let db = Arc::new(create_test_db().await);
    let booth_repo: Arc<dyn BoothRepository> = Arc::new(IndexedDbBoothRepository::new(db.clone()));
    let vendor_repo: Arc<dyn VendorRepository> =
        Arc::new(IndexedDbVendorRepository::new(db.clone()));
    let purchase_repo: Arc<dyn PurchaseRepository> =
        Arc::new(IndexedDbPurchaseRepository::new(db.clone()));

    let booth_a = create_test_booth("Spring Market 2026");
    let booth_b = create_test_booth("Autumn Fair 2026");
    let vendor_a = Vendor::new(VendorId::new("12".to_string()), booth_a.id);
    let vendor_b = Vendor::new(VendorId::new("99".to_string()), booth_b.id);
    let purchase_a = create_test_purchase(&booth_a, &vendor_a.vendor_id);
    let purchase_b = create_test_purchase(&booth_b, &vendor_b.vendor_id);

    booth_repo.save(&booth_a).await.unwrap();
    booth_repo.save(&booth_b).await.unwrap();
    vendor_repo.save(&vendor_a).await.unwrap();
    vendor_repo.save(&vendor_b).await.unwrap();
    purchase_repo.save(&purchase_a).await.unwrap();
    purchase_repo.save(&purchase_b).await.unwrap();

    let service = ExportService::with_app_version(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
        None,
        "test-version",
    );

    let backup = service.export_booth(&booth_a.id).await.unwrap();
    assert_eq!(backup.version, BACKUP_FORMAT_VERSION);
    assert_eq!(backup.booth.id, booth_a.id);
    assert_eq!(backup.vendors.len(), 1);
    assert_eq!(backup.vendors[0].booth_id, booth_a.id);
    assert_eq!(backup.purchases.len(), 1);
    assert_eq!(backup.purchases[0].booth_id, booth_a.id);

    let serialized = service.serialize_booth_backup(&backup).unwrap();
    assert_eq!(
        serialized.file_name,
        generate_booth_backup_filename(
            &backup.booth.id,
            &backup.booth.description,
            backup.created_at
        )
    );
    assert!(serialized.json.contains("Spring Market 2026"));
    assert!(!serialized.json.contains("Autumn Fair 2026"));

    let parsed: serde_json::Value = serde_json::from_str(&serialized.json).unwrap();
    let checksum = parsed
        .get("checksum")
        .and_then(|value| value.as_str())
        .unwrap();
    assert_eq!(checksum, compute_booth_checksum(&backup).unwrap());
}

#[wasm_bindgen_test]
async fn test_export_booth_returns_not_found_for_missing_booth() {
    let db = Arc::new(create_test_db().await);
    let booth_repo: Arc<dyn BoothRepository> = Arc::new(IndexedDbBoothRepository::new(db.clone()));
    let vendor_repo: Arc<dyn VendorRepository> =
        Arc::new(IndexedDbVendorRepository::new(db.clone()));
    let purchase_repo: Arc<dyn PurchaseRepository> =
        Arc::new(IndexedDbPurchaseRepository::new(db.clone()));

    let service = ExportService::with_app_version(
        booth_repo,
        vendor_repo,
        purchase_repo,
        None,
        "test-version",
    );

    let missing_booth_id = booth_a_like_id();
    let error = service.export_booth(&missing_booth_id).await.unwrap_err();

    match error {
        ExportError::BoothNotFound(found_id) => assert_eq!(found_id, missing_booth_id),
        other => panic!("Expected BoothNotFound error, got {other:?}"),
    }
}

#[wasm_bindgen_test]
async fn test_export_all_skips_vendors_with_missing_booths() {
    let booth = create_test_booth("Valid Booth");
    let valid_vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let orphaned_vendor = Vendor::new(VendorId::new("99".to_string()), BoothId::new());

    let service = mock_service(
        vec![booth],
        vec![valid_vendor.clone(), orphaned_vendor],
        Vec::new(),
    );

    let backup = service.export_all().await.unwrap();

    assert_eq!(backup.booths.len(), 1);
    assert_eq!(backup.vendors.len(), 1);
    assert_eq!(backup.vendors[0], valid_vendor);
}

#[wasm_bindgen_test]
async fn test_export_all_skips_purchases_with_missing_booths() {
    let booth = create_test_booth("Valid Booth");
    let vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let valid_purchase = create_test_purchase(&booth, &vendor.vendor_id);
    let orphaned_purchase = Purchase::new(
        BoothId::new(),
        vec![PurchaseItem::new(dec!(10.00), vendor.vendor_id.clone()).unwrap()],
    )
    .unwrap();

    let service = mock_service(
        vec![booth],
        vec![vendor],
        vec![valid_purchase.clone(), orphaned_purchase],
    );

    let backup = service.export_all().await.unwrap();

    assert_eq!(backup.purchases.len(), 1);
    assert_eq!(backup.purchases[0], valid_purchase);
}

#[wasm_bindgen_test]
async fn test_export_all_skips_purchases_with_missing_vendors() {
    let booth = create_test_booth("Valid Booth");
    let valid_vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let valid_purchase = create_test_purchase(&booth, &valid_vendor.vendor_id);
    let orphaned_purchase = Purchase::new(
        booth.id,
        vec![PurchaseItem::new(dec!(10.00), VendorId::new("999".to_string())).unwrap()],
    )
    .unwrap();

    let service = mock_service(
        vec![booth],
        vec![valid_vendor],
        vec![valid_purchase.clone(), orphaned_purchase],
    );

    let backup = service.export_all().await.unwrap();

    assert_eq!(backup.purchases.len(), 1);
    assert_eq!(backup.purchases[0], valid_purchase);
}

#[wasm_bindgen_test]
async fn test_export_all_removes_only_orphaned_items_from_mixed_purchase() {
    let booth = create_test_booth("Valid Booth");
    let valid_vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let purchase = create_test_purchase_with_items(
        booth.id,
        vec![("1", dec!(10.00)), ("999", dec!(20.00)), ("1", dec!(30.00))],
    );

    let service = mock_service(vec![booth], vec![valid_vendor], vec![purchase]);

    let backup = service.export_all().await.unwrap();

    assert_eq!(backup.purchases.len(), 1);
    assert_eq!(backup.purchases[0].items.len(), 2);
    assert!(backup.purchases[0]
        .items
        .iter()
        .all(|item| item.vendor_id.as_str() == "1"));
}

#[wasm_bindgen_test]
async fn test_export_all_skips_purchase_when_all_items_are_orphaned() {
    let booth = create_test_booth("Valid Booth");
    let valid_vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let purchase = create_test_purchase_with_items(booth.id, vec![("999", dec!(10.00))]);

    let service = mock_service(vec![booth], vec![valid_vendor], vec![purchase]);

    let backup = service.export_all().await.unwrap();

    assert!(backup.purchases.is_empty());
}

#[wasm_bindgen_test]
async fn test_export_booth_skips_purchases_with_missing_vendors() {
    let booth = create_test_booth("Booth Export");
    let valid_vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let valid_purchase = create_test_purchase(&booth, &valid_vendor.vendor_id);
    let orphaned_purchase = Purchase::new(
        booth.id,
        vec![PurchaseItem::new(dec!(10.00), VendorId::new("404".to_string())).unwrap()],
    )
    .unwrap();

    let service = mock_service(
        vec![booth.clone()],
        vec![valid_vendor.clone()],
        vec![valid_purchase.clone(), orphaned_purchase],
    );

    let backup = service.export_booth(&booth.id).await.unwrap();

    assert_eq!(backup.booth.id, booth.id);
    assert_eq!(backup.vendors, vec![valid_vendor]);
    assert_eq!(backup.purchases, vec![valid_purchase]);
}

#[wasm_bindgen_test]
async fn test_export_booth_removes_only_orphaned_items_from_mixed_purchase() {
    let booth = create_test_booth("Booth Export");
    let valid_vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let purchase = create_test_purchase_with_items(
        booth.id,
        vec![("1", dec!(5.00)), ("404", dec!(7.50)), ("1", dec!(8.25))],
    );

    let service = mock_service(vec![booth.clone()], vec![valid_vendor], vec![purchase]);

    let backup = service.export_booth(&booth.id).await.unwrap();

    assert_eq!(backup.purchases.len(), 1);
    assert_eq!(backup.purchases[0].items.len(), 2);
    assert!(backup.purchases[0]
        .items
        .iter()
        .all(|item| item.vendor_id.as_str() == "1"));
}

#[wasm_bindgen_test]
async fn test_export_booth_skips_purchase_when_all_items_are_orphaned() {
    let booth = create_test_booth("Booth Export");
    let valid_vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let purchase = create_test_purchase_with_items(booth.id, vec![("404", dec!(7.50))]);

    let service = mock_service(vec![booth.clone()], vec![valid_vendor], vec![purchase]);

    let backup = service.export_booth(&booth.id).await.unwrap();

    assert!(backup.purchases.is_empty());
}

#[wasm_bindgen_test]
async fn test_export_all_preserves_clean_data() {
    let booth = create_test_booth("Clean Booth");
    let vendor = Vendor::new(VendorId::new("1".to_string()), booth.id);
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    let service = mock_service(
        vec![booth.clone()],
        vec![vendor.clone()],
        vec![purchase.clone()],
    );

    let backup = service.export_all().await.unwrap();

    assert_eq!(backup.booths, vec![booth]);
    assert_eq!(backup.vendors, vec![vendor]);
    assert_eq!(backup.purchases, vec![purchase]);
}

fn booth_a_like_id() -> domain::BoothId {
    domain::BoothId::new()
}
