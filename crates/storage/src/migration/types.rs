use rust_decimal::Decimal;

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyBooth {
    pub booth_id: String,
    pub description: String,
    pub date_epoch_millis: i64,
    pub fees_rounding_step: Decimal,
    pub participation_fee: Decimal,
    pub sales_fee: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyVendor {
    pub booth_id: String,
    pub vendor_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyPurchase {
    pub booth_id: String,
    pub purchase_id: String,
    pub purchased_on_epoch_millis: i64,
    pub total_value: Decimal,
    pub items: Vec<LegacyPurchaseItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegacyPurchaseItem {
    pub item_id: String,
    pub booth_id: String,
    pub purchase_id: String,
    pub price: Decimal,
    pub purchased_on_epoch_millis: i64,
    pub vendor_id: String,
}
