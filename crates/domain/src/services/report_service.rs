use crate::error::{DomainError, DomainResult};
use crate::models::{BoothId, BoothSummary, Purchase, VendorBoothSummary, VendorId};
use crate::repositories::{BoothRepository, PurchaseRepository, VendorRepository};
use crate::services::dto::{ChargingConfig, VendorReportData, VendorReportItem};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

/// Service for generating reports and analytics
pub struct ReportService<PR: PurchaseRepository, BR: BoothRepository, VR: VendorRepository> {
    purchase_repository: PR,
    booth_repository: BR,
    vendor_repository: VR,
}

impl<PR: PurchaseRepository, BR: BoothRepository, VR: VendorRepository> ReportService<PR, BR, VR> {
    pub fn new(purchase_repository: PR, booth_repository: BR, vendor_repository: VR) -> Self {
        Self {
            purchase_repository,
            booth_repository,
            vendor_repository,
        }
    }

    /// Generate a comprehensive summary for a booth
    pub async fn generate_booth_summary(
        &self,
        booth_id: &BoothId,
        date_range: Option<DateRange>,
    ) -> DomainResult<BoothSummary> {
        // Get booth information
        let booth = self
            .booth_repository
            .find_by_id(booth_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Booth {} not found", booth_id)))?;

        // Get all purchases for the booth
        let mut purchases = self.purchase_repository.find_by_booth(booth_id).await?;

        // Apply date range filter if specified
        if let Some(range) = &date_range {
            purchases = Self::filter_by_date_range(purchases, range);
        }

        // Load vendor metadata (including manual payout corrections)
        let vendors = self.vendor_repository.find_by_booth(booth_id).await?;
        let correction_by_vendor: HashMap<VendorId, Decimal> = vendors
            .into_iter()
            .map(|vendor| {
                (
                    vendor.vendor_id,
                    vendor.payout_correction.unwrap_or(Decimal::ZERO),
                )
            })
            .collect();

        // Group purchase items by vendor
        let mut vendor_items: HashMap<VendorId, Vec<(&Purchase, &crate::models::PurchaseItem)>> =
            HashMap::new();
        for purchase in &purchases {
            for item in &purchase.items {
                vendor_items
                    .entry(item.vendor_id.clone())
                    .or_default()
                    .push((purchase, item));
            }
        }

        // Calculate charging config
        let charging_config = ChargingConfig::from_booth(&booth);

        // Generate vendor summaries
        let mut vendor_summaries: Vec<VendorBoothSummary> = Vec::new();
        for (vendor_id, vendor_item_list) in vendor_items.iter() {
            let gross_sales: Decimal = vendor_item_list.iter().map(|(_, item)| item.amount).sum();

            let payout = charging_config.calculate_payout(gross_sales);
            let correction = correction_by_vendor
                .get(vendor_id)
                .copied()
                .unwrap_or(Decimal::ZERO);
            let net_payout = payout.net_payout + correction;
            let fees_due = payout.gross_sales - net_payout;

            // Count total items for this vendor
            let item_count: usize = vendor_item_list.len();

            vendor_summaries.push(VendorBoothSummary {
                vendor_id: vendor_id.clone(),
                gross_sales: payout.gross_sales,
                fees_due,
                net_payout,
                item_count,
            });
        }

        // Sort vendor summaries by vendor_id (uses smart sorting)
        vendor_summaries.sort_by(|a, b| a.vendor_id.cmp(&b.vendor_id));

        // Calculate totals
        let total_revenue: Decimal = vendor_summaries.iter().map(|v| v.gross_sales).sum();
        let total_purchases = purchases.len();
        let total_items: usize = vendor_summaries.iter().map(|v| v.item_count).sum();
        let unique_vendors = vendor_items.len();

        // Calculate booth revenue metrics
        let total_participation_fees: Decimal = vendor_summaries
            .iter()
            .map(|_| charging_config.participation_fee)
            .sum();
        let total_sales_fees: Decimal = vendor_summaries
            .iter()
            .map(|v| v.fees_due - charging_config.participation_fee)
            .sum();
        let total_booth_revenue = total_participation_fees + total_sales_fees;

        debug_assert_eq!(
            vendor_summaries.iter().map(|v| v.fees_due).sum::<Decimal>(),
            total_booth_revenue,
            "sum of vendor fees must match booth revenue"
        );

        Ok(BoothSummary {
            booth_id: *booth_id,
            total_revenue,
            total_purchases,
            total_items,
            unique_vendors,
            vendor_summaries,
            participation_fee: booth.fees.participation_fee,
            sales_fee_percent: booth.fees.sales_fee_percent,
            total_participation_fees,
            total_sales_fees,
            total_booth_revenue,
        })
    }

    /// Generate a detailed report for a specific vendor in a booth
    pub async fn generate_vendor_report(
        &self,
        booth_id: &BoothId,
        vendor_id: &VendorId,
        date_range: Option<DateRange>,
    ) -> DomainResult<VendorReportData> {
        // Get booth information
        let booth = self
            .booth_repository
            .find_by_id(booth_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Booth {} not found", booth_id)))?;

        // Get vendor information
        let vendor = self
            .vendor_repository
            .find_by_id(booth_id, vendor_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!(
                    "Vendor {} not found in booth {}",
                    vendor_id, booth_id
                ))
            })?;

        // Get all purchases for the vendor
        let mut purchases = self
            .purchase_repository
            .find_by_vendor(booth_id, vendor_id)
            .await?;

        // Apply date range filter if specified
        if let Some(range) = &date_range {
            purchases = Self::filter_by_date_range(purchases, range);
        }

        // Collect all items from purchases with their transaction IDs
        // Filter to only include items for this specific vendor
        let items: Vec<VendorReportItem> = purchases
            .iter()
            .flat_map(|p| {
                p.items
                    .iter()
                    .filter(|item| &item.vendor_id == vendor_id)
                    .map(|item| VendorReportItem {
                        transaction_id: p.id,
                        item: item.clone(),
                        timestamp: p.timestamp,
                    })
            })
            .collect();

        // Calculate totals
        let sales_sum: Decimal = items
            .iter()
            .map(|report_item| report_item.item.amount)
            .sum();

        // Calculate payout with rounding applied to net payout
        let charging_config = ChargingConfig::from_booth(&booth);
        let payout = charging_config.calculate_payout(sales_sum);
        let payout_correction = vendor.payout_correction.unwrap_or(Decimal::ZERO);
        let base_total_revenue = payout.net_payout;
        let total_revenue = base_total_revenue + payout_correction;

        Ok(VendorReportData {
            payout_correction_note: vendor.payout_correction_note.clone(),
            vendor,
            booth,
            items,
            sales_sum: payout.gross_sales,
            participation_fee: charging_config.participation_fee,
            sales_fee: payout.fees_due - charging_config.participation_fee,
            base_total_revenue,
            payout_correction,
            total_revenue,
        })
    }

