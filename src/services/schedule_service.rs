use crate::models::{Schedule, ServiceError, WeeklyHours};
use crate::repositories::ScheduleRepository;
use std::sync::Arc;

pub struct ScheduleService {
    repository: Arc<dyn ScheduleRepository>,
}

impl ScheduleService {
    pub fn new(repository: Arc<dyn ScheduleRepository>) -> Self {
        Self { repository }
    }

    pub async fn update_schedule(&self, user_id: i64, hours: WeeklyHours) -> Result<(), ServiceError> {
        // Business logic: Create and validate schedule
        let schedule = Schedule::new(user_id, hours)
            .map_err(|e| ServiceError::ValidationError(e))?;

        // Persistence: Save through repository
        self.repository.save(&schedule).await?;

        println!("Schedule updated for user {}: is_synced={}", user_id, schedule.is_synced);
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_schedule(&self, user_id: i64) -> Result<Option<Schedule>, ServiceError> {
        self.repository.find_by_user_id(user_id).await
    }

    pub async fn get_sync_status(&self, user_id: i64) -> Result<ScheduleSyncStatus, ServiceError> {
        match self.repository.find_by_user_id(user_id).await? {
            Some(schedule) => Ok(ScheduleSyncStatus {
                is_synced: schedule.is_synced,
                schedule: Some(schedule.hours),
                last_synced: schedule.last_synced.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string()),
                last_modified: Some(schedule.last_modified.format("%Y-%m-%d %H:%M").to_string()),
            }),
            None => Ok(ScheduleSyncStatus {
                is_synced: true, // No schedule means no sync needed
                schedule: None,
                last_synced: None,
                last_modified: None,
            }),
        }
    }

    #[allow(dead_code)]
    pub async fn mark_as_synced(&self, user_id: i64) -> Result<(), ServiceError> {
        self.repository.mark_as_synced(user_id).await
    }

    #[allow(dead_code)]
    pub async fn get_unsynced_schedules(&self) -> Result<Vec<Schedule>, ServiceError> {
        self.repository.find_unsynced().await
    }
}

#[derive(serde::Serialize)]
pub struct ScheduleSyncStatus {
    pub is_synced: bool,
    pub schedule: Option<WeeklyHours>,
    pub last_synced: Option<String>,
    pub last_modified: Option<String>,
}