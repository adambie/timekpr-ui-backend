use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyHours {
    pub monday: f64,
    pub tuesday: f64,
    pub wednesday: f64,
    pub thursday: f64,
    pub friday: f64,
    pub saturday: f64,
    pub sunday: f64,
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
    pub is_synced: bool,
    pub last_synced: Option<DateTime<Utc>>,
    pub last_modified: DateTime<Utc>,
}

impl Schedule {
    pub fn new(user_id: i64, hours: WeeklyHours) -> Result<Self, String> {
        hours.validate()?;
        
        Ok(Self {
            user_id,
            hours,
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