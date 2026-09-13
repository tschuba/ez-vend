#![allow(clippy::arc_with_non_send_sync)]

// Integration tests for BoothRepository
// These tests run in a real browser environment using wasm-bindgen-test

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use chrono::NaiveDate;
use domain::models::booth::{ArchivedBoothSummary, ArchivedVendorSummary, Booth, FeeConfig};
use domain::repositories::BoothRepository;
use ez_vend_storage::indexeddb::Database;
use ez_vend_storage::repositories::IndexedDbBoothRepository;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use wasm_bindgen_test::*;

/// Helper to create a test database with a unique name
async fn create_test_db() -> Database {
    // Use a unique database name for each test to avoid conflicts
    let db_name = format!("test_db_{}", js_sys::Math::random());
    Database::new_with_name(&db_name)
        .await
        .expect("Failed to create test database")
}

/// Helper to create a test booth
fn create_test_booth(description: &str) -> Booth {
    create_test_booth_with_stepping(description, None)
}

fn create_test_booth_with_stepping(description: &str, amount_stepping: Option<Decimal>) -> Booth {
    let fees = FeeConfig {
        participation_fee: Decimal::from_str("10.00").unwrap(),
        sales_fee_percent: Decimal::from_str("15.00").unwrap(),
        rounding_step: Decimal::from_str("0.50").unwrap(),
    };

    let mut booth = Booth::new(
        description.to_string(),
        NaiveDate::from_ymd_opt(2026, 3, 25).unwrap(),
        fees,
    )
    .unwrap();

    booth.update_amount_stepping(amount_stepping).unwrap();
    booth
}

#[wasm_bindgen_test]
async fn test_save_and_find_booth() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let booth = create_test_booth("Test Booth 1");
    let booth_id = booth.id;

    // Save the booth
    let save_result = repo.save(&booth).await;
    assert!(
        save_result.is_ok(),
        "Failed to save booth: {:?}",
        save_result.err()
    );

    // Find the booth by ID
    let found = repo.find_by_id(&booth_id).await;
    assert!(found.is_ok(), "Failed to find booth: {:?}", found.err());

    let found_booth = found.unwrap();
    assert!(found_booth.is_some(), "Booth not found after save");

    let found_booth = found_booth.unwrap();
    assert_eq!(found_booth.id, booth_id);
    assert_eq!(found_booth.description, "Test Booth 1");
}

#[wasm_bindgen_test]
async fn test_find_all_booths() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    // Initially empty
    let all_booths = repo.find_all().await.unwrap();
    assert_eq!(all_booths.len(), 0);

    // Save multiple booths
    let booth1 = create_test_booth("Booth 1");
    let booth2 = create_test_booth("Booth 2");
    let booth3 = create_test_booth("Booth 3");

    repo.save(&booth1).await.unwrap();
    repo.save(&booth2).await.unwrap();
    repo.save(&booth3).await.unwrap();

    // Find all should return 3 booths
    let all_booths = repo.find_all().await.unwrap();
    assert_eq!(all_booths.len(), 3);

    let descriptions: Vec<String> = all_booths.iter().map(|b| b.description.clone()).collect();
    assert!(descriptions.contains(&"Booth 1".to_string()));
    assert!(descriptions.contains(&"Booth 2".to_string()));
    assert!(descriptions.contains(&"Booth 3".to_string()));
}

#[wasm_bindgen_test]
async fn test_update_booth() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let mut booth = create_test_booth("Original Description");
    let booth_id = booth.id;

    // Save initial booth
    repo.save(&booth).await.unwrap();

    // Update the booth
    booth.description = "Updated Description".to_string();
    let update_result = repo.save(&booth).await;
    assert!(
        update_result.is_ok(),
        "Failed to update booth: {:?}",
        update_result.err()
    );

    // Verify the update
    let found = repo.find_by_id(&booth_id).await.unwrap().unwrap();
    assert_eq!(found.description, "Updated Description");
}

#[wasm_bindgen_test]
async fn test_amount_stepping_persistence() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let create_step = Decimal::from_str("0.50").unwrap();
    let update_step = Decimal::from_str("1.25").unwrap();

    let booth_with_step = create_test_booth_with_stepping("Stepped Booth", Some(create_step));
    let booth_with_step_id = booth_with_step.id;
    repo.save(&booth_with_step).await.unwrap();

    let found_with_step = repo.find_by_id(&booth_with_step_id).await.unwrap().unwrap();
    assert_eq!(found_with_step.amount_stepping, Some(create_step));

    let mut booth_without_step = create_test_booth("Optional Step Booth");
    let booth_without_step_id = booth_without_step.id;
    repo.save(&booth_without_step).await.unwrap();

    booth_without_step
        .update_amount_stepping(Some(update_step))
        .unwrap();
    repo.save(&booth_without_step).await.unwrap();

    let found_updated_step = repo
        .find_by_id(&booth_without_step_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found_updated_step.amount_stepping, Some(update_step));

    let mut booth_cleared_step = found_with_step;
    booth_cleared_step.update_amount_stepping(None).unwrap();
    repo.save(&booth_cleared_step).await.unwrap();

    let found_cleared_step = repo.find_by_id(&booth_with_step_id).await.unwrap().unwrap();
    assert_eq!(found_cleared_step.amount_stepping, None);
}

