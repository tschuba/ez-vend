use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use super::Transaction;

/// Booth/bazaar with all its transactions
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Booth {
    pub id: Uuid,
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub transactions: Vec<Transaction>,
}

impl Booth {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            created_at: now,
            updated_at: now,
            transactions: Vec::new(),
        }
    }

    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.transactions.push(transaction);
        self.updated_at = Utc::now();
    }

    pub fn total_revenue(&self) -> Decimal {
        self.transactions.iter().map(|t| t.amount).sum()
    }

    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }
}
