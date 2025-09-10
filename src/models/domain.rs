use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeInterval {
    pub start_time: String, // Format: "HH:MM"
    pub end_time: String,   // Format: "HH:MM"
}

impl TimeInterval {
    pub fn new(start_time: String, end_time: String) -> Result<Self, String> {
        // Validate time format
        if !Self::is_valid_time_format(&start_time) {
            return Err(format!("Invalid start time format: {}. Expected HH:MM", start_time));
        }
        if !Self::is_valid_time_format(&end_time) {
            return Err(format!("Invalid end time format: {}. Expected HH:MM", end_time));
        }
        
        // Validate start < end
        if start_time >= end_time {
            return Err("Start time must be before end time".to_string());
        }
        
        Ok(Self {
            start_time,
            end_time,
        })
    }
    
    pub fn default() -> Self {
        Self {
            start_time: "00:00".to_string(),
            end_time: "23:59".to_string(),
        }
    }
    
    #[allow(dead_code)]
    pub fn format_time(&self) -> String {
        format!("{}-{}", self.start_time, self.end_time)
    }
    
    fn is_valid_time_format(time_str: &str) -> bool {
        if time_str.len() != 5 || !time_str.chars().nth(2).map_or(false, |c| c == ':') {
            return false;
        }
        
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return false;
        }
        
        if let (Ok(hour), Ok(minute)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
            hour <= 23 && minute <= 59
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WeeklyHours {
    pub monday: f64,
    pub tuesday: f64,
    pub wednesday: f64,
    pub thursday: f64,
    pub friday: f64,
    pub saturday: f64,
    pub sunday: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WeeklyTimeIntervals {
    pub monday: TimeInterval,
    pub tuesday: TimeInterval,
    pub wednesday: TimeInterval,
    pub thursday: TimeInterval,
    pub friday: TimeInterval,
    pub saturday: TimeInterval,
    pub sunday: TimeInterval,
}

impl WeeklyHours {

    pub fn validate(&self) -> Result<(), String> {
        for (day, hours) in [
            ("Monday", self.monday),
            ("Tuesday", self.tuesday),
            ("Wednesday", self.wednesday),
            ("Thursday", self.thursday),
            ("Friday", self.friday),
            ("Saturday", self.saturday),
            ("Sunday", self.sunday),
        ] {
            if hours < 0.0 || hours > 24.0 {
                return Err(format!("{} hours must be between 0 and 24, got {}", day, hours));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub user_id: i64,
    pub hours: WeeklyHours,
    pub intervals: WeeklyTimeIntervals,
    pub is_synced: bool,
    pub last_synced: Option<DateTime<Utc>>,
    pub last_modified: DateTime<Utc>,
}

impl WeeklyTimeIntervals {
    pub fn default() -> Self {
        Self {
            monday: TimeInterval::default(),
            tuesday: TimeInterval::default(),
            wednesday: TimeInterval::default(),
            thursday: TimeInterval::default(),
            friday: TimeInterval::default(),
            saturday: TimeInterval::default(),
            sunday: TimeInterval::default(),
        }
    }
}

impl Schedule {
    pub fn new(user_id: i64, hours: WeeklyHours) -> Result<Self, String> {
        hours.validate()?;
        
        Ok(Self {
            user_id,
            hours,
            intervals: WeeklyTimeIntervals::default(),
            is_synced: false, // New schedules always need sync
            last_synced: None,
            last_modified: Utc::now(),
        })
    }

    pub fn new_with_intervals(user_id: i64, hours: WeeklyHours, intervals: WeeklyTimeIntervals) -> Result<Self, String> {
        hours.validate()?;
        
        Ok(Self {
            user_id,
            hours,
            intervals,
            is_synced: false, // New schedules always need sync
            last_synced: None,
            last_modified: Utc::now(),
        })
    }

}

#[derive(Debug, Clone)]
pub struct TimeModification {
    pub user_id: i64,
    pub operation: String, // "+" or "-"
    pub seconds: i64,
}

impl TimeModification {
    pub fn new(user_id: i64, operation: String, seconds: i64) -> Result<Self, String> {
        if operation != "+" && operation != "-" {
            return Err("Operation must be '+' or '-'".to_string());
        }
        
        if seconds <= 0 {
            return Err("Seconds must be positive".to_string());
        }
        
        Ok(Self {
            user_id,
            operation,
            seconds,
        })
    }
}