    /// Generate reports for multiple vendors in a booth
    pub async fn generate_vendor_reports(
        &self,
        booth_id: &BoothId,
        vendor_ids: Vec<VendorId>,
        date_range: Option<DateRange>,
    ) -> DomainResult<Vec<VendorReportData>> {
        let mut reports = Vec::new();

        for vendor_id in vendor_ids {
            let report = self
                .generate_vendor_report(booth_id, &vendor_id, date_range.clone())
                .await?;
            reports.push(report);
        }

        // Sort reports by vendor_id (uses smart sorting through Ord implementation)
        reports.sort();

        Ok(reports)
    }

    /// Get all vendors who have made purchases in a booth
    pub async fn get_active_vendors(
        &self,
        booth_id: &BoothId,
        date_range: Option<DateRange>,
    ) -> DomainResult<Vec<VendorId>> {
        let mut purchases = self.purchase_repository.find_by_booth(booth_id).await?;

        // Apply date range filter if specified
        if let Some(range) = &date_range {
            purchases = Self::filter_by_date_range(purchases, range);
        }

        // Collect unique vendor IDs from all items across all purchases
        let vendor_ids: HashSet<VendorId> = purchases
            .into_iter()
            .flat_map(|p| p.items.into_iter().map(|item| item.vendor_id))
            .collect();

        // Convert to sorted vector
        let mut vendor_ids: Vec<VendorId> = vendor_ids.into_iter().collect();
        vendor_ids.sort();

        Ok(vendor_ids)
    }

    /// Filter purchases by date range
    fn filter_by_date_range(purchases: Vec<Purchase>, range: &DateRange) -> Vec<Purchase> {
        purchases
            .into_iter()
            .filter(|p| {
                let timestamp = p.timestamp;
                if let Some(start) = range.start {
                    if timestamp < start {
                        return false;
                    }
                }
                if let Some(end) = range.end {
                    if timestamp > end {
                        return false;
                    }
                }
                true
            })
            .collect()
    }
}

/// Date range filter for reports
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

impl DateRange {
    pub fn new(start: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>) -> Self {
        Self { start, end }
    }

