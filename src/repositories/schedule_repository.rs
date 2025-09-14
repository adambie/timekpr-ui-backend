use crate::models::{Schedule, ServiceError, TimeInterval, WeeklyHours, WeeklyTimeIntervals};
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

        // Extract interval values to avoid borrowing issues
        let mon_start = &schedule.intervals.monday.start_time;
        let mon_end = &schedule.intervals.monday.end_time;
        let tue_start = &schedule.intervals.tuesday.start_time;
        let tue_end = &schedule.intervals.tuesday.end_time;
        let wed_start = &schedule.intervals.wednesday.start_time;
        let wed_end = &schedule.intervals.wednesday.end_time;
        let thu_start = &schedule.intervals.thursday.start_time;
        let thu_end = &schedule.intervals.thursday.end_time;
        let fri_start = &schedule.intervals.friday.start_time;
        let fri_end = &schedule.intervals.friday.end_time;
        let sat_start = &schedule.intervals.saturday.start_time;
        let sat_end = &schedule.intervals.saturday.end_time;
        let sun_start = &schedule.intervals.sunday.start_time;
        let sun_end = &schedule.intervals.sunday.end_time;

        sqlx::query!(
            "INSERT OR REPLACE INTO user_weekly_schedule 
             (user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours, 
              friday_hours, saturday_hours, sunday_hours, is_synced, last_modified,
              monday_start_time, monday_end_time, tuesday_start_time, tuesday_end_time,
              wednesday_start_time, wednesday_end_time, thursday_start_time, thursday_end_time,
              friday_start_time, friday_end_time, saturday_start_time, saturday_end_time,
              sunday_start_time, sunday_end_time)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                     ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            schedule.user_id,
            schedule.hours.monday,
            schedule.hours.tuesday,
            schedule.hours.wednesday,
            schedule.hours.thursday,
            schedule.hours.friday,
            schedule.hours.saturday,
            schedule.hours.sunday,
            schedule.is_synced,
            last_modified,
            mon_start,
            mon_end,
            tue_start,
            tue_end,
            wed_start,
            wed_end,
            thu_start,
            thu_end,
            fri_start,
            fri_end,
            sat_start,
            sat_end,
            sun_start,
            sun_end
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_by_user_id(&self, user_id: i64) -> Result<Option<Schedule>, ServiceError> {
        let row = sqlx::query!(
            "SELECT user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours,
                    friday_hours, saturday_hours, sunday_hours, is_synced, last_synced, last_modified,
                    monday_start_time, monday_end_time, tuesday_start_time, tuesday_end_time,
                    wednesday_start_time, wednesday_end_time, thursday_start_time, thursday_end_time,
                    friday_start_time, friday_end_time, saturday_start_time, saturday_end_time,
                    sunday_start_time, sunday_end_time
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
                intervals: WeeklyTimeIntervals {
                    monday: TimeInterval {
                        start_time: row.monday_start_time.unwrap_or("00:00".to_string()),
                        end_time: row.monday_end_time.unwrap_or("23:59".to_string()),
                    },
                    tuesday: TimeInterval {
                        start_time: row.tuesday_start_time.unwrap_or("00:00".to_string()),
                        end_time: row.tuesday_end_time.unwrap_or("23:59".to_string()),
                    },
                    wednesday: TimeInterval {
                        start_time: row.wednesday_start_time.unwrap_or("00:00".to_string()),
                        end_time: row.wednesday_end_time.unwrap_or("23:59".to_string()),
                    },
                    thursday: TimeInterval {
                        start_time: row.thursday_start_time.unwrap_or("00:00".to_string()),
                        end_time: row.thursday_end_time.unwrap_or("23:59".to_string()),
                    },
                    friday: TimeInterval {
                        start_time: row.friday_start_time.unwrap_or("00:00".to_string()),
                        end_time: row.friday_end_time.unwrap_or("23:59".to_string()),
                    },
                    saturday: TimeInterval {
                        start_time: row.saturday_start_time.unwrap_or("00:00".to_string()),
                        end_time: row.saturday_end_time.unwrap_or("23:59".to_string()),
                    },
                    sunday: TimeInterval {
                        start_time: row.sunday_start_time.unwrap_or("00:00".to_string()),
                        end_time: row.sunday_end_time.unwrap_or("23:59".to_string()),
                    },
                },
                is_synced: row.is_synced.unwrap_or(false),
                last_synced: row.last_synced.map(|dt| dt.and_utc()),
                last_modified: row
                    .last_modified
                    .map(|dt| dt.and_utc())
                    .unwrap_or_else(|| Utc::now()),
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
                intervals: WeeklyTimeIntervals::default(),
                is_synced: row.is_synced.unwrap_or(false),
                last_synced: row.last_synced.map(|dt| dt.and_utc()),
                last_modified: row
                    .last_modified
                    .map(|dt| dt.and_utc())
                    .unwrap_or_else(|| Utc::now()),
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
