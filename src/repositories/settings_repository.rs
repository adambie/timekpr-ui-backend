use crate::models::{SettingsEntry, ServiceError};
use async_trait::async_trait;
use sqlx::SqlitePool;

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<SettingsEntry>, ServiceError>;
    async fn find_by_key(&self, key: &str) -> Result<Option<SettingsEntry>, ServiceError>;
    async fn find_all(&self) -> Result<Vec<SettingsEntry>, ServiceError>;
    async fn save(&self, entry: &SettingsEntry) -> Result<(), ServiceError>;
    async fn delete(&self, id: i64) -> Result<(), ServiceError>;

}

pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<SettingsEntry>, ServiceError> {
        let row = sqlx::query!(
            "SELECT id, key, value FROM settings WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(SettingsEntry::with_id(row.id, row.key, row.value)))
        } else {
            Ok(None)
        }
    }
    
    async fn find_by_key(&self, key: &str) -> Result<Option<SettingsEntry>, ServiceError> {
        let row = sqlx::query!(
            "SELECT id, key, value FROM settings WHERE key = ?",
            key
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let id = row.id.ok_or_else(|| ServiceError::DatabaseError("Invalid settings row: missing ID".to_string()))?;
            Ok(Some(SettingsEntry::with_id(id, row.key, row.value)))

        } else {
            Ok(None)
        }
    }


    async fn find_all(&self) -> Result<Vec<SettingsEntry>, ServiceError> {
        let rows = sqlx::query!(
            "SELECT id, key, value FROM settings ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await?;

        let settings = rows
            .into_iter()
            .map(|row| SettingsEntry::with_id(row.id, row.key, row.value))
            .collect();

        Ok(settings)
    }

    async fn save(&self, entry: &SettingsEntry) -> Result<(), ServiceError> {
        if entry.id == 0 {
            // Insert new entry
            sqlx::query!(
                "INSERT INTO settings (key, value) 
                 VALUES (?, ?)",
                entry.key,
                entry.value,
            )
            .execute(&self.pool)
            .await?;
        } else {
            // Update existing entry
            sqlx::query!(
                "UPDATE settings SET key = ?, value = ? WHERE id = ?",
                entry.key,
                entry.value,
                entry.id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), ServiceError> {
        sqlx::query!("DELETE FROM settings WHERE id = ?", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
