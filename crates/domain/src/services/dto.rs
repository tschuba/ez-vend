use crate::models::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// An item in a vendor report with its associated transaction ID
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VendorReportItem {
    pub transaction_id: PurchaseId,
    pub item: PurchaseItem,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckoutItem {
    pub vendor: VendorId,
    pub price: Decimal,
    pub purchased_on: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkout {
    pub booth: BoothId,
    pub items: Vec<CheckoutItem>,
    pub print_receipt: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChargedFees {
    pub participation_fee: Decimal,
    pub sales_fee: Decimal,
}

impl ChargedFees {
    pub fn total(&self) -> Decimal {
        self.participation_fee + self.sales_fee
    }
}

/// Result of calculating vendor payout with rounding applied
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VendorPayout {
    pub gross_sales: Decimal,
    pub fees_due: Decimal,
    pub net_payout: Decimal,
}

impl VendorPayout {
    pub fn participation_fee(&self, config: &ChargingConfig) -> Decimal {
        config.participation_fee
    }

    pub fn sales_fee(&self, config: &ChargingConfig) -> Decimal {
        self.fees_due - config.participation_fee
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChargingConfig {
    pub participation_fee: Decimal,
    pub sales_fee: Decimal,
    pub rounding_step: Decimal,
}

impl ChargingConfig {
    pub fn from_booth(booth: &Booth) -> Self {
        Self {
            participation_fee: booth.fees.participation_fee,
            sales_fee: booth.fees.sales_fee_percent,
            rounding_step: booth.fees.rounding_step,
        }
    }

    /// Round a value to the nearest multiple of the rounding step
    fn round_to_step(&self, value: Decimal) -> Decimal {
        if self.rounding_step == Decimal::ZERO {
            // If rounding step is 0, just round to 2 decimal places
            return value
                .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
        }

        // Round to nearest multiple of rounding_step
        // Formula: round(value / step) * step
        let divided = value / self.rounding_step;
        let rounded =
            divided.round_dp_with_strategy(0, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
        rounded * self.rounding_step
    }

    #[deprecated(
        since = "0.1.0",
        note = "Use calculate_payout() for consistent fee calculations"
    )]
    pub fn calculate_fees(&self, value: Decimal) -> ChargedFees {
        let sales_fee_raw = self.sales_fee * value / dec!(100);
        let sales_fee = self.round_to_step(sales_fee_raw);

        ChargedFees {
            participation_fee: self.participation_fee,
            sales_fee,
        }
    }

    /// Calculate vendor payout with rounding applied to net payout
    /// The net payout is rounded to the rounding step, and fees are adjusted to match
    pub fn calculate_payout(&self, gross_sales: Decimal) -> VendorPayout {
        // Calculate the theoretical net payout before rounding
        let sales_fee_raw = self.sales_fee * gross_sales / dec!(100);
        let theoretical_net = gross_sales - self.participation_fee - sales_fee_raw;

        // Round the net payout to the rounding step
        let net_payout = self.round_to_step(theoretical_net);

        // Calculate actual fees as the difference
        let fees_due = gross_sales - net_payout;

        VendorPayout {
            gross_sales,
            fees_due,
            net_payout,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceInput {
    pub total_sales_amount: Decimal,
    pub charging_config: ChargingConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BalanceOutput {
    pub total_revenue: Decimal,
    pub charged_fees: ChargedFees,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VendorReportInput {
    pub vendors: Vec<VendorId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VendorReportData {
    pub vendor: Vendor,
    pub booth: Booth,
    pub items: Vec<VendorReportItem>,
    pub sales_sum: Decimal,
    pub participation_fee: Decimal,
    pub sales_fee: Decimal,
    pub base_total_revenue: Decimal,
    pub payout_correction: Decimal,
    pub payout_correction_note: Option<String>,
    pub total_revenue: Decimal,
}

impl PartialOrd for VendorReportData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for VendorReportData {}

impl Ord for VendorReportData {
    fn cmp(&self, other: &Self) -> Ordering {
        // Use VendorId's built-in smart sorting (handles numeric vs alphanumeric)
        self.vendor.vendor_id.cmp(&other.vendor.vendor_id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeData {
    pub booth: Booth,
    pub vendors: Vec<Vendor>,
    pub purchases: Vec<Purchase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeReceiver {
    pub name: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeSubscription {
    pub id: String,
    pub booth: BoothId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rounding_step_half_euro() {
        let config = ChargingConfig {
            participation_fee: dec!(1.00),
            sales_fee: dec!(15.0), // 15%
            rounding_step: dec!(0.50),
        };

        // 15% of 23.50 = 3.525 -> should round to 3.50
        let payout = config.calculate_payout(dec!(23.50));
        assert_eq!(payout.sales_fee(&config), dec!(3.50));

        // 15% of 24.00 = 3.60 -> should round to 3.50
        let payout = config.calculate_payout(dec!(24.00));
        assert_eq!(payout.sales_fee(&config), dec!(3.50));

        // Payout-authoritative fee component after net rounding lands at 3.50
        let payout = config.calculate_payout(dec!(25.00));
        assert_eq!(payout.sales_fee(&config), dec!(3.50));

        // 15% of 27.00 = 4.05 -> should round to 4.00
        let payout = config.calculate_payout(dec!(27.00));
        assert_eq!(payout.sales_fee(&config), dec!(4.00));
    }

    #[test]
    fn test_rounding_step_quarter_euro() {
        let config = ChargingConfig {
            participation_fee: dec!(1.00),
            sales_fee: dec!(10.0), // 10%
            rounding_step: dec!(0.25),
        };

        // 10% of 23.00 = 2.30 -> should round to 2.25
        let payout = config.calculate_payout(dec!(23.00));
        assert_eq!(payout.sales_fee(&config), dec!(2.25));

        // 10% of 24.00 = 2.40 -> should round to 2.50
        let payout = config.calculate_payout(dec!(24.00));
        assert_eq!(payout.sales_fee(&config), dec!(2.50));

        // 10% of 26.75 = 2.675 -> should round to 2.75
        let payout = config.calculate_payout(dec!(26.75));
        assert_eq!(payout.sales_fee(&config), dec!(2.75));
    }

    #[test]
    fn test_rounding_step_one_euro() {
        let config = ChargingConfig {
            participation_fee: dec!(5.00),
            sales_fee: dec!(15.0), // 15%
            rounding_step: dec!(1.00),
        };

        // Payout-authoritative fee component after net rounding lands at 1.00
        let payout = config.calculate_payout(dec!(10.00));
        assert_eq!(payout.sales_fee(&config), dec!(1.00));

        // 15% of 20.00 = 3.00 -> should stay 3.00
        let payout = config.calculate_payout(dec!(20.00));
        assert_eq!(payout.sales_fee(&config), dec!(3.00));

        // 15% of 23.00 = 3.45 -> should round to 3.00
        let payout = config.calculate_payout(dec!(23.00));
        assert_eq!(payout.sales_fee(&config), dec!(3.00));

        // 15% of 27.00 = 4.05 -> should round to 4.00
        let payout = config.calculate_payout(dec!(27.00));
        assert_eq!(payout.sales_fee(&config), dec!(4.00));
    }

    #[test]
    fn test_calculate_fees_vs_calculate_payout_consistency() {
        let config = ChargingConfig {
            participation_fee: dec!(10.00),
            sales_fee: dec!(15.0),
            rounding_step: dec!(0.50),
        };

        for gross_sales in [
            dec!(100.00),
            dec!(518.11),
            dec!(1234.56),
            dec!(0.50),
            dec!(99999.99),
        ] {
            let payout = config.calculate_payout(gross_sales);
            let theoretical_sales_fee = (config.sales_fee * gross_sales / dec!(100))
                .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
            let payout_sales_fee = payout.sales_fee(&config);
            let diff = (theoretical_sales_fee - payout_sales_fee).abs();
            assert!(diff <= config.rounding_step);
        }
    }

    #[test]
    fn test_vendor_fee_sum_matches_components() {
        let config = ChargingConfig {
            participation_fee: dec!(10.00),
            sales_fee: dec!(15.0),
            rounding_step: dec!(0.50),
        };

        let mut total_fees_due = Decimal::ZERO;
        let mut total_participation = Decimal::ZERO;
        let mut total_sales_fees = Decimal::ZERO;

        for sales in [dec!(100.00), dec!(250.50), dec!(518.11), dec!(75.25)] {
            let payout = config.calculate_payout(sales);
            total_fees_due += payout.fees_due;
            total_participation += config.participation_fee;
            total_sales_fees += payout.fees_due - config.participation_fee;
        }

        assert_eq!(total_fees_due, total_participation + total_sales_fees);
    }

    #[test]
    fn test_rounding_step_zero_uses_cent_precision() {
        let config = ChargingConfig {
            participation_fee: dec!(1.00),
            sales_fee: dec!(15.0), // 15%
            rounding_step: dec!(0.00),
        };

        // 15% of 23.33 = 3.4995 -> should round to 3.50 (2 decimal places)
        let payout = config.calculate_payout(dec!(23.33));
        assert_eq!(payout.sales_fee(&config), dec!(3.50));

        // 15% of 23.34 = 3.501 -> should round to 3.50 (2 decimal places)
        let payout = config.calculate_payout(dec!(23.34));
        assert_eq!(payout.sales_fee(&config), dec!(3.50));
    }

    #[test]
    fn test_participation_fee_not_rounded() {
        let config = ChargingConfig {
            participation_fee: dec!(1.50),
            sales_fee: dec!(15.0),
            rounding_step: dec!(0.50),
        };

        // Participation fee should always be the configured amount
        let payout = config.calculate_payout(dec!(100.00));
        assert_eq!(payout.participation_fee(&config), dec!(1.50));
        // Revenue share: 15% of 100 = 15.00 (already a multiple of 0.50)
        assert_eq!(payout.sales_fee(&config), dec!(15.00));
    }

    #[test]
    fn test_payout_rounding_half_euro() {
        let config = ChargingConfig {
            participation_fee: dec!(10.00),
            sales_fee: dec!(15.0), // 15%
            rounding_step: dec!(0.50),
        };

        // Example: gross_sales = 518.11
        // Theoretical net = 518.11 - 10.00 - (15% of 518.11) = 518.11 - 10.00 - 77.7165 = 430.3935
        // Rounded net = 430.50
        // Actual fees = 518.11 - 430.50 = 87.61
        let payout = config.calculate_payout(dec!(518.11));
        assert_eq!(payout.gross_sales, dec!(518.11));
        assert_eq!(payout.net_payout, dec!(430.50));
        assert_eq!(payout.fees_due, dec!(87.61));
    }

    #[test]
    fn test_payout_rounding_one_euro() {
        let config = ChargingConfig {
            participation_fee: dec!(5.00),
            sales_fee: dec!(10.0), // 10%
            rounding_step: dec!(1.00),
        };

        // Example: gross_sales = 100.00
        // Theoretical net = 100.00 - 5.00 - (10% of 100.00) = 100.00 - 5.00 - 10.00 = 85.00
        // Rounded net = 85.00 (already rounded)
        // Actual fees = 100.00 - 85.00 = 15.00
        let payout = config.calculate_payout(dec!(100.00));
        assert_eq!(payout.gross_sales, dec!(100.00));
        assert_eq!(payout.net_payout, dec!(85.00));
        assert_eq!(payout.fees_due, dec!(15.00));

        // Example: gross_sales = 100.50
        // Theoretical net = 100.50 - 5.00 - (10% of 100.50) = 100.50 - 5.00 - 10.05 = 85.45
        // Rounded net = 85.00
        // Actual fees = 100.50 - 85.00 = 15.50
        let payout = config.calculate_payout(dec!(100.50));
        assert_eq!(payout.gross_sales, dec!(100.50));
        assert_eq!(payout.net_payout, dec!(85.00));
        assert_eq!(payout.fees_due, dec!(15.50));
    }

    #[test]
    fn test_payout_rounding_zero_uses_cent_precision() {
        let config = ChargingConfig {
            participation_fee: dec!(2.00),
            sales_fee: dec!(12.5), // 12.5%
            rounding_step: dec!(0.00),
        };

        // Example: gross_sales = 50.00
        // Theoretical net = 50.00 - 2.00 - (12.5% of 50.00) = 50.00 - 2.00 - 6.25 = 41.75
        // Rounded net = 41.75 (2 decimal places)
        // Actual fees = 50.00 - 41.75 = 8.25
        let payout = config.calculate_payout(dec!(50.00));
        assert_eq!(payout.gross_sales, dec!(50.00));
        assert_eq!(payout.net_payout, dec!(41.75));
        assert_eq!(payout.fees_due, dec!(8.25));

        // Example with more complex calculation
        // Theoretical net = 47.33 - 2.00 - (12.5% of 47.33) = 47.33 - 2.00 - 5.91625 = 39.41375
        // Rounded net = 39.41 (2 decimal places)
        // Actual fees = 47.33 - 39.41 = 7.92
        let payout = config.calculate_payout(dec!(47.33));
        assert_eq!(payout.gross_sales, dec!(47.33));
        assert_eq!(payout.net_payout, dec!(39.41));
        assert_eq!(payout.fees_due, dec!(7.92));
    }
}
