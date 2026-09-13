use crate::models::{Booth, Transaction};
// TODO: Re-enable BoothReport once report module is fixed
// use crate::models::BoothReport;
use std::sync::Arc;
use uuid::Uuid;

use super::{BoothStorage, StorageResult};

/// Service for managing booths and transactions
pub struct BoothService {
    storage: Arc<dyn BoothStorage>,
}

impl BoothService {
    pub fn new(storage: Arc<dyn BoothStorage>) -> Self {
        Self { storage }
    }

    pub fn create_booth(&self, name: String) -> StorageResult<Booth> {
        let booth = Booth::new(name);
        self.storage.save_booth(&booth)?;
        Ok(booth)
    }

    pub fn get_booth(&self, id: &Uuid) -> StorageResult<Booth> {
        self.storage.load_booth(id)
    }

    pub fn get_all_booths(&self) -> StorageResult<Vec<Booth>> {
        self.storage.load_all_booths()
    }

    pub fn add_transaction(
        &self,
        booth_id: &Uuid,
        transaction: Transaction,
    ) -> StorageResult<Booth> {
        let mut booth = self.storage.load_booth(booth_id)?;
        booth.add_transaction(transaction);
        self.storage.save_booth(&booth)?;
        Ok(booth)
    }

    pub fn delete_booth(&self, id: &Uuid) -> StorageResult<()> {
        self.storage.delete_booth(id)
    }

    // TODO: Re-enable once BoothReport is fixed
    // pub fn generate_report(&self, booth_id: &Uuid, commission_rate: Decimal) -> StorageResult<BoothReport> {
    //     let booth = self.storage.load_booth(booth_id)?;
    //     Ok(BoothReport::from_booth(&booth, commission_rate))
    // }

    pub fn export_data(&self) -> StorageResult<String> {
        self.storage.export_data()
    }

    pub fn import_data(&self, data: &str) -> StorageResult<Vec<Booth>> {
        self.storage.import_data(data)
    }
}
