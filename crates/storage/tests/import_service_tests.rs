#![allow(clippy::arc_with_non_send_sync)]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use chrono::{Duration, NaiveDate, Utc};
use domain::repositories::{BoothRepository, PurchaseRepository, VendorRepository};
use domain::{ArchivedBoothSummary, Booth, FeeConfig, Purchase, PurchaseItem, Vendor, VendorId};
use ez_vend_storage::archive::ArchiveService;
use ez_vend_storage::export::{
    BackupData, BoothBackupData, ConflictStrategy, ImportService, ImportSummary,
    BACKUP_FORMAT_VERSION,
};
use ez_vend_storage::indexeddb::Database;
use ez_vend_storage::repositories::{
    IndexedDbBoothRepository, IndexedDbPurchaseRepository, IndexedDbVendorRepository,
};
use rust_decimal_macros::dec;
use std::sync::Arc;
use wasm_bindgen_test::*;

async fn create_test_db() -> Database {
    let db_name = format!("test_import_service_db_{}", js_sys::Math::random());
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

fn create_test_vendor(booth: &Booth, vendor_id: &str) -> Vendor {
    Vendor::new(VendorId::new(vendor_id.to_string()), booth.id)
}

fn create_test_purchase(booth: &Booth, vendor_id: &VendorId) -> Purchase {
    Purchase::new(
        booth.id,
        vec![PurchaseItem::new(dec!(42.00), vendor_id.clone()).unwrap()],
    )
    .unwrap()
}

fn full_backup(booth: Booth, vendor: Vendor, purchase: Purchase) -> BackupData {
    BackupData {
        version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now(),
        app_version: "test-version".to_string(),
        checksum: None,
        device_info: None,
        booths: vec![booth],
        vendors: vec![vendor],
        purchases: vec![purchase],
        metadata: Default::default(),
    }
}

fn booth_backup(booth: Booth, vendor: Vendor, purchase: Purchase) -> BoothBackupData {
    BoothBackupData {
        version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now(),
        app_version: "test-version".to_string(),
        checksum: None,
        device_info: None,
        booth,
        vendors: vec![vendor],
        purchases: vec![purchase],
    }
}

fn booth_backup_with_records(
    booth: Booth,
    vendors: Vec<Vendor>,
    purchases: Vec<Purchase>,
) -> BoothBackupData {
    BoothBackupData {
        version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now(),
        app_version: "test-version".to_string(),
        checksum: None,
        device_info: None,
        booth,
        vendors,
        purchases,
    }
}

fn full_backup_with_records(
    booths: Vec<Booth>,
    vendors: Vec<Vendor>,
    purchases: Vec<Purchase>,
) -> BackupData {
    BackupData {
        version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now(),
        app_version: "test-version".to_string(),
        checksum: None,
        device_info: None,
        booths,
        vendors,
        purchases,
        metadata: Default::default(),
    }
}

async fn build_service() -> (
    Arc<dyn BoothRepository>,
    Arc<dyn VendorRepository>,
    Arc<dyn PurchaseRepository>,
    ImportService,
) {
    let db = Arc::new(create_test_db().await);
    let booth_repo: Arc<dyn BoothRepository> = Arc::new(IndexedDbBoothRepository::new(db.clone()));
    let vendor_repo: Arc<dyn VendorRepository> =
        Arc::new(IndexedDbVendorRepository::new(db.clone()));
    let purchase_repo: Arc<dyn PurchaseRepository> =
        Arc::new(IndexedDbPurchaseRepository::new(db.clone()));
    let service = ImportService::new(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
    );
    (booth_repo, vendor_repo, purchase_repo, service)
}

async fn build_service_with_archive() -> (
    Arc<dyn BoothRepository>,
    Arc<dyn VendorRepository>,
    Arc<dyn PurchaseRepository>,
    ImportService,
) {
    let db = Arc::new(create_test_db().await);
    let booth_repo: Arc<dyn BoothRepository> = Arc::new(IndexedDbBoothRepository::new(db.clone()));
    let vendor_repo: Arc<dyn VendorRepository> =
        Arc::new(IndexedDbVendorRepository::new(db.clone()));
    let purchase_repo: Arc<dyn PurchaseRepository> =
        Arc::new(IndexedDbPurchaseRepository::new(db.clone()));
    let archive_service = Arc::new(ArchiveService::new(db.clone()));
    let service = ImportService::with_archive_service(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
        Some(archive_service),
    );
    (booth_repo, vendor_repo, purchase_repo, service)
}

/// Create a booth with the same name+date as `base` but a new random UUID (cross-device scenario)
fn cross_device_booth(base: &Booth) -> Booth {
    let mut b = Booth::new(base.description.clone(), base.date, base.fees.clone()).unwrap();
    b.updated_at = base.updated_at;
    b
}

fn empty_archived_summary() -> ArchivedBoothSummary {
    ArchivedBoothSummary {
        total_revenue: rust_decimal::Decimal::ZERO,
        total_booth_revenue: rust_decimal::Decimal::ZERO,
        vendor_count: 0,
        purchase_count: 0,
        item_count: 0,
        first_purchase_at: None,
        last_purchase_at: None,
        vendor_summaries: vec![],
    }
}

#[wasm_bindgen_test]
async fn import_all_saves_new_records() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Spring Market 2026");
    let vendor = create_test_vendor(&booth, "12");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    let summary = service
        .import_all(
            full_backup(booth.clone(), vendor.clone(), purchase.clone()),
            ConflictStrategy::Skip,
        )
        .await
        .unwrap();

    assert_eq!(summary.booths_imported, 1);
    assert_eq!(summary.vendors_imported, 1);
    assert_eq!(summary.purchases_imported, 1);
    assert_eq!(summary.conflicts_resolved, 0);
    assert!(summary.skipped_records.is_empty());

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_some());
    assert!(vendor_repo
        .find_by_id(&booth.id, &vendor.vendor_id)
        .await
        .unwrap()
        .is_some());
    assert!(purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .is_some());
}

#[wasm_bindgen_test]
async fn import_skip_leaves_existing_records_untouched() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Spring Market 2026");
    let vendor = create_test_vendor(&booth, "12");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let mut incoming_booth = booth.clone();
    incoming_booth.description = "Updated Description".to_string();
    let incoming_vendor = create_test_vendor(&booth, "12");
    let mut incoming_purchase = purchase.clone();
    incoming_purchase.note = Some("Imported note".to_string());

    let summary = service
        .import_all(
            full_backup(incoming_booth, incoming_vendor, incoming_purchase),
            ConflictStrategy::Skip,
        )
        .await
        .unwrap();

    assert_eq!(summary.skipped_records.len(), 3);
    assert_eq!(summary.conflicts_resolved, 0);

    let saved_booth = booth_repo.find_by_id(&booth.id).await.unwrap().unwrap();
    assert_eq!(saved_booth.description, "Spring Market 2026");
}

