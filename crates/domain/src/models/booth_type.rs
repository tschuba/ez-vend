use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoothType {
    DirectSale,
    #[default]
    ThirdPartySale,
}
