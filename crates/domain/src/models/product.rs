use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::booth_type::TailwindColor;
use super::shared::{BoothId, ProductGroupId, ProductId};

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
}
