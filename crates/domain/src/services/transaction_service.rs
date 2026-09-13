use crate::error::{DomainError, DomainResult};
use crate::error_code::ValidationError;
use crate::models::{BoothId, Purchase, PurchaseId, PurchaseItem, VendorId};
use crate::repositories::PurchaseRepository;
use crate::services::dto::{ChargingConfig, VendorPayout};
use rust_decimal::Decimal;

/// Service for transaction and purchase operations
pub struct TransactionService<R: PurchaseRepository> {
    repository: R,
}

impl<R: PurchaseRepository> TransactionService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Process a checkout with multiple items for a single vendor
    pub async fn checkout(
        &self,
        booth_id: BoothId,
        vendor_id: VendorId,
        item_amounts: Vec<Decimal>,
    ) -> DomainResult<Purchase> {
        if item_amounts.is_empty() {
            return Err(DomainError::Validation(ValidationError::PurchaseEmpty));
        }

        let purchase_items: Vec<PurchaseItem> = item_amounts
            .into_iter()
            .map(|amount| PurchaseItem::new(amount, vendor_id.clone()))
            .collect::<DomainResult<Vec<_>>>()?;

        let purchase = Purchase::new(booth_id, purchase_items)?;

        self.repository.save(&purchase).await?;
        Ok(purchase)
    }

    /// Get a purchase by ID
    pub async fn get_purchase(&self, purchase_id: PurchaseId) -> DomainResult<Purchase> {
        self.repository
            .find_by_id(&purchase_id)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Purchase {} not found", purchase_id.as_str()))
            })
    }

    /// List all purchases for a booth
    pub async fn list_purchases(&self, booth_id: BoothId) -> DomainResult<Vec<Purchase>> {
        self.repository.find_by_booth(&booth_id).await
    }

    /// List all purchases for a specific vendor in a booth
    pub async fn list_vendor_purchases(
        &self,
        booth_id: BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Vec<Purchase>> {
        self.repository.find_by_vendor(&booth_id, vendor_id).await
    }

    /// Calculate total sales for a vendor
    pub async fn calculate_vendor_sales(
        &self,
        booth_id: BoothId,
        vendor_id: &VendorId,
    ) -> DomainResult<Decimal> {
        let purchases = self.list_vendor_purchases(booth_id, vendor_id).await?;
        let total = purchases
            .iter()
            .flat_map(|p| &p.items)
            .map(|item| item.amount)
            .sum();
        Ok(total)
    }

    /// Calculate vendor payout using the authoritative rounding path
    pub fn calculate_payout(&self, total_sales: Decimal, config: &ChargingConfig) -> VendorPayout {
        config.calculate_payout(total_sales)
    }

    /// Delete a purchase (for corrections)
    pub async fn delete_purchase(&self, purchase_id: PurchaseId) -> DomainResult<()> {
        self.repository.delete(&purchase_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockPurchaseRepository;
    use rust_decimal_macros::dec;

    fn create_test_booth_id() -> BoothId {
        BoothId::new()
    }

    fn create_test_vendor_id() -> VendorId {
        VendorId::new("V123".to_string())
    }

    #[tokio::test]
    async fn test_checkout() {
        let repo = MockPurchaseRepository::new();
        let service = TransactionService::new(repo);
        let booth_id = create_test_booth_id();
        let vendor_id = create_test_vendor_id();

        let items = vec![dec!(10.50), dec!(25.00)];

        let purchase = service
            .checkout(booth_id, vendor_id.clone(), items)
            .await
            .unwrap();

        assert_eq!(purchase.booth_id, booth_id);
        assert_eq!(purchase.items.len(), 2);
        assert_eq!(purchase.items[0].amount, dec!(10.50));
        assert_eq!(purchase.items[0].vendor_id, vendor_id);
        assert_eq!(purchase.items[1].amount, dec!(25.00));
        assert_eq!(purchase.items[1].vendor_id, vendor_id);
    }

    #[tokio::test]
    async fn test_checkout_empty_items() {
        let repo = MockPurchaseRepository::new();
        let service = TransactionService::new(repo);
        let booth_id = create_test_booth_id();
        let vendor_id = create_test_vendor_id();

        let result = service.checkout(booth_id, vendor_id, vec![]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_vendor_purchases() {
        let repo = MockPurchaseRepository::new();
        let service = TransactionService::new(repo);
        let booth_id = create_test_booth_id();
        let vendor1 = VendorId::new("V1".to_string());
        let vendor2 = VendorId::new("V2".to_string());

        // Create purchases for two different vendors
        service
            .checkout(booth_id, vendor1.clone(), vec![dec!(10.00)])
            .await
            .unwrap();

        service
            .checkout(booth_id, vendor2.clone(), vec![dec!(20.00)])
            .await
            .unwrap();

        service
            .checkout(booth_id, vendor1.clone(), vec![dec!(15.00)])
            .await
            .unwrap();

        // List purchases for vendor1
        let vendor1_purchases = service
            .list_vendor_purchases(booth_id, &vendor1)
            .await
            .unwrap();
        assert_eq!(vendor1_purchases.len(), 2);

        // List purchases for vendor2
        let vendor2_purchases = service
            .list_vendor_purchases(booth_id, &vendor2)
            .await
            .unwrap();
        assert_eq!(vendor2_purchases.len(), 1);
    }

    #[tokio::test]
    async fn test_calculate_vendor_sales() {
        let repo = MockPurchaseRepository::new();
        let service = TransactionService::new(repo);
        let booth_id = create_test_booth_id();
        let vendor_id = create_test_vendor_id();

        // Create multiple purchases
        service
            .checkout(booth_id, vendor_id.clone(), vec![dec!(10.50), dec!(15.00)])
            .await
            .unwrap();

        service
            .checkout(booth_id, vendor_id.clone(), vec![dec!(20.00)])
            .await
            .unwrap();

        // Calculate total sales
        let total_sales = service
            .calculate_vendor_sales(booth_id, &vendor_id)
            .await
            .unwrap();

        assert_eq!(total_sales, dec!(45.50));
    }

    #[tokio::test]
    async fn test_calculate_payout() {
        let repo = MockPurchaseRepository::new();
        let service = TransactionService::new(repo);

        let config = ChargingConfig {
            participation_fee: dec!(5.00),
            sales_fee: dec!(10.0), // 10%
            rounding_step: dec!(0.50),
        };

        let payout = service.calculate_payout(dec!(100.00), &config);

        assert_eq!(payout.gross_sales, dec!(100.00));
        assert_eq!(payout.fees_due, dec!(15.00));
        assert_eq!(payout.net_payout, dec!(85.00));
    }

    #[tokio::test]
    async fn test_delete_purchase() {
        let repo = MockPurchaseRepository::new();
        let service = TransactionService::new(repo);
        let booth_id = create_test_booth_id();
        let vendor_id = create_test_vendor_id();

        let purchase = service
            .checkout(booth_id, vendor_id, vec![dec!(10.00)])
            .await
            .unwrap();

        // Delete purchase
        service.delete_purchase(purchase.id).await.unwrap();

        // Verify deletion
        let result = service.get_purchase(purchase.id).await;
        assert!(result.is_err());
    }
}
