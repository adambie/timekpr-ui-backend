use crate::models::ServiceError;
use async_trait::async_trait;
use chrono::NaiveDate;
use sqlx::SqlitePool;

#[async_trait]
pub trait UsageRepository: Send + Sync {
    async fn get_time_spent(&self, user_id: i64, date: NaiveDate) -> Result<Option<i64>, ServiceError>;
    async fn get_usage_data(&self, user_id: i64, days: i32) -> Result<Vec<(NaiveDate, i64)>, ServiceError>;
}

pub struct SqliteUsageRepository {
    pool: SqlitePool,
}

impl SqliteUsageRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UsageRepository for SqliteUsageRepository {
    async fn get_time_spent(&self, user_id: i64, date: NaiveDate) -> Result<Option<i64>, ServiceError> {
        let time_spent = sqlx::query_scalar!(
            "SELECT time_spent FROM user_time_usage WHERE user_id = ? AND date = ?",
            user_id, date
        )
        .fetch_optional(&self.pool)
        .await?;
        
        Ok(time_spent.flatten())
    }

    async fn get_usage_data(&self, user_id: i64, days: i32) -> Result<Vec<(NaiveDate, i64)>, ServiceError> {
        let rows = sqlx::query!(
            "SELECT date, time_spent FROM user_time_usage 
             WHERE user_id = ? AND date >= date('now', '-' || ? || ' days')
             ORDER BY date ASC",
            user_id, days
        )
        .fetch_all(&self.pool)
        .await?;

        let mut usage_data = Vec::new();
        for row in rows {
            usage_data.push((row.date, row.time_spent.unwrap_or(0)));
        }

        Ok(usage_data)
    }
}