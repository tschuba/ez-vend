use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::{Booth, Vendor, VendorId};
use std::collections::HashMap;

/// Report data for an event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventReport {
    pub event_name: String,
    pub generated_at: DateTime<Utc>,
    pub total_revenue: Decimal,
    pub transaction_count: usize,
    pub vendor_reports: Vec<VendorReport>,
}

/// Report data for a single vendor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VendorReport {
    pub vendor_id: VendorId,
    pub total: Decimal,
    pub item_count: usize,
    pub commission_rate: Decimal,
    pub commission_amount: Decimal,
    pub payout_amount: Decimal,
}

impl EventReport {
    pub fn from_booth(booth: &Booth, commission_rate: Decimal) -> Self {
        // Group transactions by vendor
        let mut vendor_map: HashMap<VendorId, Vendor> = HashMap::new();
        
        for transaction in &event.transactions {
            vendor_map
                .entry(transaction.vendor_id.clone())
                .or_insert_with(|| Vendor::new(transaction.vendor_id.clone()))
                .add_transaction(transaction.amount);
        }

        // Convert to sorted vector
        let mut vendors: Vec<_> = vendor_map.into_values().collect();
        vendors.sort_by(|a, b| a.id.cmp(&b.id));

        // Create vendor reports
        let vendor_reports: Vec<VendorReport> = vendors
            .into_iter()
            .map(|v| {
                let commission_amount = v.total * commission_rate;
                VendorReport {
                    vendor_id: v.id,
                    total: v.total,
                    item_count: v.item_count,
                    commission_rate,
                    commission_amount,
                    payout_amount: v.total - commission_amount,
                }
            })
            .collect();

        EventReport {
            event_name: event.name.clone(),
            generated_at: Utc::now(),
            total_revenue: event.total_revenue(),
            transaction_count: event.transaction_count(),
            vendor_reports,
        }
    }
}
