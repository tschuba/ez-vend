use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Vendor ID that supports both numeric and alphanumeric identifiers
/// with natural sorting (numeric IDs sort numerically)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VendorId(String);

impl VendorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if the ID is numeric-only
    fn is_numeric(&self) -> bool {
        self.0.chars().all(|c| c.is_ascii_digit())
    }

    /// Parse as number if numeric
    fn as_number(&self) -> Option<u64> {
        if self.is_numeric() {
            self.0.parse().ok()
        } else {
            None
        }
    }
}

impl PartialOrd for VendorId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VendorId {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.as_number(), other.as_number()) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less, // numeric before alphanumeric
            (None, Some(_)) => Ordering::Greater,
            (None, None) => self.0.cmp(&other.0),
        }
    }
}

/// Vendor with their transaction total and item count
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vendor {
    pub id: VendorId,
    pub total: Decimal,
    pub item_count: usize,
}

impl Vendor {
    pub fn new(id: VendorId) -> Self {
        Self {
            id,
            total: Decimal::ZERO,
            item_count: 0,
        }
    }

    pub fn add_transaction(&mut self, amount: Decimal) {
        self.total += amount;
        self.item_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_id_numeric_sorting() {
        let mut ids = [
            VendorId::new("100"),
            VendorId::new("2"),
            VendorId::new("30"),
            VendorId::new("1"),
        ];
        ids.sort();
        assert_eq!(ids[0].as_str(), "1");
        assert_eq!(ids[1].as_str(), "2");
        assert_eq!(ids[2].as_str(), "30");
        assert_eq!(ids[3].as_str(), "100");
    }

    #[test]
    fn test_vendor_id_mixed_sorting() {
        let mut ids = [
            VendorId::new("A10"),
            VendorId::new("100"),
            VendorId::new("2"),
            VendorId::new("B5"),
        ];
        ids.sort();
        // Numeric IDs come first, then alphanumeric
        assert_eq!(ids[0].as_str(), "2");
        assert_eq!(ids[1].as_str(), "100");
        assert_eq!(ids[2].as_str(), "A10");
        assert_eq!(ids[3].as_str(), "B5");
    }
}