    /// Create a date range for all time
    pub fn all_time() -> Self {
        Self {
            start: None,
            end: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Booth, FeeConfig, PurchaseItem, Vendor};
    use crate::test_support::{MockBoothRepository, MockPurchaseRepository, MockVendorRepository};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn create_test_booth() -> Booth {
        Booth {
            id: BoothId::new(),
            description: "Test Booth".to_string(),
            date: NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            fees: FeeConfig {
                participation_fee: dec!(5.00),
                sales_fee_percent: dec!(10.0),
                rounding_step: dec!(0.50),
            },
            vendor_id_validation: crate::models::VendorIdValidation::default(),
            vendor_id_omission_rules: crate::models::VendorIdOmissionRules::empty(),
            keyboard_config: crate::models::CheckoutKeyboardConfig::default(),
            amount_stepping: None,
            archived_at: None,
            archived_summary: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_vendor(booth_id: &BoothId, vendor_id: &str) -> Vendor {
        Vendor {
            vendor_id: VendorId::new(vendor_id.to_string()),
            booth_id: *booth_id,
            created_at: Utc::now(),
            payout_correction: None,
            payout_correction_note: None,
        }
    }

    #[tokio::test]
    async fn test_generate_booth_summary() {
        let booth = create_test_booth();
        let vendor1 = create_test_vendor(&booth.id, "1");
        let vendor2 = create_test_vendor(&booth.id, "2");

        let purchase_repo = MockPurchaseRepository::new();
        let booth_repo = MockBoothRepository::new();
        let vendor_repo = MockVendorRepository::new();

        booth_repo.add(booth.clone());
        vendor_repo.add(vendor1.clone());
        vendor_repo.add(vendor2.clone());

        // Add some purchases
        let purchase1 = Purchase::new(
            booth.id,
            vec![
                PurchaseItem::new(dec!(10.00), vendor1.vendor_id.clone()).unwrap(),
                PurchaseItem::new(dec!(5.00), vendor1.vendor_id.clone()).unwrap(),
            ],
        )
        .unwrap();
        let purchase2 = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(20.00), vendor2.vendor_id.clone()).unwrap()],
        )
        .unwrap();

        purchase_repo.add(purchase1);
        purchase_repo.add(purchase2);

        let service = ReportService::new(purchase_repo, booth_repo, vendor_repo);
        let summary = service
            .generate_booth_summary(&booth.id, None)
            .await
            .unwrap();

        assert_eq!(summary.total_revenue, dec!(35.00)); // 15.00 + 20.00
        assert_eq!(summary.total_purchases, 2);
        assert_eq!(summary.unique_vendors, 2);
        assert_eq!(summary.vendor_summaries.len(), 2);
        assert_eq!(summary.participation_fee, dec!(5.00));
        assert_eq!(summary.sales_fee_percent, dec!(10.0));
        assert_eq!(summary.total_participation_fees, dec!(10.00));
        assert_eq!(summary.total_sales_fees, dec!(3.50));
        assert_eq!(summary.total_booth_revenue, dec!(13.50));

        // Check vendor1 summary
        let v1_summary = summary
            .vendor_summaries
            .iter()
            .find(|v| v.vendor_id == vendor1.vendor_id)
            .unwrap();
        assert_eq!(v1_summary.gross_sales, dec!(15.00));
        assert_eq!(v1_summary.item_count, 2); // 2 items in the purchase
                                              // Fees: 5.00 participation + 1.50 sales (10% of 15.00) = 6.50
        assert_eq!(v1_summary.fees_due, dec!(6.50));
        assert_eq!(v1_summary.net_payout, dec!(8.50)); // 15.00 - 6.50
    }

