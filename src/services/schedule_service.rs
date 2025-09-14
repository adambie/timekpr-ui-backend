use crate::models::{
    Schedule, ScheduleSyncStatus, ScheduleWithIntervals, ServiceError, WeeklyHours,
    WeeklyTimeIntervals,
};
use crate::repositories::ScheduleRepository;
use std::sync::Arc;

pub struct ScheduleService {
    repository: Arc<dyn ScheduleRepository>,
}

impl ScheduleService {
    pub fn new(repository: Arc<dyn ScheduleRepository>) -> Self {
        Self { repository }
    }

    pub async fn update_schedule(
        &self,
        user_id: i64,
        hours: WeeklyHours,
    ) -> Result<(), ServiceError> {
        // Business logic: Create and validate schedule (backward compatibility)
        let schedule =
            Schedule::new(user_id, hours).map_err(|e| ServiceError::ValidationError(e))?;

        // Persistence: Save through repository
        self.repository.save(&schedule).await?;

        println!(
            "Schedule updated for user {}: is_synced={}",
            user_id, schedule.is_synced
        );
        Ok(())
    }

    pub async fn update_schedule_with_intervals(
        &self,
        user_id: i64,
        hours: WeeklyHours,
        intervals: WeeklyTimeIntervals,
    ) -> Result<(), ServiceError> {
        // Business logic: Create and validate schedule with intervals
        let schedule = Schedule::new_with_intervals(user_id, hours, intervals)
            .map_err(|e| ServiceError::ValidationError(e))?;

        // Persistence: Save through repository
        self.repository.save(&schedule).await?;

        println!(
            "Schedule with intervals updated for user {}: is_synced={}",
            user_id, schedule.is_synced
        );
        Ok(())
    }

    pub async fn get_sync_status(&self, user_id: i64) -> Result<ScheduleSyncStatus, ServiceError> {
        match self.repository.find_by_user_id(user_id).await? {
            Some(schedule) => Ok(ScheduleSyncStatus {
                is_synced: schedule.is_synced,
                schedule: Some(ScheduleWithIntervals {
                    hours: schedule.hours,
                    intervals: schedule.intervals,
                }),
                last_synced: schedule
                    .last_synced
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string()),
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

    pub async fn mark_as_synced(&self, user_id: i64) -> Result<(), ServiceError> {
        self.repository.mark_as_synced(user_id).await
    }

    pub async fn get_unsynced_schedules(&self) -> Result<Vec<Schedule>, ServiceError> {
        self.repository.find_unsynced().await
    }

    // Helper method to prepare sync data for SSH operations
    pub fn prepare_sync_data(
        &self,
        schedule: &Schedule,
    ) -> (
        std::collections::HashMap<String, f64>,
        std::collections::HashMap<String, (String, String)>,
    ) {
        // Create time limits dict with non-null values only
        let mut schedule_dict = std::collections::HashMap::new();
        let hours = &schedule.hours;

        if hours.monday > 0.0 {
            schedule_dict.insert("monday".to_string(), hours.monday);
        }
        if hours.tuesday > 0.0 {
            schedule_dict.insert("tuesday".to_string(), hours.tuesday);
        }
        if hours.wednesday > 0.0 {
            schedule_dict.insert("wednesday".to_string(), hours.wednesday);
        }
        if hours.thursday > 0.0 {
            schedule_dict.insert("thursday".to_string(), hours.thursday);
        }
        if hours.friday > 0.0 {
            schedule_dict.insert("friday".to_string(), hours.friday);
        }
        if hours.saturday > 0.0 {
            schedule_dict.insert("saturday".to_string(), hours.saturday);
        }
        if hours.sunday > 0.0 {
            schedule_dict.insert("sunday".to_string(), hours.sunday);
        }

        // Create time intervals dict
        let mut intervals_dict = std::collections::HashMap::new();
        let intervals = &schedule.intervals;

        intervals_dict.insert(
            "monday".to_string(),
            (
                intervals.monday.start_time.clone(),
                intervals.monday.end_time.clone(),
            ),
        );
        intervals_dict.insert(
            "tuesday".to_string(),
            (
                intervals.tuesday.start_time.clone(),
                intervals.tuesday.end_time.clone(),
            ),
        );
        intervals_dict.insert(
            "wednesday".to_string(),
            (
                intervals.wednesday.start_time.clone(),
                intervals.wednesday.end_time.clone(),
            ),
        );
        intervals_dict.insert(
            "thursday".to_string(),
            (
                intervals.thursday.start_time.clone(),
                intervals.thursday.end_time.clone(),
            ),
        );
        intervals_dict.insert(
            "friday".to_string(),
            (
                intervals.friday.start_time.clone(),
                intervals.friday.end_time.clone(),
            ),
        );
        intervals_dict.insert(
            "saturday".to_string(),
            (
                intervals.saturday.start_time.clone(),
                intervals.saturday.end_time.clone(),
            ),
        );
        intervals_dict.insert(
            "sunday".to_string(),
            (
                intervals.sunday.start_time.clone(),
                intervals.sunday.end_time.clone(),
            ),
        );

        (schedule_dict, intervals_dict)
    }
}
