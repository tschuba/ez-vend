use chrono::{DateTime, Utc};
use domain::{
    Booth, BoothId, FeeConfig, ItemId, Purchase, PurchaseId, PurchaseItem, Vendor, VendorId,
};
use uuid::Uuid;

use super::error::MigrationError;
use super::types::{LegacyBooth, LegacyPurchase, LegacyPurchaseItem, LegacyVendor};

pub fn map_booths(legacy_booths: &[LegacyBooth]) -> Result<Vec<Booth>, MigrationError> {
    legacy_booths.iter().cloned().map(map_booth).collect()
}

pub fn map_vendors(legacy_vendors: &[LegacyVendor]) -> Result<Vec<Vendor>, MigrationError> {
    legacy_vendors.iter().cloned().map(map_vendor).collect()
}

pub fn map_purchases(legacy_purchases: &[LegacyPurchase]) -> Result<Vec<Purchase>, MigrationError> {
    legacy_purchases.iter().cloned().map(map_purchase).collect()
}

fn map_booth(legacy: LegacyBooth) -> Result<Booth, MigrationError> {
    let id = parse_booth_id(&legacy.booth_id)?;
    let date = DateTime::from_timestamp_millis(legacy.date_epoch_millis)
        .ok_or(MigrationError::InvalidDate {
            field: "booths.date",
            value: legacy.date_epoch_millis,
        })?
        .date_naive();

    let now = Utc::now();
    let fees = FeeConfig {
        participation_fee: legacy.participation_fee,
        sales_fee_percent: legacy.sales_fee,
        rounding_step: legacy.fees_rounding_step,
    };
    fees.validate_ranges()?;

    Ok(Booth {
        id,
        description: legacy.description,
        date,
        fees,
        vendor_id_validation: Default::default(),
        vendor_id_omission_rules: Default::default(),
        keyboard_config: Default::default(),
        amount_stepping: None,
        archived_at: None,
        archived_summary: None,
        created_at: now,
        updated_at: now,
    })
}

fn map_vendor(legacy: LegacyVendor) -> Result<Vendor, MigrationError> {
    Ok(Vendor {
        vendor_id: VendorId::from(legacy.vendor_id),
        booth_id: parse_booth_id(&legacy.booth_id)?,
        created_at: Utc::now(),
        payout_correction: None,
        payout_correction_note: None,
    })
}

fn map_purchase(legacy: LegacyPurchase) -> Result<Purchase, MigrationError> {
    if legacy.items.is_empty() {
        return Ok(Purchase {
            id: parse_purchase_id(&legacy.purchase_id)?,
            booth_id: parse_booth_id(&legacy.booth_id)?,
            items: Vec::new(),
            timestamp: DateTime::from_timestamp_millis(legacy.purchased_on_epoch_millis).ok_or(
                MigrationError::InvalidTimestamp {
                    field: "purchases.purchased_on",
                    value: legacy.purchased_on_epoch_millis,
                },
            )?,
            note: None,
        });
    }

    let items = legacy
        .items
        .into_iter()
        .map(map_purchase_item)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Purchase {
        id: parse_purchase_id(&legacy.purchase_id)?,
        booth_id: parse_booth_id(&legacy.booth_id)?,
        items,
        timestamp: DateTime::from_timestamp_millis(legacy.purchased_on_epoch_millis).ok_or(
            MigrationError::InvalidTimestamp {
                field: "purchases.purchased_on",
                value: legacy.purchased_on_epoch_millis,
            },
        )?,
        note: None,
    })
}

fn map_purchase_item(legacy: LegacyPurchaseItem) -> Result<PurchaseItem, MigrationError> {
    let id = parse_item_id(&legacy.item_id)?;
    let item = PurchaseItem::new(legacy.price, VendorId::from(legacy.vendor_id))?;

    Ok(PurchaseItem {
        id,
        amount: item.amount,
        vendor_id: item.vendor_id,
    })
}

fn parse_booth_id(value: &str) -> Result<BoothId, MigrationError> {
    Uuid::parse_str(value)
        .map(BoothId::from)
        .map_err(|_| MigrationError::InvalidUuid {
            field: "booth_id",
            value: value.to_string(),
        })
}

fn parse_purchase_id(value: &str) -> Result<PurchaseId, MigrationError> {
    Uuid::parse_str(value)
        .map(PurchaseId::from)
        .map_err(|_| MigrationError::InvalidUuid {
            field: "purchase_id",
            value: value.to_string(),
        })
}

fn parse_item_id(value: &str) -> Result<ItemId, MigrationError> {
    Uuid::parse_str(value)
        .map(ItemId::from)
        .map_err(|_| MigrationError::InvalidUuid {
            field: "item_id",
            value: value.to_string(),
        })
}
