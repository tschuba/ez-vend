use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::VendorId;

/// A single transaction/sale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Uuid,
    pub vendor_id: VendorId,
    pub amount: Decimal,
    pub timestamp: DateTime<Utc>,
}

impl Transaction {
    pub fn new(vendor_id: VendorId, amount: Decimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            vendor_id,
            amount,
            timestamp: Utc::now(),
        }
    }
}