#[wasm_bindgen_test]
async fn import_replace_overwrites_existing_records() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Spring Market 2026");
    let vendor = create_test_vendor(&booth, "12");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let mut incoming_booth = booth.clone();
    incoming_booth.description = "Updated Description".to_string();
    let incoming_vendor = create_test_vendor(&incoming_booth, "12");
    let mut incoming_purchase = purchase.clone();
    incoming_purchase.note = Some("Imported note".to_string());

    let summary = service
        .import_all(
            full_backup(
                incoming_booth.clone(),
                incoming_vendor.clone(),
                incoming_purchase.clone(),
            ),
            ConflictStrategy::Replace,
        )
        .await
        .unwrap();

    assert_eq!(summary.conflicts_resolved, 3);
    assert_eq!(summary.booths_imported, 1);
    assert_eq!(summary.vendors_imported, 1);
    assert_eq!(summary.purchases_imported, 1);

    let saved_booth = booth_repo.find_by_id(&booth.id).await.unwrap().unwrap();
    assert_eq!(saved_booth.description, "Updated Description");

    let saved_vendor = vendor_repo
        .find_by_id(&booth.id, &incoming_vendor.vendor_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_vendor.vendor_id, incoming_vendor.vendor_id);

    let saved_purchase = purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_purchase.note.as_deref(), Some("Imported note"));
}

#[wasm_bindgen_test]
async fn import_merge_prefers_newer_booth_and_purchase_data() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let mut booth = create_test_booth("Spring Market 2026");
    let vendor = create_test_vendor(&booth, "12");
    let mut purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth.updated_at = Utc::now() - Duration::days(1);
    purchase.timestamp = Utc::now() - Duration::days(1);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let mut incoming_booth = booth.clone();
    incoming_booth.description = "Merged Description".to_string();
    incoming_booth.updated_at = Utc::now();

    let incoming_vendor = create_test_vendor(&incoming_booth, "12");

    let mut incoming_purchase = purchase.clone();
    incoming_purchase.note = Some("Merged note".to_string());
    incoming_purchase.timestamp = Utc::now();

    let summary = service
        .import_all(
            full_backup(
                incoming_booth.clone(),
                incoming_vendor.clone(),
                incoming_purchase.clone(),
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    assert_eq!(summary.conflicts_resolved, 3);

    let saved_booth = booth_repo.find_by_id(&booth.id).await.unwrap().unwrap();
    assert_eq!(saved_booth.description, "Merged Description");

    let saved_vendor = vendor_repo
        .find_by_id(&booth.id, &incoming_vendor.vendor_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_vendor.vendor_id, incoming_vendor.vendor_id);

    let saved_purchase = purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_purchase.note.as_deref(), Some("Merged note"));
}

#[wasm_bindgen_test]
async fn import_merge_keeps_earliest_vendor_created_at() {
    let (booth_repo, vendor_repo, _purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Vendor Merge 2026");
    let mut existing_vendor = create_test_vendor(&booth, "12");
    existing_vendor.created_at = Utc::now() - Duration::minutes(5);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&existing_vendor).await.unwrap();

    let mut incoming_vendor = create_test_vendor(&booth, "12");
    incoming_vendor.created_at = Utc::now();

    service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test-version".to_string(),
                checksum: None,
                device_info: None,
                booth: booth.clone(),
                vendors: vec![incoming_vendor],
                purchases: vec![],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let saved_vendor = vendor_repo
        .find_by_id(&booth.id, &existing_vendor.vendor_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_vendor.created_at, existing_vendor.created_at);
}

#[wasm_bindgen_test]
async fn import_merge_with_equal_booth_timestamps_keeps_existing_record() {
    let (booth_repo, _vendor_repo, _purchase_repo, service) = build_service().await;
    let mut booth = create_test_booth("Equal Timestamp Booth");
    let shared_timestamp = Utc::now();
    booth.updated_at = shared_timestamp;

    booth_repo.save(&booth).await.unwrap();

    let mut incoming_booth = booth.clone();
    incoming_booth.description = "A deterministic description".to_string();
    incoming_booth.updated_at = shared_timestamp;

    service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test-version".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming_booth,
                vendors: vec![],
                purchases: vec![],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let saved_booth = booth_repo.find_by_id(&booth.id).await.unwrap().unwrap();
    assert_eq!(saved_booth.description, "Equal Timestamp Booth");
}

#[wasm_bindgen_test]
async fn import_merge_with_equal_purchase_timestamps_keeps_existing_record() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Equal Timestamp Purchase");
    let vendor = create_test_vendor(&booth, "12");
    let mut existing_purchase = create_test_purchase(&booth, &vendor.vendor_id);
    let shared_timestamp = Utc::now();
    existing_purchase.timestamp = shared_timestamp;

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&existing_purchase).await.unwrap();

    let mut incoming_purchase = existing_purchase.clone();
    incoming_purchase.note = Some("Imported deterministic note".to_string());
    incoming_purchase.timestamp = shared_timestamp;

    service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test-version".to_string(),
                checksum: None,
                device_info: None,
                booth: booth.clone(),
                vendors: vec![vendor.clone()],
                purchases: vec![incoming_purchase],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let saved_purchase = purchase_repo
        .find_by_id(&existing_purchase.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_purchase.note.as_deref(), None);
}

