#![allow(clippy::arc_with_non_send_sync)]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use domain::models::purchase::{Purchase, PurchaseItem};
use domain::models::shared::{BoothId, VendorId};
use domain::repositories::PurchaseRepository;
use ez_vend_storage::indexeddb::Database;
use ez_vend_storage::repositories::IndexedDbPurchaseRepository;
use rust_decimal_macros::dec;
use std::sync::Arc;
use wasm_bindgen_test::*;
use web_sys::window;

const TEST_DRAFT_KEY: &str = "ez-vend-safari-draft-test";

async fn create_test_db() -> Database {
    let db_name = format!("test_safari_db_{}", js_sys::Math::random());
    Database::new_with_name(&db_name)
        .await
        .expect("Failed to create Safari-specific test database")
}

fn local_storage() -> web_sys::Storage {
    window()
        .expect("window should be available")
        .local_storage()
        .expect("localStorage access should not throw")
        .expect("localStorage should exist in browser tests")
}

fn current_browser_is_safari() -> bool {
    window()
        .and_then(|window| window.navigator().user_agent().ok())
        .map(|user_agent| user_agent.contains("Safari") && !user_agent.contains("Chrome"))
        .unwrap_or(false)
}

fn create_purchase(
    booth_id: BoothId,
    vendor_suffix: &str,
    amount: rust_decimal::Decimal,
) -> Purchase {
    Purchase::new(
        booth_id,
        vec![PurchaseItem::new(amount, VendorId::new(vendor_suffix.to_string())).unwrap()],
    )
    .unwrap()
}

#[wasm_bindgen_test]
async fn safari_handles_large_local_storage_draft_payloads_without_truncation() {
    let storage = local_storage();
    let browser_is_safari = current_browser_is_safari();
    let large_payload = format!(
        "{{\"browser\":\"{}\",\"payload\":\"{}\"}}",
        if browser_is_safari {
            "safari"
        } else {
            "non-safari"
        },
        "x".repeat(512 * 1024)
    );

    let write_result = storage.set_item(TEST_DRAFT_KEY, &large_payload);

    match write_result {
        Ok(()) => {
            let restored = storage
                .get_item(TEST_DRAFT_KEY)
                .expect("stored payload should remain readable")
                .expect("stored payload should exist");
            assert_eq!(restored.len(), large_payload.len());
            assert_eq!(restored, large_payload);
        }
        Err(_) => {
            assert!(
                browser_is_safari,
                "non-Safari browsers should store this payload size"
            );
        }
    }

    let _ = storage.remove_item(TEST_DRAFT_KEY);
}

#[wasm_bindgen_test]
async fn safari_rapid_indexeddb_round_trips_preserve_purchase_consistency() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbPurchaseRepository::new(db);
    let booth_id = BoothId::new();

    for index in 0..8 {
        let purchase = create_purchase(booth_id, &format!("vendor-{index}"), dec!(42.50));
        repo.save(&purchase)
            .await
            .expect("rapid Safari-sensitive save should succeed");

        let purchases = repo
            .find_by_booth(&booth_id)
            .await
            .expect("rapid Safari-sensitive read should succeed");
        assert_eq!(purchases.len(), index + 1);
    }

    let totals = repo
        .get_running_totals(&booth_id)
        .await
        .expect("running totals should remain correct after rapid round trips");
    assert_eq!(totals.total_checkouts, 8);
    assert_eq!(totals.total_items, 8);
    assert_eq!(totals.total_sales, dec!(340.00));
}

#[wasm_bindgen_test]
async fn safari_decimal_serialization_preserves_currency_edge_cases() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbPurchaseRepository::new(db);
    let booth_id = BoothId::new();
    let amounts = [dec!(0.01), dec!(0.99), dec!(123.45), dec!(999999.99)];

    for (index, amount) in amounts.iter().enumerate() {
        let purchase = create_purchase(booth_id, &format!("edge-{index}"), *amount);
        repo.save(&purchase)
            .await
            .expect("edge-case purchase should save");

        let restored = repo
            .find_by_booth(&booth_id)
            .await
            .expect("edge-case purchase should reload")
            .into_iter()
            .find(|candidate| candidate.id == purchase.id)
            .expect("saved purchase should be present");

        assert_eq!(restored.items.len(), 1);
        assert_eq!(restored.items[0].amount, *amount);
    }
}

#[wasm_bindgen_test]
async fn safari_corrupted_draft_payloads_remain_detectable_and_replaceable() {
    let storage = local_storage();
    let corrupted = "{\"vendor_id\":\"1\",\"amount\":\"100";
    let replacement = r#"{"vendor_id":"1","amount":"100.00","items":[]}"#;

    storage
        .set_item(TEST_DRAFT_KEY, corrupted)
        .expect("corrupted payload should still be written as raw text");

    let raw = storage
        .get_item(TEST_DRAFT_KEY)
        .expect("corrupted payload should still be readable")
        .expect("corrupted payload should exist");
    assert!(serde_json::from_str::<serde_json::Value>(&raw).is_err());

    storage
        .set_item(TEST_DRAFT_KEY, replacement)
        .expect("replacement payload should overwrite corrupted data");

    let restored = storage
        .get_item(TEST_DRAFT_KEY)
        .expect("replacement payload should be readable")
        .expect("replacement payload should exist");
    let restored_json: serde_json::Value =
        serde_json::from_str(&restored).expect("replacement payload should parse cleanly");

    assert_eq!(restored_json["vendor_id"], "1");
    assert_eq!(restored_json["amount"], "100.00");

    let _ = storage.remove_item(TEST_DRAFT_KEY);
}