#[wasm_bindgen_test]
async fn test_delete_booth() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let booth = create_test_booth("To Be Deleted");
    let booth_id = booth.id;

    // Save the booth
    repo.save(&booth).await.unwrap();

    // Verify it exists
    let found = repo.find_by_id(&booth_id).await.unwrap();
    assert!(found.is_some());

    // Delete the booth
    let delete_result = repo.delete(&booth_id).await;
    assert!(
        delete_result.is_ok(),
        "Failed to delete booth: {:?}",
        delete_result.err()
    );

    // Verify it's gone
    let found = repo.find_by_id(&booth_id).await.unwrap();
    assert!(found.is_none(), "Booth still exists after deletion");
}

#[wasm_bindgen_test]
async fn test_delete_nonexistent_booth() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let booth_id = domain::models::shared::BoothId::new();

    // Deleting a non-existent booth should succeed (idempotent)
    let delete_result = repo.delete(&booth_id).await;
    assert!(delete_result.is_ok());
}

#[wasm_bindgen_test]
async fn test_concurrent_saves() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    // Create multiple booths
    let booth1 = create_test_booth("Concurrent 1");
    let booth2 = create_test_booth("Concurrent 2");
    let booth3 = create_test_booth("Concurrent 3");

    // Save concurrently
    let repo1 = repo.clone();
    let repo2 = repo.clone();
    let repo3 = repo.clone();

    let f1 = repo1.save(&booth1);
    let f2 = repo2.save(&booth2);
    let f3 = repo3.save(&booth3);

    // Wait for all to complete
    let (r1, r2, r3) = futures::join!(f1, f2, f3);

    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());

    // Verify all were saved
    let all_booths = repo.find_all().await.unwrap();
    assert_eq!(all_booths.len(), 3);
}

#[wasm_bindgen_test]
async fn test_find_by_description_and_date() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let booth = create_test_booth("Duplicate Check Booth");
    let booth_id = booth.id;
    let booth_date = booth.date;
    repo.save(&booth).await.unwrap();

    let found = repo
        .find_by_description_and_date("Duplicate Check Booth", &booth_date)
        .await
        .unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, booth_id);

    let missing = repo
        .find_by_description_and_date(
            "Duplicate Check Booth",
            &NaiveDate::from_ymd_opt(2026, 3, 26).unwrap(),
        )
        .await
        .unwrap();
    assert!(missing.is_none());
}

#[wasm_bindgen_test]
async fn test_find_by_description_and_date_trims_lookup_value() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let booth = create_test_booth("Duplicate Check Booth");
    let booth_id = booth.id;
    let booth_date = booth.date;
    repo.save(&booth).await.unwrap();

    let found = repo
        .find_by_description_and_date("  Duplicate Check Booth  ", &booth_date)
        .await
        .unwrap();

    assert!(found.is_some());
    assert_eq!(found.unwrap().id, booth_id);
}

#[wasm_bindgen_test]
async fn test_archived_summary_persists_decimal_values() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbBoothRepository::new(db);

    let mut booth = create_test_booth("Archived Booth");
    let booth_id = booth.id;
    booth
        .archive(ArchivedBoothSummary {
            total_revenue: Decimal::from_str("76").unwrap(),
            total_booth_revenue: Decimal::from_str("17.0").unwrap(),
            vendor_count: 5,
            purchase_count: 8,
            item_count: 10,
            first_purchase_at: None,
            last_purchase_at: None,
            vendor_summaries: vec![ArchivedVendorSummary {
                vendor_id: domain::VendorId::new("1".to_string()),
                gross_sales: Decimal::from_str("66").unwrap(),
                fees_due: Decimal::from_str("11.0").unwrap(),
                net_payout: Decimal::from_str("55.0").unwrap(),
                item_count: 6,
            }],
        })
        .unwrap();

    repo.save(&booth).await.unwrap();

    let found = repo.find_by_id(&booth_id).await.unwrap().unwrap();
    let summary = found
        .archived_summary
        .expect("archived summary should load");

    assert_eq!(summary.total_revenue, Decimal::from_str("76").unwrap());
    assert_eq!(
        summary.total_booth_revenue,
        Decimal::from_str("17.0").unwrap()
    );
    assert_eq!(summary.vendor_count, 5);
    assert_eq!(summary.purchase_count, 8);
    assert_eq!(summary.item_count, 10);
    assert_eq!(summary.vendor_summaries.len(), 1);
    assert_eq!(
        summary.vendor_summaries[0].gross_sales,
        Decimal::from_str("66").unwrap()
    );
    assert_eq!(
        summary.vendor_summaries[0].fees_due,
        Decimal::from_str("11.0").unwrap()
    );
    assert_eq!(
        summary.vendor_summaries[0].net_payout,
        Decimal::from_str("55.0").unwrap()
    );
}
