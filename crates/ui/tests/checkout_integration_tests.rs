#![allow(clippy::arc_with_non_send_sync)]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use chrono::NaiveDate;
use domain::models::booth::{Booth, FeeConfig};
use domain::models::purchase::{Purchase, PurchaseItem};
use domain::models::shared::{BoothId, VendorId};
use domain::repositories::{BoothRepository, PurchaseRepository, VendorRepository};
use domain::services::ReportService;
use ez_vend_storage::indexeddb::Database;
use ez_vend_storage::repositories::{
    IndexedDbBoothRepository, IndexedDbProductGroupRepository, IndexedDbProductRepository,
    IndexedDbPurchaseRepository, IndexedDbVendorRepository,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::str::FromStr;
use std::sync::Arc;
use wasm_bindgen_test::*;

async fn create_test_db() -> Database {
    let db_name = format!("test_ui_checkout_{}", js_sys::Math::random());
    Database::new_with_name(&db_name)
        .await
        .expect("Failed to create test database")
}

async fn create_test_booth(repo: &IndexedDbBoothRepository) -> Booth {
    let booth = Booth::new(
        "Integration Test Booth".to_string(),
        NaiveDate::from_ymd_opt(2026, 3, 27).unwrap(),
        FeeConfig {
            participation_fee: Decimal::from_str("10.00").unwrap(),
            sales_fee_percent: Decimal::from_str("15.00").unwrap(),
            rounding_step: Decimal::from_str("0.50").unwrap(),
        },
        domain::models::BoothType::ThirdPartySale,
        None,
    )
    .unwrap();

    repo.save(&booth).await.unwrap();
    booth
}

#[wasm_bindgen_test]
async fn purchase_persists_and_loads_single_item_checkout() {
    let db = Arc::new(create_test_db().await);
    let booth_repo = IndexedDbBoothRepository::new(db.clone());
    let purchase_repo = IndexedDbPurchaseRepository::new(db.clone());

    let booth = create_test_booth(&booth_repo).await;
    let purchase = Purchase::new(
        booth.id,
        vec![PurchaseItem::new(dec!(100.00), VendorId::new("1".to_string())).unwrap()],
    )
    .unwrap();
    purchase_repo.save(&purchase).await.unwrap();

    let loaded = purchase_repo
        .find_by_booth(&booth.id)
        .await
        .unwrap()
        .into_iter()
        .find(|saved| saved.id == purchase.id)
        .unwrap();
    assert_eq!(loaded.total_amount(), dec!(100.00));
    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.items[0].vendor_id.as_str(), "1");
}

#[wasm_bindgen_test]
async fn purchase_supports_multiple_vendors_and_totals() {
    let db = Arc::new(create_test_db().await);
    let booth_repo = IndexedDbBoothRepository::new(db.clone());
    let purchase_repo = IndexedDbPurchaseRepository::new(db.clone());

    let booth = create_test_booth(&booth_repo).await;
    let purchase = Purchase::new(
        booth.id,
        vec![
            PurchaseItem::new(dec!(50.00), VendorId::new("1".to_string())).unwrap(),
            PurchaseItem::new(dec!(75.50), VendorId::new("2".to_string())).unwrap(),
            PurchaseItem::new(dec!(25.25), VendorId::new("3".to_string())).unwrap(),
        ],
    )
    .unwrap();

    purchase_repo.save(&purchase).await.unwrap();

    let purchases = purchase_repo.find_by_booth(&booth.id).await.unwrap();
    assert_eq!(purchases.len(), 1);
    assert_eq!(purchases[0].total_amount(), dec!(150.75));
    assert_eq!(purchases[0].get_vendor_ids().len(), 3);
}

#[wasm_bindgen_test]
async fn purchase_running_totals_match_saved_data() {
    let db = Arc::new(create_test_db().await);
    let booth_repo = IndexedDbBoothRepository::new(db.clone());
    let purchase_repo = IndexedDbPurchaseRepository::new(db.clone());

    let booth = create_test_booth(&booth_repo).await;

    for amount in [dec!(10.00), dec!(20.50), dec!(30.25)] {
        let purchase = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(amount, VendorId::new("1".to_string())).unwrap()],
        )
        .unwrap();
        purchase_repo.save(&purchase).await.unwrap();
    }

    let totals = purchase_repo.get_running_totals(&booth.id).await.unwrap();
    assert_eq!(totals.total_sales, dec!(60.75));
    assert_eq!(totals.total_items, 3);
    assert_eq!(totals.total_checkouts, 3);
}

#[wasm_bindgen_test]
async fn purchase_validation_rejects_invalid_amounts() {
    assert!(PurchaseItem::new(dec!(-10.00), VendorId::new("1".to_string())).is_err());
    assert!(PurchaseItem::new(dec!(0), VendorId::new("1".to_string())).is_err());
    assert!(PurchaseItem::new(dec!(10.123), VendorId::new("1".to_string())).is_err());
    assert!(PurchaseItem::new(dec!(1000000.01), VendorId::new("1".to_string())).is_err());
}

