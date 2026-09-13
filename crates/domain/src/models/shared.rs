use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// Type-safe ID types using macro to reduce boilerplate
macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }

            pub fn as_str(&self) -> String {
                self.0.to_string()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

// Define type-safe IDs
define_id!(BoothId);
define_id!(PurchaseId);
define_id!(ItemId);

// Legacy Id type for backward compatibility during migration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Id(String);

impl Id {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Vendor identifier - supports both numeric and text IDs with smart sorting
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VendorId(String);

impl VendorId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if the vendor ID is numeric-only
    pub fn is_numeric(&self) -> bool {
        self.0.chars().all(|c| c.is_ascii_digit())
    }

    /// Get numeric value if ID is numeric
    pub fn as_number(&self) -> Option<u64> {
        self.0.parse().ok()
    }
}

impl fmt::Display for VendorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for VendorId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for VendorId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Smart ordering for vendor IDs: numeric IDs sort numerically, text IDs alphabetically
impl Ord for VendorId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.as_number(), other.as_number()) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => self.0.cmp(&other.0),
        }
    }
}

impl PartialOrd for VendorId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_id_numeric_sorting() {
        let mut ids = [
            VendorId::new("100".to_string()),
            VendorId::new("2".to_string()),
            VendorId::new("30".to_string()),
        ];
        ids.sort();
        assert_eq!(ids[0].as_str(), "2");
        assert_eq!(ids[1].as_str(), "30");
        assert_eq!(ids[2].as_str(), "100");
    }

    #[test]
    fn test_vendor_id_mixed_sorting() {
        let mut ids = [
            VendorId::new("A10".to_string()),
            VendorId::new("5".to_string()),
            VendorId::new("B2".to_string()),
            VendorId::new("15".to_string()),
        ];
        ids.sort();
        // Numeric IDs come first, then alphabetic
        assert_eq!(ids[0].as_str(), "5");
        assert_eq!(ids[1].as_str(), "15");
        assert_eq!(ids[2].as_str(), "A10");
        assert_eq!(ids[3].as_str(), "B2");
    }
}
