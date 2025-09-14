use crate::models::{ManagedUser, ServiceError};
use async_trait::async_trait;
use sqlx::SqlitePool;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: i64) -> Result<Option<ManagedUser>, ServiceError>;
    async fn find_all_valid(&self) -> Result<Vec<ManagedUser>, ServiceError>;
    async fn find_all_pending(&self) -> Result<Vec<ManagedUser>, ServiceError>;
    async fn find_all(&self) -> Result<Vec<ManagedUser>, ServiceError>;
    async fn save(&self, user: &ManagedUser) -> Result<(), ServiceError>;
    async fn delete(&self, id: i64) -> Result<(), ServiceError>;
    async fn update_pending_time_adjustment(
        &self,
        user_id: i64,
        operation: &str,
        seconds: i64,
    ) -> Result<(), ServiceError>;
    #[allow(dead_code)]
    async fn clear_pending_time_adjustment(&self, user_id: i64) -> Result<(), ServiceError>;
}

pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn find_by_id(&self, id: i64) -> Result<Option<ManagedUser>, ServiceError> {
        let row = sqlx::query!(
            "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_users WHERE id = ?",
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(ManagedUser {
                id: row.id,
                username: row.username,
                system_ip: row.system_ip,
                is_valid: row.is_valid.unwrap_or(false),
                date_added: row.date_added.map(|dt| dt.and_utc()),
                last_checked: row.last_checked.map(|dt| dt.and_utc()),
                last_config: row.last_config,
                pending_time_adjustment: row.pending_time_adjustment,
                pending_time_operation: row.pending_time_operation,
            }))
        } else {
            Ok(None)
        }
    }

    async fn find_all_valid(&self) -> Result<Vec<ManagedUser>, ServiceError> {
        let rows = sqlx::query!(
            "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_users WHERE is_valid = 1 ORDER BY username"
        )
        .fetch_all(&self.pool)
        .await?;

        let users = rows
            .into_iter()
            .map(|row| ManagedUser {
                id: row.id,
                username: row.username,
                system_ip: row.system_ip,
                is_valid: row.is_valid.unwrap_or(false),
                date_added: row.date_added.map(|dt| dt.and_utc()),
                last_checked: row.last_checked.map(|dt| dt.and_utc()),
                last_config: row.last_config,
                pending_time_adjustment: row.pending_time_adjustment,
                pending_time_operation: row.pending_time_operation,
            })
            .collect();

        Ok(users)
    }

    async fn find_all_pending(&self) -> Result<Vec<ManagedUser>, ServiceError> {
        let rows = sqlx::query!(
            "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_users WHERE pending_time_adjustment IS NOT NULL AND pending_time_operation IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await?;

        let users = rows
            .into_iter()
            .map(|row| ManagedUser {
                id: row.id,
                username: row.username,
                system_ip: row.system_ip,
                is_valid: row.is_valid.unwrap_or(false),
                date_added: row.date_added.map(|dt| dt.and_utc()),
                last_checked: row.last_checked.map(|dt| dt.and_utc()),
                last_config: row.last_config,
                pending_time_adjustment: row.pending_time_adjustment,
                pending_time_operation: row.pending_time_operation,
            })
            .collect();

        Ok(users)
    }

    async fn find_all(&self) -> Result<Vec<ManagedUser>, ServiceError> {
        let rows = sqlx::query!(
            "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_users ORDER BY username"
        )
        .fetch_all(&self.pool)
        .await?;

        let users = rows
            .into_iter()
            .map(|row| ManagedUser {
                id: row.id,
                username: row.username,
                system_ip: row.system_ip,
                is_valid: row.is_valid.unwrap_or(false),
                date_added: row.date_added.map(|dt| dt.and_utc()),
                last_checked: row.last_checked.map(|dt| dt.and_utc()),
                last_config: row.last_config,
                pending_time_adjustment: row.pending_time_adjustment,
                pending_time_operation: row.pending_time_operation,
            })
            .collect();

        Ok(users)
    }

    async fn save(&self, user: &ManagedUser) -> Result<(), ServiceError> {
        if user.id == 0 {
            // Insert new user
            let date_added = user.date_added.map(|dt| dt.naive_utc());
            let last_checked = user.last_checked.map(|dt| dt.naive_utc());
            sqlx::query!(
                "INSERT INTO managed_users (username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                user.username,
                user.system_ip,
                user.is_valid,
                date_added,
                last_checked,
                user.last_config,
                user.pending_time_adjustment,
                user.pending_time_operation
            )
            .execute(&self.pool)
            .await?;
        } else {
            // Update existing user
            let last_checked = user.last_checked.map(|dt| dt.naive_utc());
            sqlx::query!(
                "UPDATE managed_users SET username = ?, system_ip = ?, is_valid = ?, last_checked = ?, last_config = ?, pending_time_adjustment = ?, pending_time_operation = ? WHERE id = ?",
                user.username,
                user.system_ip,
                user.is_valid,
                last_checked,
                user.last_config,
                user.pending_time_adjustment,
                user.pending_time_operation,
                user.id
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), ServiceError> {
        sqlx::query!("DELETE FROM managed_users WHERE id = ?", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_pending_time_adjustment(
        &self,
        user_id: i64,
        operation: &str,
        seconds: i64,
    ) -> Result<(), ServiceError> {
        sqlx::query!(
            "UPDATE managed_users SET pending_time_adjustment = ?, pending_time_operation = ? WHERE id = ?",
            seconds,
            operation,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn clear_pending_time_adjustment(&self, user_id: i64) -> Result<(), ServiceError> {
        sqlx::query!(
            "UPDATE managed_users SET pending_time_adjustment = NULL, pending_time_operation = NULL WHERE id = ?",
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
