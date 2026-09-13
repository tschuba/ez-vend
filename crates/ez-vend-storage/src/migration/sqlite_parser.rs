use std::collections::BTreeMap;
use std::str::FromStr;

use rusqlite::{types::Type, Connection};
use rust_decimal::Decimal;

use super::error::MigrationError;
use super::types::{LegacyBooth, LegacyPurchase, LegacyPurchaseItem, LegacyVendor};

pub struct SqliteParser {
    conn: Connection,
}

impl SqliteParser {
    pub fn open_database(bytes: Vec<u8>) -> Result<Self, MigrationError> {
        if bytes.is_empty() {
            return Err(MigrationError::InvalidSqliteFile(
                "database file was empty".to_string(),
            ));
        }

        let mut conn = Connection::open_in_memory()
            .map_err(|err| MigrationError::InvalidSqliteFile(err.to_string()))?;

        let byte_len = bytes.len();
        conn.deserialize_read_exact(
            rusqlite::MAIN_DB,
            std::io::Cursor::new(bytes),
            byte_len,
            true,
        )
        .map_err(|err| MigrationError::InvalidSqliteFile(err.to_string()))?;

        validate_required_tables(&conn)?;

        Ok(Self { conn })
    }

    pub fn parse_booths(&self) -> Result<Vec<LegacyBooth>, MigrationError> {
        let mut stmt = self.conn.prepare(
            "SELECT booth_id, description, date, fees_rounding_step, participation_fee, sales_fee FROM booths ORDER BY date ASC, booth_id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(LegacyBooth {
                booth_id: row.get(0)?,
                description: row.get(1)?,
                date_epoch_millis: row.get(2)?,
                fees_rounding_step: parse_decimal_cell(row.get_ref(3)?)?,
                participation_fee: parse_decimal_cell(row.get_ref(4)?)?,
                sales_fee: parse_decimal_cell(row.get_ref(5)?)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(MigrationError::from)
    }

    pub fn parse_vendors(&self) -> Result<Vec<LegacyVendor>, MigrationError> {
        let mut stmt = self.conn.prepare(
            "SELECT booth_id, vendor_id FROM vendors ORDER BY booth_id ASC, vendor_id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(LegacyVendor {
                booth_id: row.get(0)?,
                vendor_id: row.get(1)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(MigrationError::from)
    }

    pub fn parse_purchases(&self) -> Result<Vec<LegacyPurchase>, MigrationError> {
        let mut purchases_stmt = self.conn.prepare(
            "SELECT booth_id, purchase_id, purchased_on, value FROM purchases ORDER BY purchased_on ASC, purchase_id ASC",
        )?;

        let purchase_rows = purchases_stmt.query_map([], |row| {
            Ok(LegacyPurchase {
                booth_id: row.get(0)?,
                purchase_id: row.get(1)?,
                purchased_on_epoch_millis: row.get(2)?,
                total_value: parse_decimal_cell(row.get_ref(3)?)?,
                items: Vec::new(),
            })
        })?;

        let mut purchases = BTreeMap::new();
        for purchase in purchase_rows {
            let purchase = purchase?;
            purchases.insert(
                (purchase.booth_id.clone(), purchase.purchase_id.clone()),
                purchase,
            );
        }

        let mut items_stmt = self.conn.prepare(
            "SELECT item_id, booth_id, purchase_id, price, purchased_on, vendor_id FROM purchase_items ORDER BY purchased_on ASC, item_id ASC",
        )?;

        let item_rows = items_stmt.query_map([], |row| {
            Ok(LegacyPurchaseItem {
                item_id: row.get(0)?,
                booth_id: row.get(1)?,
                purchase_id: row.get(2)?,
                price: parse_decimal_cell(row.get_ref(3)?)?,
                purchased_on_epoch_millis: row.get(4)?,
                vendor_id: row.get(5)?,
            })
        })?;

        for item in item_rows {
            let item = item?;
            let key = (item.booth_id.clone(), item.purchase_id.clone());
            let purchase = purchases.get_mut(&key).ok_or_else(|| {
                MigrationError::ReplaceFailed(format!(
                    "purchase item {} references missing purchase {} in booth {}",
                    item.item_id, item.purchase_id, item.booth_id
                ))
            })?;
            purchase.items.push(item);
        }

        Ok(purchases.into_values().collect())
    }
}

fn validate_required_tables(conn: &Connection) -> Result<(), MigrationError> {
    for table in ["booths", "vendors", "purchases", "purchase_items"] {
        if !conn.table_exists(None::<&str>, table)? {
            return Err(MigrationError::InvalidSqliteFile(format!(
                "required table '{table}' is missing"
            )));
        }
    }

    Ok(())
}

fn parse_decimal_cell(value: rusqlite::types::ValueRef<'_>) -> rusqlite::Result<Decimal> {
    match value {
        rusqlite::types::ValueRef::Text(text) => {
            Decimal::from_str(std::str::from_utf8(text).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err))
            })?)
            .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(err)))
        }
        rusqlite::types::ValueRef::Integer(integer) => Ok(Decimal::from(integer)),
        rusqlite::types::ValueRef::Real(real) => Decimal::from_str(&real.to_string())
            .map_err(|err| rusqlite::Error::FromSqlConversionFailure(0, Type::Real, Box::new(err))),
        _ => Err(rusqlite::Error::InvalidColumnType(
            0,
            "decimal".to_string(),
            value.data_type(),
        )),
    }
}
