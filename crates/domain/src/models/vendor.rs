use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::shared::{BoothId, VendorId};

/// Represents a vendor at the booth
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vendor {
    #[serde(rename = "id")]
    pub vendor_id: VendorId,
    pub booth_id: BoothId,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub payout_correction: Option<Decimal>,
    #[serde(default)]
    pub payout_correction_note: Option<String>,
}

impl Vendor {
    pub fn new(vendor_id: VendorId, booth_id: BoothId) -> Self {
        Self {
            vendor_id,
            booth_id,
            created_at: Utc::now(),
            payout_correction: None,
            payout_correction_note: None,
        }
    }
}

/// Summary statistics for a vendor across all booths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorSummary {
    pub vendor_id: VendorId,
    pub total_revenue: Decimal,
    pub purchase_count: usize,
}

impl VendorSummary {
    pub fn new(vendor_id: VendorId) -> Self {
        Self {
            vendor_id,
            total_revenue: Decimal::ZERO,
            purchase_count: 0,
        }
    }

    pub fn add_purchase(&mut self, amount: Decimal) {
        self.total_revenue += amount;
        self.purchase_count += 1;
    }
}
