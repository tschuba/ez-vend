use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoothType {
    DirectSale,
    ThirdPartySale,
}

impl Default for BoothType {
    fn default() -> Self {
        Self::ThirdPartySale
    }
}