#[wasm_bindgen_test]
async fn repeated_multi_device_import_of_shared_history_does_not_duplicate_records() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Shared History 2026");
    let vendor = create_test_vendor(&booth, "12");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    let device_a_backup = BoothBackupData {
        version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now(),
        app_version: "device-a".to_string(),
        checksum: None,
        device_info: None,
        booth: booth.clone(),
        vendors: vec![vendor.clone()],
        purchases: vec![purchase.clone()],
    };

    let mut device_b_purchase = create_test_purchase(&booth, &vendor.vendor_id);
    device_b_purchase.note = Some("Device B checkout".to_string());
    let device_b_backup = BoothBackupData {
        version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now(),
        app_version: "device-b".to_string(),
        checksum: None,
        device_info: None,
        booth: booth.clone(),
        vendors: vec![vendor.clone()],
        purchases: vec![purchase.clone(), device_b_purchase.clone()],
    };

    let summary_a = service
        .import_booth_backup(device_a_backup, ConflictStrategy::Merge)
        .await
        .unwrap();
    let summary_b = service
        .import_booth_backup(device_b_backup, ConflictStrategy::Merge)
        .await
        .unwrap();

    assert_eq!(summary_a.booths_imported, 1);
    assert_eq!(summary_a.vendors_imported, 1);
    assert_eq!(summary_a.purchases_imported, 1);
    assert_eq!(summary_b.purchases_imported, 2);

    let saved_booth = booth_repo.find_by_id(&booth.id).await.unwrap();
    assert!(saved_booth.is_some());

    let vendors = vendor_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(vendors.len(), 1);

    let purchases = purchase_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(purchases.len(), 2);
    assert!(purchases
        .iter()
        .any(|candidate| candidate.id == purchase.id));
    assert!(purchases
        .iter()
        .any(|candidate| candidate.id == device_b_purchase.id));
}