    #[tokio::test]
    async fn test_generate_vendor_report() {
        let booth = create_test_booth();
        let vendor = create_test_vendor(&booth.id, "1");

        let purchase_repo = MockPurchaseRepository::new();
        let booth_repo = MockBoothRepository::new();
        let vendor_repo = MockVendorRepository::new();

        booth_repo.add(booth.clone());
        vendor_repo.add(vendor.clone());

        // Add purchases for vendor
        let purchase1 = Purchase::new(
            booth.id,
            vec![
                PurchaseItem::new(dec!(10.00), vendor.vendor_id.clone()).unwrap(),
                PurchaseItem::new(dec!(5.00), vendor.vendor_id.clone()).unwrap(),
            ],
        )
        .unwrap();
        let purchase2 = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(8.00), vendor.vendor_id.clone()).unwrap()],
        )
        .unwrap();

        purchase_repo.add(purchase1);
        purchase_repo.add(purchase2);

        let service = ReportService::new(purchase_repo, booth_repo, vendor_repo);
        let report = service
            .generate_vendor_report(&booth.id, &vendor.vendor_id, None)
            .await
            .unwrap();

        assert_eq!(report.sales_sum, dec!(23.00)); // 10 + 5 + 8
        assert_eq!(report.participation_fee, dec!(5.00));
        assert_eq!(report.sales_fee, dec!(2.50)); // 10% of 23.00 = 2.30, rounded to nearest 0.50 = 2.50
        assert_eq!(report.base_total_revenue, dec!(15.50)); // 23.00 - 5.00 - 2.50
        assert_eq!(report.payout_correction, dec!(0.00));
        assert_eq!(report.total_revenue, dec!(15.50));
        assert_eq!(report.items.len(), 3);
    }

    #[tokio::test]
    async fn test_get_active_vendors() {
        let booth = create_test_booth();
        let vendor1 = create_test_vendor(&booth.id, "1");
        let _vendor2 = create_test_vendor(&booth.id, "10");
        let vendor3 = create_test_vendor(&booth.id, "2");

        let purchase_repo = MockPurchaseRepository::new();
        let booth_repo = MockBoothRepository::new();
        let vendor_repo = MockVendorRepository::new();

        booth_repo.add(booth.clone());

        // Add purchases (vendor2 has no purchases)
        let purchase1 = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(10.00), vendor1.vendor_id.clone()).unwrap()],
        )
        .unwrap();
        let purchase2 = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(20.00), vendor3.vendor_id.clone()).unwrap()],
        )
        .unwrap();

        purchase_repo.add(purchase1);
        purchase_repo.add(purchase2);

        let service = ReportService::new(purchase_repo, booth_repo, vendor_repo);
        let active_vendors = service.get_active_vendors(&booth.id, None).await.unwrap();

        assert_eq!(active_vendors.len(), 2);
        // Should be sorted with smart sorting: "1", "2" (numeric sorting)
        assert_eq!(active_vendors[0].as_str(), "1");
        assert_eq!(active_vendors[1].as_str(), "2");
    }

    #[tokio::test]
    async fn test_date_range_filtering() {
        let booth = create_test_booth();
        let vendor = create_test_vendor(&booth.id, "1");

        let purchase_repo = MockPurchaseRepository::new();
        let booth_repo = MockBoothRepository::new();
        let vendor_repo = MockVendorRepository::new();

        booth_repo.add(booth.clone());
        vendor_repo.add(vendor.clone());

        // Create purchases with different timestamps
        let now = Utc::now();
        let one_hour_ago = now - chrono::Duration::hours(1);
        let two_hours_ago = now - chrono::Duration::hours(2);

        let mut purchase1 = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(10.00), vendor.vendor_id.clone()).unwrap()],
        )
        .unwrap();
        purchase1.timestamp = two_hours_ago;

        let mut purchase2 = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(20.00), vendor.vendor_id.clone()).unwrap()],
        )
        .unwrap();
        purchase2.timestamp = one_hour_ago;

        let mut purchase3 = Purchase::new(
            booth.id,
            vec![PurchaseItem::new(dec!(30.00), vendor.vendor_id.clone()).unwrap()],
        )
        .unwrap();
        purchase3.timestamp = now;

        purchase_repo.add(purchase1);
        purchase_repo.add(purchase2);
        purchase_repo.add(purchase3);

        let service = ReportService::new(purchase_repo, booth_repo, vendor_repo);

        // Test filtering to last hour (should get 2 purchases: one_hour_ago and now)
        let date_range = DateRange::new(Some(one_hour_ago - chrono::Duration::minutes(1)), None);
        let report = service
            .generate_vendor_report(&booth.id, &vendor.vendor_id, Some(date_range))
            .await
            .unwrap();

        assert_eq!(report.sales_sum, dec!(50.00)); // 20 + 30
        assert_eq!(report.items.len(), 2);
    }

    #[tokio::test]
    async fn test_mixed_vendor_purchases() {
        // Test scenario: A purchase contains items from multiple vendors
        // Each vendor's report should only include their own items
        let booth = create_test_booth();
        let vendor1 = create_test_vendor(&booth.id, "V1");
        let vendor2 = create_test_vendor(&booth.id, "V2");

        let purchase_repo = MockPurchaseRepository::new();
        let booth_repo = MockBoothRepository::new();
        let vendor_repo = MockVendorRepository::new();

        booth_repo.add(booth.clone());
        vendor_repo.add(vendor1.clone());
        vendor_repo.add(vendor2.clone());

        // Create a purchase with items from both vendors
        let mixed_purchase = Purchase::new(
            booth.id,
            vec![
                PurchaseItem::new(dec!(10.00), vendor1.vendor_id.clone()).unwrap(),
                PurchaseItem::new(dec!(20.00), vendor2.vendor_id.clone()).unwrap(),
                PurchaseItem::new(dec!(5.00), vendor1.vendor_id.clone()).unwrap(),
            ],
        )
        .unwrap();

        purchase_repo.add(mixed_purchase);

        let service = ReportService::new(purchase_repo, booth_repo, vendor_repo);

        // Test vendor1's report - should only include their items (10.00 + 5.00)
        let report1 = service
            .generate_vendor_report(&booth.id, &vendor1.vendor_id, None)
            .await
            .unwrap();

        assert_eq!(report1.sales_sum, dec!(15.00)); // 10.00 + 5.00
        assert_eq!(report1.items.len(), 2, "Vendor 1 should have 2 items");
        assert!(
            report1
                .items
                .iter()
                .all(|item| item.item.vendor_id == vendor1.vendor_id),
            "All items in vendor 1 report should belong to vendor 1"
        );

        // Test vendor2's report - should only include their item (20.00)
        let report2 = service
            .generate_vendor_report(&booth.id, &vendor2.vendor_id, None)
            .await
            .unwrap();

        assert_eq!(report2.sales_sum, dec!(20.00));
        assert_eq!(report2.items.len(), 1, "Vendor 2 should have 1 item");
        assert_eq!(
            report2.items[0].item.vendor_id, vendor2.vendor_id,
            "The item should belong to vendor 2"
        );

        // Test booth summary - should correctly aggregate per vendor
        let booth_summary = service
            .generate_booth_summary(&booth.id, None)
            .await
            .unwrap();

        assert_eq!(booth_summary.unique_vendors, 2);
        assert_eq!(booth_summary.total_revenue, dec!(35.00)); // 10 + 20 + 5

        // Find vendor summaries
        let v1_summary = booth_summary
            .vendor_summaries
            .iter()
            .find(|v| v.vendor_id == vendor1.vendor_id)
            .unwrap();
        let v2_summary = booth_summary
            .vendor_summaries
            .iter()
            .find(|v| v.vendor_id == vendor2.vendor_id)
            .unwrap();

        assert_eq!(v1_summary.gross_sales, dec!(15.00), "Vendor 1 gross sales");
        assert_eq!(v1_summary.item_count, 2, "Vendor 1 item count");

        assert_eq!(v2_summary.gross_sales, dec!(20.00), "Vendor 2 gross sales");
        assert_eq!(v2_summary.item_count, 1, "Vendor 2 item count");
    }

    #[tokio::test]
    async fn test_booth_summary_fees_match_vendor_sum() {
        let booth = create_test_booth();
        let vendor1 = create_test_vendor(&booth.id, "1");
        let vendor2 = create_test_vendor(&booth.id, "2");
        let vendor3 = create_test_vendor(&booth.id, "3");

        let purchase_repo = MockPurchaseRepository::new();
        let booth_repo = MockBoothRepository::new();
        let vendor_repo = MockVendorRepository::new();

        booth_repo.add(booth.clone());
        vendor_repo.add(vendor1.clone());
        vendor_repo.add(vendor2.clone());
        vendor_repo.add(vendor3.clone());

        purchase_repo.add(
            Purchase::new(
                booth.id,
                vec![PurchaseItem::new(dec!(100.00), vendor1.vendor_id.clone()).unwrap()],
            )
            .unwrap(),
        );
        purchase_repo.add(
            Purchase::new(
                booth.id,
                vec![PurchaseItem::new(dec!(518.11), vendor2.vendor_id.clone()).unwrap()],
            )
            .unwrap(),
        );
        purchase_repo.add(
            Purchase::new(
                booth.id,
                vec![PurchaseItem::new(dec!(75.25), vendor3.vendor_id.clone()).unwrap()],
            )
            .unwrap(),
        );

        let service = ReportService::new(purchase_repo, booth_repo, vendor_repo);
        let summary = service
            .generate_booth_summary(&booth.id, None)
            .await
            .unwrap();

        let sum_vendor_fees: Decimal = summary.vendor_summaries.iter().map(|v| v.fees_due).sum();
        assert_eq!(sum_vendor_fees, summary.total_booth_revenue);
        assert_eq!(
            summary.total_booth_revenue,
            summary.total_participation_fees + summary.total_sales_fees
        );
    }
}
