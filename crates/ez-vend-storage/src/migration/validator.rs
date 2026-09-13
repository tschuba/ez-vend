use std::collections::HashSet;

use domain::{Booth, BoothId, Purchase, Vendor};
use rust_decimal::Decimal;

use super::types::LegacyPurchase;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationIssue {
    PurchaseTotalMismatch {
        purchase_id: String,
        expected: Decimal,
        actual: Decimal,
    },
    VendorMissingBooth {
        booth_id: String,
        vendor_id: String,
    },
    PurchaseMissingBooth {
        booth_id: String,
        purchase_id: String,
    },
    PurchaseWithoutItems {
        purchase_id: String,
    },
    PurchaseItemMissingVendor {
        booth_id: String,
        purchase_id: String,
        item_id: String,
        vendor_id: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MigrationValidationSummary {
    pub booth_count: usize,
    pub vendor_count: usize,
    pub purchase_count: usize,
    pub total_sales: Decimal,
    pub issues: Vec<ValidationIssue>,
}

pub fn validate_dataset(
    booths: &[Booth],
    vendors: &[Vendor],
    purchases: &[Purchase],
    legacy_purchases: &[LegacyPurchase],
) -> MigrationValidationSummary {
    let mut issues = collect_purchase_total_issues(purchases, legacy_purchases);
    collect_reference_issues(booths, vendors, purchases, &mut issues);

    MigrationValidationSummary {
        booth_count: booths.len(),
        vendor_count: vendors.len(),
        purchase_count: purchases.len(),
        total_sales: purchases.iter().map(Purchase::total_amount).sum(),
        issues,
    }
}

fn collect_purchase_total_issues(
    purchases: &[Purchase],
    legacy_purchases: &[LegacyPurchase],
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    for purchase in purchases {
        let expected = legacy_purchases
            .iter()
            .find(|legacy| legacy.purchase_id == purchase.id.to_string())
            .map(|legacy| legacy.total_value);

        if let Some(expected) = expected {
            let actual = purchase.total_amount();
            if actual != expected {
                issues.push(ValidationIssue::PurchaseTotalMismatch {
                    purchase_id: purchase.id.to_string(),
                    expected,
                    actual,
                });
            }
        }
    }

    issues
}

fn collect_reference_issues(
    booths: &[Booth],
    vendors: &[Vendor],
    purchases: &[Purchase],
    issues: &mut Vec<ValidationIssue>,
) {
    let booth_ids: HashSet<BoothId> = booths.iter().map(|booth| booth.id).collect();
    let vendor_keys: HashSet<(BoothId, String)> = vendors
        .iter()
        .map(|vendor| (vendor.booth_id, vendor.vendor_id.to_string()))
        .collect();

    for vendor in vendors {
        if !booth_ids.contains(&vendor.booth_id) {
            issues.push(ValidationIssue::VendorMissingBooth {
                booth_id: vendor.booth_id.to_string(),
                vendor_id: vendor.vendor_id.to_string(),
            });
        }
    }

    for purchase in purchases {
        if !booth_ids.contains(&purchase.booth_id) {
            issues.push(ValidationIssue::PurchaseMissingBooth {
                booth_id: purchase.booth_id.to_string(),
                purchase_id: purchase.id.to_string(),
            });
            continue;
        }

        if purchase.items.is_empty() {
            issues.push(ValidationIssue::PurchaseWithoutItems {
                purchase_id: purchase.id.to_string(),
            });
            continue;
        }

        for item in &purchase.items {
            if !vendor_keys.contains(&(purchase.booth_id, item.vendor_id.to_string())) {
                issues.push(ValidationIssue::PurchaseItemMissingVendor {
                    booth_id: purchase.booth_id.to_string(),
                    purchase_id: purchase.id.to_string(),
                    item_id: item.id.to_string(),
                    vendor_id: item.vendor_id.to_string(),
                });
            }
        }
    }
}