#[wasm_bindgen_test]
async fn parallel_three_device_booth_merge_preserves_all_unique_purchases() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Parallel Merge 2026");
    let vendor = create_test_vendor(&booth, "12");

    let mut purchase_a = create_test_purchase(&booth, &vendor.vendor_id);
    purchase_a.note = Some("device-a".to_string());
    purchase_a.timestamp = Utc::now() - Duration::minutes(3);

    let mut purchase_b = create_test_purchase(&booth, &vendor.vendor_id);
    purchase_b.note = Some("device-b".to_string());
    purchase_b.timestamp = Utc::now() - Duration::minutes(2);

    let mut purchase_c = create_test_purchase(&booth, &vendor.vendor_id);
    purchase_c.note = Some("device-c".to_string());
    purchase_c.timestamp = Utc::now() - Duration::minutes(1);

    service
        .import_booth_backup(
            booth_backup_with_records(
                booth.clone(),
                vec![vendor.clone()],
                vec![purchase_a.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    service
        .import_booth_backup(
            booth_backup_with_records(
                booth.clone(),
                vec![vendor.clone()],
                vec![purchase_a.clone(), purchase_b.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    service
        .import_booth_backup(
            booth_backup_with_records(
                booth.clone(),
                vec![vendor.clone()],
                vec![purchase_a.clone(), purchase_c.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_some());

    let vendors = vendor_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(vendors.len(), 1);

    let purchases = purchase_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(purchases.len(), 3);
    assert!(purchases
        .iter()
        .any(|purchase| purchase.id == purchase_a.id));
    assert!(purchases
        .iter()
        .any(|purchase| purchase.id == purchase_b.id));
    assert!(purchases
        .iter()
        .any(|purchase| purchase.id == purchase_c.id));
}

#[wasm_bindgen_test]
async fn round_trip_import_merges_new_records_without_readding_shared_history() {
    let (_booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Round Trip 2026");
    let vendor = create_test_vendor(&booth, "12");

    let mut original_purchase = create_test_purchase(&booth, &vendor.vendor_id);
    original_purchase.note = Some("original".to_string());

    let device_a_backup = booth_backup_with_records(
        booth.clone(),
        vec![vendor.clone()],
        vec![original_purchase.clone()],
    );

    service
        .import_booth_backup(device_a_backup, ConflictStrategy::Merge)
        .await
        .unwrap();

    let mut new_purchase = create_test_purchase(&booth, &vendor.vendor_id);
    new_purchase.note = Some("device-b-new".to_string());

    let device_b_backup = booth_backup_with_records(
        booth.clone(),
        vec![vendor.clone()],
        vec![original_purchase.clone(), new_purchase.clone()],
    );

    service
        .import_booth_backup(device_b_backup, ConflictStrategy::Merge)
        .await
        .unwrap();

    let vendors = vendor_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(vendors.len(), 1);

    let purchases = purchase_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(purchases.len(), 2);
    assert!(purchases
        .iter()
        .any(|purchase| purchase.id == original_purchase.id));
    assert!(purchases
        .iter()
        .any(|purchase| purchase.id == new_purchase.id));
}

#[wasm_bindgen_test]
async fn multi_device_booth_merge_prefers_latest_booth_update_even_when_imported_out_of_order() {
    let (booth_repo, _vendor_repo, _purchase_repo, service) = build_service().await;
    let mut base_booth = create_test_booth("Config Merge 2026");
    let t1 = Utc::now() - Duration::hours(3);
    let t2 = Utc::now() - Duration::hours(2);
    let t3 = Utc::now() - Duration::hours(1);

    base_booth.updated_at = t1;

    let mut booth_a = base_booth.clone();
    booth_a.description = "Config from A".to_string();
    booth_a.updated_at = t1;

    let mut booth_b = base_booth.clone();
    booth_b.description = "Config from B".to_string();
    booth_b.updated_at = t2;
    booth_b.fees.participation_fee = dec!(15.00);

    let mut booth_c = base_booth.clone();
    booth_c.description = "Config from C".to_string();
    booth_c.updated_at = t3;
    booth_c.fees.participation_fee = dec!(20.00);

    service
        .import_booth_backup(
            booth_backup_with_records(booth_b.clone(), vec![], vec![]),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    service
        .import_booth_backup(
            booth_backup_with_records(booth_a.clone(), vec![], vec![]),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    service
        .import_booth_backup(
            booth_backup_with_records(booth_c.clone(), vec![], vec![]),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let saved_booth = booth_repo
        .find_by_id(&base_booth.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_booth.description, "Config from C");
    assert_eq!(saved_booth.fees.participation_fee, dec!(20.00));
}

#[wasm_bindgen_test]
async fn import_all_from_multiple_devices_preserves_other_booths_while_merging_shared_booth() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let shared_booth = create_test_booth("Shared Booth 2026");
    let other_booth = create_test_booth("Other Booth 2026");

    let shared_vendor = create_test_vendor(&shared_booth, "12");
    let other_vendor = create_test_vendor(&other_booth, "55");

    let shared_purchase = create_test_purchase(&shared_booth, &shared_vendor.vendor_id);
    let other_purchase = create_test_purchase(&other_booth, &other_vendor.vendor_id);

    service
        .import_all(
            full_backup_with_records(
                vec![shared_booth.clone()],
                vec![shared_vendor.clone()],
                vec![shared_purchase.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    service
        .import_all(
            full_backup_with_records(
                vec![shared_booth.clone(), other_booth.clone()],
                vec![shared_vendor.clone(), other_vendor.clone()],
                vec![shared_purchase.clone(), other_purchase.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let booths = booth_repo.find_all().await.unwrap();
    assert_eq!(booths.len(), 2);

    let shared_vendors = vendor_repo.find_by_booth(&shared_booth.id).await.unwrap();
    let other_vendors = vendor_repo.find_by_booth(&other_booth.id).await.unwrap();
    assert_eq!(shared_vendors.len(), 1);
    assert_eq!(other_vendors.len(), 1);

    let shared_purchases = purchase_repo.find_by_booth(&shared_booth.id).await.unwrap();
    let other_purchases = purchase_repo.find_by_booth(&other_booth.id).await.unwrap();
    assert_eq!(shared_purchases.len(), 1);
    assert_eq!(other_purchases.len(), 1);
}

#[wasm_bindgen_test]
async fn same_purchase_id_from_multiple_devices_keeps_newer_purchase_data() {
    let (_booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Purchase Conflict 2026");
    let vendor = create_test_vendor(&booth, "12");

    let mut older_purchase = create_test_purchase(&booth, &vendor.vendor_id);
    older_purchase.note = Some("older note".to_string());
    older_purchase.timestamp = Utc::now() - Duration::minutes(5);

    let mut newer_purchase = older_purchase.clone();
    newer_purchase.note = Some("newer note".to_string());
    newer_purchase.timestamp = Utc::now();

    service
        .import_booth_backup(
            booth_backup_with_records(
                booth.clone(),
                vec![vendor.clone()],
                vec![older_purchase.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    service
        .import_booth_backup(
            booth_backup_with_records(
                booth.clone(),
                vec![vendor.clone()],
                vec![newer_purchase.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let purchases = purchase_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(purchases.len(), 1);
    assert_eq!(purchases[0].note.as_deref(), Some("newer note"));

    let vendors = vendor_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(vendors.len(), 1);
}

#[wasm_bindgen_test]
async fn mixed_full_and_booth_imports_preserve_shared_history_without_duplication() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let shared_booth = create_test_booth("Mixed Shared Booth 2026");
    let extra_booth = create_test_booth("Mixed Extra Booth 2026");

    let shared_vendor = create_test_vendor(&shared_booth, "12");
    let extra_vendor = create_test_vendor(&extra_booth, "55");

    let shared_purchase = create_test_purchase(&shared_booth, &shared_vendor.vendor_id);
    let mut second_shared_purchase = create_test_purchase(&shared_booth, &shared_vendor.vendor_id);
    second_shared_purchase.note = Some("second shared".to_string());
    let extra_purchase = create_test_purchase(&extra_booth, &extra_vendor.vendor_id);

    service
        .import_all(
            full_backup_with_records(
                vec![shared_booth.clone(), extra_booth.clone()],
                vec![shared_vendor.clone(), extra_vendor.clone()],
                vec![shared_purchase.clone(), extra_purchase.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    service
        .import_booth_backup(
            booth_backup_with_records(
                shared_booth.clone(),
                vec![shared_vendor.clone()],
                vec![shared_purchase.clone(), second_shared_purchase.clone()],
            ),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let booths = booth_repo.find_all().await.unwrap();
    assert_eq!(booths.len(), 2);

    let shared_purchases = purchase_repo.find_by_booth(&shared_booth.id).await.unwrap();
    let extra_purchases = purchase_repo.find_by_booth(&extra_booth.id).await.unwrap();
    assert_eq!(shared_purchases.len(), 2);
    assert_eq!(extra_purchases.len(), 1);
    assert!(shared_purchases
        .iter()
        .any(|purchase| purchase.id == shared_purchase.id));
    assert!(shared_purchases
        .iter()
        .any(|purchase| purchase.id == second_shared_purchase.id));

    let shared_vendors = vendor_repo.find_by_booth(&shared_booth.id).await.unwrap();
    let extra_vendors = vendor_repo.find_by_booth(&extra_booth.id).await.unwrap();
    assert_eq!(shared_vendors.len(), 1);
    assert_eq!(extra_vendors.len(), 1);
}

#[wasm_bindgen_test]
async fn large_multi_device_merge_keeps_expected_record_counts() {
    let (_booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Stress Merge 2026");

    let shared_vendor = create_test_vendor(&booth, "1");
    let mut vendors = vec![shared_vendor.clone()];
    for vendor_number in 2..=6 {
        vendors.push(create_test_vendor(&booth, &vendor_number.to_string()));
    }

    let mut device_a_purchases = Vec::new();
    let mut device_b_purchases = Vec::new();
    let mut device_c_purchases = Vec::new();

    for (index, vendor) in vendors.iter().enumerate() {
        let mut purchase_a = create_test_purchase(&booth, &vendor.vendor_id);
        purchase_a.note = Some(format!("device-a-{index}"));
        device_a_purchases.push(purchase_a);

        let mut purchase_b = create_test_purchase(&booth, &vendor.vendor_id);
        purchase_b.note = Some(format!("device-b-{index}"));
        device_b_purchases.push(purchase_b);

        let mut purchase_c = create_test_purchase(&booth, &vendor.vendor_id);
        purchase_c.note = Some(format!("device-c-{index}"));
        device_c_purchases.push(purchase_c);
    }

    let mut device_b_vendors = vendors.clone();
    device_b_vendors[0] = create_test_vendor(&booth, "1");

    let mut device_c_booth = booth.clone();
    device_c_booth.updated_at = Utc::now() + Duration::minutes(1);
    device_c_booth.description = "Stress Merge Final".to_string();

    service
        .import_booth_backup(
            booth_backup_with_records(booth.clone(), vendors.clone(), device_a_purchases),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    service
        .import_booth_backup(
            booth_backup_with_records(booth.clone(), device_b_vendors, device_b_purchases),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    service
        .import_booth_backup(
            booth_backup_with_records(device_c_booth.clone(), vendors.clone(), device_c_purchases),
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    let saved_vendor = vendor_repo
        .find_by_id(&booth.id, &VendorId::new("1".to_string()))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_vendor.vendor_id.as_str(), "1");

    let purchases = purchase_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(purchases.len(), 18);
}

#[wasm_bindgen_test]
async fn import_booth_backup_saves_only_that_scope() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Scoped Event 2026");
    let vendor = create_test_vendor(&booth, "12");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    let summary = service
        .import_booth_backup(
            booth_backup(booth.clone(), vendor.clone(), purchase.clone()),
            ConflictStrategy::Skip,
        )
        .await
        .unwrap();

    assert_eq!(
        summary,
        ImportSummary {
            booths_imported: 1,
            vendors_imported: 1,
            purchases_imported: 1,
            conflicts_resolved: 0,
            skipped_records: vec![],
        }
    );

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_some());
    assert!(vendor_repo
        .find_by_id(&booth.id, &vendor.vendor_id)
        .await
        .unwrap()
        .is_some());
    assert!(purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .is_some());
}

#[wasm_bindgen_test]
async fn import_after_delete_recreates_booth_with_skip_strategy() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Spring Market 2026");
    let vendor = create_test_vendor(&booth, "12");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let backup = booth_backup(booth.clone(), vendor.clone(), purchase.clone());

    purchase_repo
        .delete_from_booth(&booth.id, &purchase.id)
        .await
        .unwrap();
    vendor_repo
        .delete(&booth.id, &vendor.vendor_id)
        .await
        .unwrap();
    booth_repo.delete(&booth.id).await.unwrap();

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_none());
    assert!(vendor_repo
        .find_by_id(&booth.id, &vendor.vendor_id)
        .await
        .unwrap()
        .is_none());
    assert!(purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .is_none());

    let summary = service
        .import_booth_backup(backup, ConflictStrategy::Skip)
        .await
        .unwrap();

    assert_eq!(summary.booths_imported, 1);
    assert_eq!(summary.vendors_imported, 1);
    assert_eq!(summary.purchases_imported, 1);
    assert_eq!(summary.skipped_records.len(), 0);
    assert_eq!(summary.conflicts_resolved, 0);

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_some());
    assert!(vendor_repo
        .find_by_id(&booth.id, &vendor.vendor_id)
        .await
        .unwrap()
        .is_some());
    assert!(purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .is_some());
}

#[wasm_bindgen_test]
async fn import_after_delete_recreates_booth_with_replace_strategy() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Summer Festival 2026");
    let vendor = create_test_vendor(&booth, "42");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let backup = booth_backup(booth.clone(), vendor.clone(), purchase.clone());

    purchase_repo
        .delete_from_booth(&booth.id, &purchase.id)
        .await
        .unwrap();
    vendor_repo
        .delete(&booth.id, &vendor.vendor_id)
        .await
        .unwrap();
    booth_repo.delete(&booth.id).await.unwrap();

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_none());

    let summary = service
        .import_booth_backup(backup, ConflictStrategy::Replace)
        .await
        .unwrap();

    assert_eq!(summary.booths_imported, 1);
    assert_eq!(summary.vendors_imported, 1);
    assert_eq!(summary.purchases_imported, 1);
    assert_eq!(summary.conflicts_resolved, 0);

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_some());
    assert!(vendor_repo
        .find_by_id(&booth.id, &vendor.vendor_id)
        .await
        .unwrap()
        .is_some());
    assert!(purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .is_some());
}

#[wasm_bindgen_test]
async fn import_after_delete_recreates_booth_with_merge_strategy() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Autumn Market 2026");
    let vendor = create_test_vendor(&booth, "99");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let backup = booth_backup(booth.clone(), vendor.clone(), purchase.clone());

    purchase_repo
        .delete_from_booth(&booth.id, &purchase.id)
        .await
        .unwrap();
    vendor_repo
        .delete(&booth.id, &vendor.vendor_id)
        .await
        .unwrap();
    booth_repo.delete(&booth.id).await.unwrap();

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_none());

    let summary = service
        .import_booth_backup(backup, ConflictStrategy::Merge)
        .await
        .unwrap();

    assert_eq!(summary.booths_imported, 1);
    assert_eq!(summary.vendors_imported, 1);
    assert_eq!(summary.purchases_imported, 1);
    assert_eq!(summary.conflicts_resolved, 0);

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_some());
    assert!(vendor_repo
        .find_by_id(&booth.id, &vendor.vendor_id)
        .await
        .unwrap()
        .is_some());
    assert!(purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .is_some());
}

#[wasm_bindgen_test]
async fn import_full_backup_after_delete_recreates_records() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let booth = create_test_booth("Winter Festival 2026");
    let vendor = create_test_vendor(&booth, "77");
    let purchase = create_test_purchase(&booth, &vendor.vendor_id);

    booth_repo.save(&booth).await.unwrap();
    vendor_repo.save(&vendor).await.unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let backup = full_backup(booth.clone(), vendor.clone(), purchase.clone());

    purchase_repo
        .delete_from_booth(&booth.id, &purchase.id)
        .await
        .unwrap();
    vendor_repo
        .delete(&booth.id, &vendor.vendor_id)
        .await
        .unwrap();
    booth_repo.delete(&booth.id).await.unwrap();

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_none());

    let summary = service
        .import_all(backup, ConflictStrategy::Skip)
        .await
        .unwrap();

    assert_eq!(summary.booths_imported, 1);
    assert_eq!(summary.vendors_imported, 1);
    assert_eq!(summary.purchases_imported, 1);
    assert_eq!(summary.skipped_records.len(), 0);

    assert!(booth_repo.find_by_id(&booth.id).await.unwrap().is_some());
    assert!(vendor_repo
        .find_by_id(&booth.id, &vendor.vendor_id)
        .await
        .unwrap()
        .is_some());
    assert!(purchase_repo
        .find_by_id(&purchase.id)
        .await
        .unwrap()
        .is_some());
}

// ─── Cross-device (ByNameAndDate) tests ─────────────────────────────────────

// Case 1a: UUID match active → ConflictStrategy applied, canonical ID preserved
#[wasm_bindgen_test]
async fn case_1a_uuid_match_active_applies_conflict_strategy() {
    let (booth_repo, _, _, service) = build_service().await;
    let local_booth = create_test_booth("Spring Market 2026");
    booth_repo.save(&local_booth).await.unwrap();
    let mut incoming = local_booth.clone();
    incoming.description = "Updated Spring Market".to_string();
    incoming.updated_at = Utc::now();
    let summary = service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming,
                vendors: vec![],
                purchases: vec![],
            },
            ConflictStrategy::Replace,
        )
        .await
        .unwrap();
    assert_eq!(summary.conflicts_resolved, 1);
    let saved = booth_repo
        .find_by_id(&local_booth.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.id, local_booth.id);
    assert_eq!(saved.description, "Updated Spring Market");
}

// Case 2a: No UUID match; 1 active booth with same name+date → merge under existing ID
#[wasm_bindgen_test]
async fn case_2a_cross_device_merge_uses_existing_canonical_id() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let local_booth = create_test_booth("Autumn Fair 2026");
    booth_repo.save(&local_booth).await.unwrap();
    let incoming_booth = cross_device_booth(&local_booth);
    assert_ne!(incoming_booth.id, local_booth.id);
    let incoming_vendor = create_test_vendor(&incoming_booth, "V1");
    let incoming_purchase = create_test_purchase(&incoming_booth, &incoming_vendor.vendor_id);
    let summary = service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "device-b".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming_booth.clone(),
                vendors: vec![incoming_vendor],
                purchases: vec![incoming_purchase],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    let all_booths = booth_repo.find_all().await.unwrap();
    assert_eq!(all_booths.len(), 1);
    assert_eq!(all_booths[0].id, local_booth.id);
    assert_eq!(
        vendor_repo
            .find_by_booth(&local_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        purchase_repo
            .find_by_booth(&local_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(booth_repo
        .find_by_id(&incoming_booth.id)
        .await
        .unwrap()
        .is_none());
    assert_eq!(summary.vendors_imported, 1);
    assert_eq!(summary.purchases_imported, 1);
}

// Case 2a with Skip: metadata not updated, vendors/purchases still imported under canonical ID
#[wasm_bindgen_test]
async fn case_2a_skip_does_not_update_metadata_but_imports_subordinate_records() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service().await;
    let local_booth = create_test_booth("Winter Market 2026");
    booth_repo.save(&local_booth).await.unwrap();
    let mut incoming_booth = cross_device_booth(&local_booth);
    incoming_booth.description = "SHOULD NOT APPEAR".to_string();
    incoming_booth.updated_at = Utc::now();
    let incoming_vendor = create_test_vendor(&incoming_booth, "V99");
    let incoming_purchase = create_test_purchase(&incoming_booth, &incoming_vendor.vendor_id);
    service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming_booth,
                vendors: vec![incoming_vendor],
                purchases: vec![incoming_purchase],
            },
            ConflictStrategy::Skip,
        )
        .await
        .unwrap();
    let saved_booth = booth_repo
        .find_by_id(&local_booth.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_booth.description, "Winter Market 2026");
    assert_eq!(
        vendor_repo
            .find_by_booth(&local_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        purchase_repo
            .find_by_booth(&local_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
}

// Case 2b: No UUID; 2 active booths with same key → Ambiguous → SkippedRecord, no new booth
#[wasm_bindgen_test]
async fn case_2b_ambiguous_produces_skipped_record_not_a_new_booth() {
    let (booth_repo, _, _, service) = build_service().await;
    let booth_a = create_test_booth("Duplicate Event 2026");
    let booth_b = create_test_booth("Duplicate Event 2026");
    booth_repo.save(&booth_a).await.unwrap();
    booth_repo.save(&booth_b).await.unwrap();
    let incoming = cross_device_booth(&booth_a);
    let summary = service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming.clone(),
                vendors: vec![create_test_vendor(&incoming, "V1")],
                purchases: vec![],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    assert_eq!(booth_repo.find_all().await.unwrap().len(), 2);
    assert_eq!(summary.skipped_records.len(), 1);
    assert!(summary.skipped_records[0].reason.contains("ambiguous"));
}

// Case 2b via import_all: ambiguous booth skipped, other booths still imported
#[wasm_bindgen_test]
async fn case_2b_ambiguous_in_full_backup_skips_only_that_booth() {
    let (booth_repo, vendor_repo, _, service) = build_service().await;
    let dup_a = create_test_booth("Duplicate Event 2026");
    let dup_b = create_test_booth("Duplicate Event 2026");
    booth_repo.save(&dup_a).await.unwrap();
    booth_repo.save(&dup_b).await.unwrap();
    let incoming_dup = cross_device_booth(&dup_a);
    let other_booth = create_test_booth("Other Event 2026");
    let summary = service
        .import_all(
            BackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booths: vec![incoming_dup.clone(), other_booth.clone()],
                vendors: vec![
                    create_test_vendor(&incoming_dup, "V1"),
                    create_test_vendor(&other_booth, "V2"),
                ],
                purchases: vec![],
                metadata: Default::default(),
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    assert!(booth_repo
        .find_by_id(&other_booth.id)
        .await
        .unwrap()
        .is_some());
    assert_eq!(
        vendor_repo
            .find_by_booth(&other_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(summary
        .skipped_records
        .iter()
        .any(|r| r.reason.contains("ambiguous")));
    assert_eq!(vendor_repo.find_by_booth(&dup_a.id).await.unwrap().len(), 0);
    assert_eq!(vendor_repo.find_by_booth(&dup_b.id).await.unwrap().len(), 0);
}

// Case 2c: No UUID; 0 active; 1 archived match → archived restored, import succeeds
#[wasm_bindgen_test]
async fn case_2c_archived_by_name_and_date_is_restored_and_imported() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service_with_archive().await;
    let mut local_booth = create_test_booth("Archived Event 2026");
    local_booth.archive(empty_archived_summary()).unwrap();
    booth_repo.save(&local_booth).await.unwrap();
    let incoming = cross_device_booth(&local_booth);
    let incoming_vendor = create_test_vendor(&incoming, "V1");
    let incoming_purchase = create_test_purchase(&incoming, &incoming_vendor.vendor_id);
    let summary = service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming.clone(),
                vendors: vec![incoming_vendor],
                purchases: vec![incoming_purchase],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    let saved = booth_repo
        .find_by_id(&local_booth.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!saved.is_archived());
    assert_eq!(
        vendor_repo
            .find_by_booth(&local_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        purchase_repo
            .find_by_booth(&local_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(booth_repo.find_all().await.unwrap().len(), 1);
    assert!(summary.skipped_records.is_empty());
}

// Case 2d: No UUID; 2 archived matches → UnresolvableAmbiguous → SkippedRecord
#[wasm_bindgen_test]
async fn case_2d_two_archived_matches_produces_skipped_record() {
    let (booth_repo, _, _, service) = build_service().await;
    let mut arch_a = create_test_booth("Old Event 2026");
    let mut arch_b = create_test_booth("Old Event 2026");
    arch_a.archive(empty_archived_summary()).unwrap();
    arch_b.archive(empty_archived_summary()).unwrap();
    booth_repo.save(&arch_a).await.unwrap();
    booth_repo.save(&arch_b).await.unwrap();
    let other_booth = create_test_booth("Other Booth 2026");
    let summary = service
        .import_all(
            BackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booths: vec![cross_device_booth(&arch_a), other_booth.clone()],
                vendors: vec![],
                purchases: vec![],
                metadata: Default::default(),
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    assert!(summary
        .skipped_records
        .iter()
        .any(|r| r.reason.contains("unresolvable")));
    assert!(booth_repo
        .find_by_id(&other_booth.id)
        .await
        .unwrap()
        .is_some());
}

// Same-name/different-date: NOT merged (different date = different event)
#[wasm_bindgen_test]
async fn same_name_different_date_are_not_merged() {
    let (booth_repo, _, _, service) = build_service().await;
    let fees = FeeConfig {
        participation_fee: dec!(10.00),
        sales_fee_percent: dec!(15.00),
        rounding_step: dec!(0.50),
    };
    let local_booth = Booth::new(
        "Annual Market".to_string(),
        NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
        fees.clone(),
    )
    .unwrap();
    booth_repo.save(&local_booth).await.unwrap();
    let incoming = Booth::new(
        "Annual Market".to_string(),
        NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
        fees,
    )
    .unwrap();
    assert_ne!(incoming.id, local_booth.id);
    service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming.clone(),
                vendors: vec![],
                purchases: vec![],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    assert_eq!(booth_repo.find_all().await.unwrap().len(), 2);
    assert!(booth_repo
        .find_by_id(&local_booth.id)
        .await
        .unwrap()
        .is_some());
    assert!(booth_repo.find_by_id(&incoming.id).await.unwrap().is_some());
}

// Case 1b via import_booth_backup_restoring_archived: UUID matches archived → restored + imported
#[wasm_bindgen_test]
async fn case_1b_uuid_matches_archived_only_restores_and_imports() {
    let (booth_repo, vendor_repo, purchase_repo, service) = build_service_with_archive().await;
    let mut archived_booth = create_test_booth("Restored Event 2026");
    archived_booth.archive(empty_archived_summary()).unwrap();
    booth_repo.save(&archived_booth).await.unwrap();
    let incoming_vendor = create_test_vendor(&archived_booth, "V1");
    let incoming_purchase = create_test_purchase(&archived_booth, &incoming_vendor.vendor_id);
    let summary = service
        .import_booth_backup_restoring_archived(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booth: archived_booth.clone(),
                vendors: vec![incoming_vendor],
                purchases: vec![incoming_purchase],
            },
            ConflictStrategy::Merge,
            ez_vend_storage::export::DeviceInfo {
                identifier: "test-device".to_string(),
                platform: "test".to_string(),
                browser: "test".to_string(),
            },
        )
        .await
        .unwrap();
    let saved = booth_repo
        .find_by_id(&archived_booth.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!saved.is_archived());
    assert_eq!(
        vendor_repo
            .find_by_booth(&archived_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        purchase_repo
            .find_by_booth(&archived_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(summary.skipped_records.is_empty());
}

// Case 1c: UUID matches archived, but active with same key → active used, archived stays archived
#[wasm_bindgen_test]
async fn case_1c_uuid_matches_archived_but_active_exists_uses_active() {
    let (booth_repo, vendor_repo, _, service) = build_service_with_archive().await;
    let active_booth = create_test_booth("Dual Booth 2026");
    booth_repo.save(&active_booth).await.unwrap();
    let mut archived_booth = create_test_booth("Dual Booth 2026");
    archived_booth.archive(empty_archived_summary()).unwrap();
    booth_repo.save(&archived_booth).await.unwrap();
    // Incoming has the archived booth's UUID — but active exists with same key
    let incoming_booth = Booth {
        id: archived_booth.id,
        ..active_booth.clone()
    };
    service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "test".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming_booth,
                vendors: vec![create_test_vendor(&active_booth, "V1")],
                purchases: vec![],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();
    assert_eq!(
        vendor_repo
            .find_by_booth(&active_booth.id)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        vendor_repo
            .find_by_booth(&archived_booth.id)
            .await
            .unwrap()
            .len(),
        0
    );
    assert!(booth_repo
        .find_by_id(&archived_booth.id)
        .await
        .unwrap()
        .unwrap()
        .is_archived());
}

// ─── MergeService + merge-then-import tests ─────────────────────────────────

async fn build_service_with_merge() -> (
    Arc<dyn BoothRepository>,
    Arc<dyn VendorRepository>,
    Arc<dyn PurchaseRepository>,
    ImportService,
    Arc<ez_vend_storage::MergeService>,
) {
    let db = Arc::new(create_test_db().await);
    let booth_repo: Arc<dyn BoothRepository> = Arc::new(IndexedDbBoothRepository::new(db.clone()));
    let vendor_repo: Arc<dyn VendorRepository> =
        Arc::new(IndexedDbVendorRepository::new(db.clone()));
    let purchase_repo: Arc<dyn PurchaseRepository> =
        Arc::new(IndexedDbPurchaseRepository::new(db.clone()));
    let archive_service = Arc::new(ez_vend_storage::ArchiveService::new(db.clone()));
    let merge_service = Arc::new(ez_vend_storage::MergeService::new(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
    ));
    let service = ImportService::with_archive_service(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
        Some(archive_service),
    )
    .with_merge_service(merge_service.clone());
    (
        booth_repo,
        vendor_repo,
        purchase_repo,
        service,
        merge_service,
    )
}

// 10.3: merge two local duplicates then import — duplicate removed, data correct
#[wasm_bindgen_test]
async fn merge_then_import_removes_duplicate_and_imports_correctly() {
    let (booth_repo, vendor_repo, purchase_repo, service, merge_service) =
        build_service_with_merge().await;

    // Two local duplicates
    let dup_a = create_test_booth("Market 2026");
    let dup_b = create_test_booth("Market 2026");
    let vendor_a = create_test_vendor(&dup_a, "V1");
    let vendor_b = create_test_vendor(&dup_b, "V2");
    let purchase_a = create_test_purchase(&dup_a, &vendor_a.vendor_id);
    let purchase_b = create_test_purchase(&dup_b, &vendor_b.vendor_id);

    booth_repo.save(&dup_a).await.unwrap();
    booth_repo.save(&dup_b).await.unwrap();
    vendor_repo.save(&vendor_a).await.unwrap();
    vendor_repo.save(&vendor_b).await.unwrap();
    purchase_repo.save(&purchase_a).await.unwrap();
    purchase_repo.save(&purchase_b).await.unwrap();

    // Incoming has same name+date but third UUID (cross-device)
    let incoming = cross_device_booth(&dup_a);
    let incoming_vendor = create_test_vendor(&incoming, "V3");
    let incoming_purchase = create_test_purchase(&incoming, &incoming_vendor.vendor_id);

    // Step 1: merge the duplicates first (canonical = dup_a, other = dup_b)
    merge_service
        .merge_booths(&dup_a.id, &dup_b.id)
        .await
        .unwrap();

    // dup_b should be gone
    assert!(booth_repo.find_by_id(&dup_b.id).await.unwrap().is_none());
    // dup_a should still exist with both vendors
    let merged_vendors = vendor_repo.find_by_booth(&dup_a.id).await.unwrap();
    assert_eq!(merged_vendors.len(), 2);

    // Step 2: import the incoming file — should now find Single(ByNameAndDate) for dup_a
    let summary = service
        .import_booth_backup(
            BoothBackupData {
                version: BACKUP_FORMAT_VERSION,
                created_at: Utc::now(),
                app_version: "device-c".to_string(),
                checksum: None,
                device_info: None,
                booth: incoming.clone(),
                vendors: vec![incoming_vendor],
                purchases: vec![incoming_purchase],
            },
            ConflictStrategy::Merge,
        )
        .await
        .unwrap();

    // Only one booth — the canonical dup_a
    assert_eq!(booth_repo.find_all().await.unwrap().len(), 1);
    assert!(booth_repo.find_by_id(&dup_a.id).await.unwrap().is_some());

    // Three vendors total (V1, V2, V3) under dup_a
    let final_vendors = vendor_repo.find_by_booth(&dup_a.id).await.unwrap();
    assert_eq!(final_vendors.len(), 3);

    // Three purchases total under dup_a
    let final_purchases = purchase_repo.find_by_booth(&dup_a.id).await.unwrap();
    assert_eq!(final_purchases.len(), 3);

    assert!(summary.skipped_records.is_empty());
}

// 10.4: idempotency — merge-then-import twice → second run finds Single, proceeds normally
#[wasm_bindgen_test]
async fn merge_then_import_is_idempotent() {
    let (booth_repo, vendor_repo, purchase_repo, service, merge_service) =
        build_service_with_merge().await;

    let dup_a = create_test_booth("Festival 2026");
    let dup_b = create_test_booth("Festival 2026");
    let vendor_a = create_test_vendor(&dup_a, "V1");
    let purchase_a = create_test_purchase(&dup_a, &vendor_a.vendor_id);

    booth_repo.save(&dup_a).await.unwrap();
    booth_repo.save(&dup_b).await.unwrap();
    vendor_repo.save(&vendor_a).await.unwrap();
    purchase_repo.save(&purchase_a).await.unwrap();

    let incoming = cross_device_booth(&dup_a);
    let incoming_vendor = create_test_vendor(&incoming, "V2");
    let incoming_purchase = create_test_purchase(&incoming, &incoming_vendor.vendor_id);

    let backup = BoothBackupData {
        version: BACKUP_FORMAT_VERSION,
        created_at: Utc::now(),
        app_version: "device-b".to_string(),
        checksum: None,
        device_info: None,
        booth: incoming.clone(),
        vendors: vec![incoming_vendor.clone()],
        purchases: vec![incoming_purchase.clone()],
    };

    // First run: merge + import
    merge_service
        .merge_booths(&dup_a.id, &dup_b.id)
        .await
        .unwrap();
    let summary1 = service
        .import_booth_backup(backup.clone(), ConflictStrategy::Merge)
        .await
        .unwrap();
    assert!(summary1.skipped_records.is_empty());

    // Second run: no merge needed (dup_b is gone), import finds Single → no duplicate
    let summary2 = service
        .import_booth_backup(backup, ConflictStrategy::Merge)
        .await
        .unwrap();
    assert!(summary2.skipped_records.is_empty());

    // Still only one booth
    assert_eq!(booth_repo.find_all().await.unwrap().len(), 1);
    // Vendors still 2 (V1 and V2, no duplicate V2)
    let vendors = vendor_repo.find_by_booth(&dup_a.id).await.unwrap();
    assert_eq!(vendors.len(), 2);
    // Purchases still 2 (purchase_a and incoming_purchase, no duplicate)
    let purchases = purchase_repo.find_by_booth(&dup_a.id).await.unwrap();
    assert_eq!(purchases.len(), 2);
}

// 5.12: import_all partial failure (forced DB write error) → no orphaned records in IDB
#[wasm_bindgen_test]
async fn import_all_partial_failure_leaves_no_orphaned_records() {
    // repo_db: used by all repositories (analysis reads + assertions)
    let repo_db = Arc::new(create_test_db().await);
    // write_db: used only by the transactional write phase; will be force-failed
    let write_db = Arc::new(create_test_db().await);

    let booth_repo: Arc<dyn BoothRepository> =
        Arc::new(IndexedDbBoothRepository::new(repo_db.clone()));
    let vendor_repo: Arc<dyn VendorRepository> =
        Arc::new(IndexedDbVendorRepository::new(repo_db.clone()));
    let purchase_repo: Arc<dyn PurchaseRepository> =
        Arc::new(IndexedDbPurchaseRepository::new(repo_db.clone()));

    let service = ImportService::with_database(
        booth_repo.clone(),
        vendor_repo.clone(),
        purchase_repo.clone(),
        None,
        write_db.clone(),
    );

    // Pre-existing state: one booth already in repo_db
    let existing_booth = create_test_booth("Pre-Existing Market");
    booth_repo.save(&existing_booth).await.unwrap();

    // Backup with a new booth, vendor, and purchase that should NOT be written
    let incoming_booth = create_test_booth("Incoming Market");
    let vendor = create_test_vendor(&incoming_booth, "V1");
    let purchase = create_test_purchase(&incoming_booth, &vendor.vendor_id);
    let backup = full_backup(incoming_booth.clone(), vendor, purchase);

    // Force write_db to reject all transaction opens (simulates mid-import DB failure)
    write_db.set_fail_writes(true);

    let result = service.import_all(backup, ConflictStrategy::Merge).await;
    assert!(
        result.is_err(),
        "import_all should return Err on forced write failure"
    );

    // repo_db must still contain exactly the pre-existing booth — zero orphans
    let all_booths = booth_repo.find_all().await.unwrap();
    assert_eq!(
        all_booths.len(),
        1,
        "only the pre-existing booth should remain; got {}",
        all_booths.len()
    );
    assert_eq!(all_booths[0].id, existing_booth.id);

    let vendors = vendor_repo.find_by_booth(&incoming_booth.id).await.unwrap();
    assert!(
        vendors.is_empty(),
        "no vendors should have been written for the incoming booth"
    );

    let purchases = purchase_repo
        .find_by_booth(&incoming_booth.id)
        .await
        .unwrap();
    assert!(
        purchases.is_empty(),
        "no purchases should have been written for the incoming booth"
    );
}
