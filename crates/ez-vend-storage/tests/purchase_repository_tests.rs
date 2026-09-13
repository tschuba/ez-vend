#![allow(clippy::arc_with_non_send_sync)]

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use domain::models::purchase::{Purchase, PurchaseItem};
use domain::models::shared::{BoothId, VendorId};
use domain::repositories::PurchaseRepository;
use ez_vend_storage::indexeddb::Database;
use ez_vend_storage::repositories::IndexedDbPurchaseRepository;
use rexie::TransactionMode;
use std::sync::Arc;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

async fn create_test_db() -> Database {
    let db_name = format!("test_purchase_db_{}", js_sys::Math::random());
    Database::new_with_name(&db_name)
        .await
        .expect("Failed to create test database")
}

fn create_test_purchase(booth_id: &BoothId, amount: rust_decimal::Decimal) -> Purchase {
    Purchase::new(
        *booth_id,
        vec![PurchaseItem::new(amount, VendorId::new("1".to_string())).unwrap()],
    )
    .unwrap()
}

#[wasm_bindgen_test]
async fn test_find_by_booth_with_diagnostics_detects_corruption() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbPurchaseRepository::new(db.clone());
    let booth_id = BoothId::new();

    let purchase = create_test_purchase(&booth_id, rust_decimal_macros::dec!(100.00));
    repo.save(&purchase).await.unwrap();

    let transaction = db
        .transaction(&["purchases"], TransactionMode::ReadWrite)
        .unwrap();
    let store = transaction.store("purchases").unwrap();

    let corrupted = js_sys::Object::new();
    js_sys::Reflect::set(
        &corrupted,
        &JsValue::from_str("booth_id"),
        &JsValue::from_str(&booth_id.as_str()),
    )
    .unwrap();
    js_sys::Reflect::set(
        &corrupted,
        &JsValue::from_str("id"),
        &JsValue::from_str("corrupted"),
    )
    .unwrap();
    store.add(&corrupted, None).await.unwrap();
    transaction.done().await.unwrap();

    let (purchases, errors) = repo
        .find_by_booth_with_diagnostics(&booth_id)
        .await
        .unwrap();
    assert_eq!(purchases.len(), 1);
    assert_eq!(purchases[0].id, purchase.id);
    assert!(!errors.is_empty());
}

#[wasm_bindgen_test]
async fn test_find_by_id_works_with_compound_purchase_keys() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbPurchaseRepository::new(db.clone());
    let booth_id = BoothId::new();

    let purchase = create_test_purchase(&booth_id, rust_decimal_macros::dec!(42.00));
    repo.save(&purchase).await.unwrap();

    let loaded = repo.find_by_id(&purchase.id).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().id, purchase.id);
}

#[wasm_bindgen_test]
async fn test_delete_from_booth_preserves_purchase_for_wrong_booth_key() {
    let db = Arc::new(create_test_db().await);
    let repo = IndexedDbPurchaseRepository::new(db.clone());
    let booth_id = BoothId::new();
    let wrong_booth_id = BoothId::new();

    let purchase = create_test_purchase(&booth_id, rust_decimal_macros::dec!(42.00));
    repo.save(&purchase).await.unwrap();

    repo.delete_from_booth(&wrong_booth_id, &purchase.id)
        .await
        .unwrap();

    let loaded = repo.find_by_id(&purchase.id).await.unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().booth_id, booth_id);
}
