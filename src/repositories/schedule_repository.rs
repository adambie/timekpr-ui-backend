use crate::models::{Schedule, ServiceError, WeeklyHours};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

#[async_trait]
pub trait ScheduleRepository: Send + Sync {
    async fn save(&self, schedule: &Schedule) -> Result<(), ServiceError>;
    async fn find_by_user_id(&self, user_id: i64) -> Result<Option<Schedule>, ServiceError>;
    #[allow(dead_code)]
    async fn find_unsynced(&self) -> Result<Vec<Schedule>, ServiceError>;
    #[allow(dead_code)]
    async fn mark_as_synced(&self, user_id: i64) -> Result<(), ServiceError>;
}

pub struct SqliteScheduleRepository {
    pool: SqlitePool,
}

impl SqliteScheduleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScheduleRepository for SqliteScheduleRepository {
    async fn save(&self, schedule: &Schedule) -> Result<(), ServiceError> {
        let last_modified = schedule.last_modified.naive_utc();
        sqlx::query!(
            "INSERT OR REPLACE INTO user_weekly_schedule 
             (user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours, 
              friday_hours, saturday_hours, sunday_hours, is_synced, last_modified)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            schedule.user_id,
            schedule.hours.monday,
            schedule.hours.tuesday,
            schedule.hours.wednesday,
            schedule.hours.thursday,
            schedule.hours.friday,
            schedule.hours.saturday,
            schedule.hours.sunday,
            schedule.is_synced,
            last_modified
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    async fn find_by_user_id(&self, user_id: i64) -> Result<Option<Schedule>, ServiceError> {
        let row = sqlx::query!(
            "SELECT user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours,
                    friday_hours, saturday_hours, sunday_hours, is_synced, last_synced, last_modified
             FROM user_weekly_schedule WHERE user_id = ? ORDER BY last_modified DESC LIMIT 1",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let schedule = Schedule {
                user_id: row.user_id,
                hours: WeeklyHours {
                    monday: row.monday_hours.unwrap_or(0.0),
                    tuesday: row.tuesday_hours.unwrap_or(0.0),
                    wednesday: row.wednesday_hours.unwrap_or(0.0),
                    thursday: row.thursday_hours.unwrap_or(0.0),
                    friday: row.friday_hours.unwrap_or(0.0),
                    saturday: row.saturday_hours.unwrap_or(0.0),
                    sunday: row.sunday_hours.unwrap_or(0.0),
                },
                is_synced: row.is_synced.unwrap_or(false),
                last_synced: row.last_synced.map(|dt| dt.and_utc()),
                last_modified: row.last_modified.map(|dt| dt.and_utc()).unwrap_or_else(|| Utc::now()),
            };
            Ok(Some(schedule))
        } else {
            Ok(None)
        }
    }

    async fn find_unsynced(&self) -> Result<Vec<Schedule>, ServiceError> {
        let rows = sqlx::query!(
            "SELECT user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours,
                    friday_hours, saturday_hours, sunday_hours, is_synced, last_synced, last_modified
             FROM user_weekly_schedule WHERE is_synced = 0"
        )
        .fetch_all(&self.pool)
        .await?;

        let schedules = rows
            .into_iter()
            .map(|row| Schedule {
                user_id: row.user_id,
                hours: WeeklyHours {
                    monday: row.monday_hours.unwrap_or(0.0),
                    tuesday: row.tuesday_hours.unwrap_or(0.0),
                    wednesday: row.wednesday_hours.unwrap_or(0.0),
                    thursday: row.thursday_hours.unwrap_or(0.0),
                    friday: row.friday_hours.unwrap_or(0.0),
                    saturday: row.saturday_hours.unwrap_or(0.0),
                    sunday: row.sunday_hours.unwrap_or(0.0),
                },
                is_synced: row.is_synced.unwrap_or(false),
                last_synced: row.last_synced.map(|dt| dt.and_utc()),
                last_modified: row.last_modified.map(|dt| dt.and_utc()).unwrap_or_else(|| Utc::now()),
            })
            .collect();

        Ok(schedules)
    }

    async fn mark_as_synced(&self, user_id: i64) -> Result<(), ServiceError> {
        let now = Utc::now().naive_utc();
        sqlx::query!(
            "UPDATE user_weekly_schedule SET is_synced = 1, last_synced = ? WHERE user_id = ?",
            now,
            user_id
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}