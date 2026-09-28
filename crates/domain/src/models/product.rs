use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::shared::{BoothId, ProductGroupId, ProductId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TailwindColor {
    Red,
    Orange,
    Amber,
    Green,
    Teal,
    Blue,
    Violet,
    Pink,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductGroup {
    pub id: ProductGroupId,
    pub booth_id: BoothId,
    pub name: String,
    pub color: TailwindColor,
    pub emoji: Option<String>,
    pub sort_order: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: ProductId,
    pub booth_id: BoothId,
    pub product_group_id: ProductGroupId,
    pub name: String,
    pub price: Decimal,
    pub sort_order: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_stock: Option<u32>,
}

impl Product {
    /// Remaining stock after `sold` units were sold, or `None` if this product
    /// has no stock limit configured. Can go negative if oversold — callers
    /// decide how to display that (still selectable, per product spec).
    pub fn remaining_stock(&self, sold: usize) -> Option<i64> {
        self.initial_stock
            .map(|initial| initial as i64 - sold as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn product(initial_stock: Option<u32>) -> Product {
        Product {
            id: ProductId::new(),
            booth_id: BoothId::new(),
            product_group_id: ProductGroupId::new(),
            name: "Test".to_string(),
            price: Decimal::ZERO,
            sort_order: 0,
            initial_stock,
        }
    }

    #[test]
    fn remaining_stock_is_none_without_limit() {
        assert_eq!(product(None).remaining_stock(5), None);
    }

    #[test]
    fn remaining_stock_subtracts_sold_count() {
        assert_eq!(product(Some(10)).remaining_stock(3), Some(7));
    }

    #[test]
    fn remaining_stock_goes_negative_when_oversold() {
        assert_eq!(product(Some(10)).remaining_stock(12), Some(-2));
    }
}
