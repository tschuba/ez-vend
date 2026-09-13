use crate::error::{DomainError, DomainResult};
use crate::error_code::ValidationError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use super::shared::{BoothId, ItemId, PurchaseId, VendorId};

/// Represents a purchase transaction with multiple items
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Purchase {
    pub id: PurchaseId,
    pub booth_id: BoothId,
    pub items: Vec<PurchaseItem>,
    pub timestamp: DateTime<Utc>,
    pub note: Option<String>,
}

/// Individual item within a purchase
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PurchaseItem {
    pub id: ItemId,
    pub amount: Decimal,
    pub vendor_id: VendorId,
}

impl Purchase {
    pub fn new(booth_id: BoothId, items: Vec<PurchaseItem>) -> DomainResult<Self> {
        if items.is_empty() {
            return Err(DomainError::Validation(ValidationError::PurchaseEmpty));
        }

        let total: Decimal = items.iter().map(|item| item.amount).sum();
        if total > dec!(10000000) {
            return Err(DomainError::Validation(
                ValidationError::PurchaseTotalTooLarge,
            ));
        }

        Ok(Self {
            id: PurchaseId::new(),
            booth_id,
            items,
            timestamp: Utc::now(),
            note: None,
        })
    }

    pub fn with_note(mut self, note: String) -> Self {
        self.note = Some(note);
        self
    }

    /// Calculate total amount of all items in the purchase
    pub fn total_amount(&self) -> Decimal {
        self.items.iter().map(|item| item.amount).sum()
    }

    /// Get unique vendor IDs from all items in this purchase
    pub fn get_vendor_ids(&self) -> Vec<VendorId> {
        use std::collections::HashSet;
        let mut vendors: Vec<VendorId> = self
            .items
            .iter()
            .map(|item| item.vendor_id.clone())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        vendors.sort();
        vendors
    }

    /// Get the primary vendor (most common vendor in items, or first if tied)
    pub fn primary_vendor_id(&self) -> Option<VendorId> {
        use std::collections::HashMap;
        let mut counts: HashMap<VendorId, usize> = HashMap::new();
        for item in &self.items {
            *counts.entry(item.vendor_id.clone()).or_insert(0) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(vendor_id, _)| vendor_id)
    }
}

impl PurchaseItem {
    pub fn new(amount: Decimal, vendor_id: VendorId) -> DomainResult<Self> {
        if amount <= Decimal::ZERO {
            return Err(DomainError::Validation(
                ValidationError::ItemAmountNotPositive,
            ));
        }

        if amount.scale() > 2 {
            return Err(DomainError::Validation(
                ValidationError::ItemAmountTooManyDecimals,
            ));
        }

        if amount > dec!(1000000) {
            return Err(DomainError::Validation(ValidationError::ItemAmountTooLarge));
        }

        Ok(Self {
            id: ItemId::new(),
            amount,
            vendor_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vendor_id(id: &str) -> VendorId {
        VendorId::new(id.to_string())
    }

    #[test]
    fn rejects_negative_item_amount() {
        assert!(PurchaseItem::new(dec!(-10.00), vendor_id("1")).is_err());
    }

    #[test]
    fn rejects_zero_item_amount() {
        assert!(PurchaseItem::new(Decimal::ZERO, vendor_id("1")).is_err());
    }

    #[test]
    fn rejects_item_precision_over_two_decimals() {
        assert!(PurchaseItem::new(dec!(10.123), vendor_id("1")).is_err());
    }

    #[test]
    fn rejects_large_item_amount() {
        assert!(PurchaseItem::new(dec!(1000000.01), vendor_id("1")).is_err());
    }

    #[test]
    fn accepts_valid_item_amounts() {
        for amount in [dec!(0.01), dec!(10.50), dec!(999999.99)] {
            assert!(PurchaseItem::new(amount, vendor_id("1")).is_ok());
        }
    }

    #[test]
    fn rejects_empty_purchase() {
        assert!(Purchase::new(BoothId::new(), Vec::new()).is_err());
    }

    #[test]
    fn rejects_purchase_total_overflow() {
        let items: Vec<PurchaseItem> = (0..11)
            .map(|idx| PurchaseItem::new(dec!(1000000.00), vendor_id(&idx.to_string())).unwrap())
            .collect();
        assert!(Purchase::new(BoothId::new(), items).is_err());
    }

    #[test]
    fn creates_valid_purchase() {
        let items = vec![
            PurchaseItem::new(dec!(100.00), vendor_id("1")).unwrap(),
            PurchaseItem::new(dec!(250.50), vendor_id("2")).unwrap(),
        ];

        let purchase = Purchase::new(BoothId::new(), items).unwrap();
        assert_eq!(purchase.total_amount(), dec!(350.50));
        assert_eq!(purchase.items.len(), 2);
    }
}
