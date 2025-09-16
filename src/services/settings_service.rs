use crate::models::{SettingsEntry, ServiceError};
use crate::repositories::SettingsRepository;
use std::sync::Arc;

pub struct SettingsService {
    repository: Arc<dyn SettingsRepository>,
}

impl SettingsService {
    pub fn new(repository: Arc<dyn SettingsRepository>) -> Self {
        Self { repository }
    }

    pub async fn add_entry(
        &self,
        key: String,
        value: String,
    ) -> Result<String, ServiceError> {

        // Business logic: Check if entry already exists
        if self.repository.find_by_key(&key).await?.is_some() {
            return Err(ServiceError::ValidationError("Setting key already exists".to_string()));
        }

        // Create new entry
        let new_entry = SettingsEntry::new(key.clone(), value.clone());

        self.repository.save(&new_entry).await?;

        println!("Added new setting: {} = {}", key, value);
        Ok(format!("Setting {} added successfully", key))
    }

    #[allow(dead_code)]
    pub async fn delete_entry(&self, id: i64) -> Result<String, ServiceError> {
        let _entry = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("Entry not found".to_string()))?;

        self.repository.delete(id).await?;

        println!("Deleted entry with id: {}", id);
        Ok(format!("Entry {} deleted successfully", id))
    }

    #[allow(dead_code)]
    pub async fn find_by_id(&self, id: i64) -> Result<Option<SettingsEntry>, ServiceError> {
        self.repository.find_by_id(id).await
    }

    pub async fn find_by_key(&self, key: &str) -> Result<Option<SettingsEntry>, ServiceError> {
        self.repository.find_by_key(key).await
    }

    #[allow(dead_code)]
    pub async fn find_all(&self) -> Result<Vec<SettingsEntry>, ServiceError> {
        self.repository.find_all().await
    }

    pub async fn update_entry_value(
        &self,
        id: i64,
        value: String,
    ) -> Result<String, ServiceError> {
        let mut entry = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("Entry not found".to_string()))?;

        entry.value = value;

        self.repository.save(&entry).await?;

        println!("Updated entry with id: {}", id);
        Ok(format!("Entry {} updated successfully", id))
    }

    // Convenience methods for common settings
    pub async fn get_admin_password_hash(&self) -> Result<Option<String>, ServiceError> {
        Ok(self.find_by_key("admin_password_hash").await?.map(|entry| entry.value))
    }

    #[allow(dead_code)]
    pub async fn get_jwt_secret(&self) -> Result<Option<String>, ServiceError> {
        Ok(self.find_by_key("jwt_secret").await?.map(|entry| entry.value))
    }

    #[allow(dead_code)]
    pub async fn get_check_interval(&self) -> Result<Option<i32>, ServiceError> {
        if let Some(entry) = self.find_by_key("check_interval").await? {
            entry.value.parse::<i32>()
                .map(Some)
                .map_err(|_| ServiceError::ValidationError("Invalid check_interval value".to_string()))
        } else {
            Ok(None)
        }
    }
}