#[wasm_bindgen_test]
async fn purchase_validation_rejects_empty_purchase() {
    assert!(Purchase::new(BoothId::new(), Vec::new()).is_err());
}

#[wasm_bindgen_test]
async fn fee_reports_remain_consistent_across_storage_backed_services() {
    let db = Arc::new(create_test_db().await);
    let booth_repo = IndexedDbBoothRepository::new(db.clone());
    let vendor_repo = IndexedDbVendorRepository::new(db.clone());
    let purchase_repo = IndexedDbPurchaseRepository::new(db.clone());

    let booth = create_test_booth(&booth_repo).await;

    for vendor_id in ["1", "2", "3"] {
        vendor_repo
            .save(&domain::models::vendor::Vendor::new(
                VendorId::new(vendor_id.to_string()),
                booth.id,
            ))
            .await
            .unwrap();
    }

    for (vendor_id, amount) in [("1", dec!(100.00)), ("2", dec!(518.11)), ("3", dec!(75.25))] {
        let purchase = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(amount, VendorId::new((*vendor_id).to_string())).unwrap()],
        )
        .unwrap();
        purchase_repo.save(&purchase).await.unwrap();
    }

    let report_service = ReportService::new(
        IndexedDbPurchaseRepository::new(db.clone()),
        IndexedDbBoothRepository::new(db.clone()),
        IndexedDbVendorRepository::new(db.clone()),
        Arc::new(IndexedDbProductGroupRepository::new(db.clone())),
        Arc::new(IndexedDbProductRepository::new(db.clone())),
    );

    let summary = report_service
        .generate_booth_summary(&booth.id, None)
        .await
        .unwrap();
    let vendor_reports = report_service
        .generate_vendor_reports(
            &booth.id,
            vec![
                VendorId::new("1".to_string()),
                VendorId::new("2".to_string()),
                VendorId::new("3".to_string()),
            ],
            None,
        )
        .await
        .unwrap();

    let total_vendor_fees: Decimal = vendor_reports
        .iter()
        .map(|report| report.participation_fee + report.sales_fee)
        .sum();

    assert_eq!(total_vendor_fees, summary.total_booth_revenue);
    assert_eq!(
        summary.total_booth_revenue,
        summary.total_participation_fees + summary.total_sales_fees
    );
}

#[wasm_bindgen_test]
async fn deleting_a_purchase_recalculates_running_totals_and_reports() {
    let db = Arc::new(create_test_db().await);
    let booth_repo = IndexedDbBoothRepository::new(db.clone());
    let vendor_repo = IndexedDbVendorRepository::new(db.clone());
    let purchase_repo = IndexedDbPurchaseRepository::new(db.clone());

    let booth = create_test_booth(&booth_repo).await;

    for vendor_id in ["1", "2"] {
        vendor_repo
            .save(&domain::models::vendor::Vendor::new(
                VendorId::new(vendor_id.to_string()),
                booth.id,
            ))
            .await
            .unwrap();
    }

    let purchase_one = Purchase::new(
        booth.id,
        vec![PurchaseItem::new(dec!(100.00), VendorId::new("1".to_string())).unwrap()],
    )
    .unwrap();
    let purchase_two = Purchase::new(
        booth.id,
        vec![PurchaseItem::new(dec!(50.00), VendorId::new("2".to_string())).unwrap()],
    )
    .unwrap();

    purchase_repo.save(&purchase_one).await.unwrap();
    purchase_repo.save(&purchase_two).await.unwrap();

    purchase_repo
        .delete_from_booth(&booth.id, &purchase_one.id)
        .await
        .unwrap();

    let totals = purchase_repo.get_running_totals(&booth.id).await.unwrap();
    assert_eq!(totals.total_sales, dec!(50.00));
    assert_eq!(totals.total_items, 1);
    assert_eq!(totals.total_checkouts, 1);

    let report_service = ReportService::new(
        IndexedDbPurchaseRepository::new(db.clone()),
        IndexedDbBoothRepository::new(db.clone()),
        IndexedDbVendorRepository::new(db.clone()),
        Arc::new(IndexedDbProductGroupRepository::new(db.clone())),
        Arc::new(IndexedDbProductRepository::new(db.clone())),
    );

    let summary = report_service
        .generate_booth_summary(&booth.id, None)
        .await
        .unwrap();
    assert_eq!(summary.total_revenue, dec!(50.00));
    assert_eq!(summary.unique_vendors, 1);
    assert_eq!(summary.vendor_summaries.len(), 1);
    assert_eq!(summary.vendor_summaries[0].vendor_id.as_str(), "2");

    let vendor_reports = report_service
        .generate_vendor_reports(&booth.id, vec![VendorId::new("2".to_string())], None)
        .await
        .unwrap();
    assert_eq!(vendor_reports.len(), 1);
    assert_eq!(vendor_reports[0].sales_sum, dec!(50.00));
}
