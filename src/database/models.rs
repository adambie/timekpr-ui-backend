use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Settings {
    pub id: i64,
    pub key: String,
    pub value: String,
}

impl Settings {
    pub async fn get_value(pool: &SqlitePool, key: &str) -> sqlx::Result<Option<String>> {
        let result = sqlx::query_as::<_, Settings>(
            "SELECT id, key, value FROM settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;
        
        Ok(result.map(|s| s.value))
    }
    
    pub async fn set_value(pool: &SqlitePool, key: &str, value: &str) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)"
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn set_admin_password(pool: &SqlitePool, password: &str) -> sqlx::Result<()> {
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|_| sqlx::Error::ColumnDecode { index: "password".to_string(), source: Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Failed to hash password")) })?;
        
        Self::set_value(pool, "admin_password_hash", &password_hash.to_string()).await?;
        
        // Remove old plain text password if it exists
        sqlx::query("DELETE FROM settings WHERE key = 'admin_password'")
            .execute(pool)
            .await?;
        
        Ok(())
    }
    
    pub async fn check_admin_password(pool: &SqlitePool, password: &str) -> sqlx::Result<bool> {
        use argon2::{Argon2, PasswordVerifier};
        use argon2::password_hash::PasswordHash;
        
        if let Some(hash_str) = Self::get_value(pool, "admin_password_hash").await? {
            let parsed_hash = PasswordHash::new(&hash_str)
                .map_err(|_| sqlx::Error::ColumnDecode { index: "password".to_string(), source: Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid password hash")) })?;
            
            let argon2 = Argon2::default();
            Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
        } else {
            // Check for legacy plain text password
            if let Some(old_password) = Self::get_value(pool, "admin_password").await? {
                if password == old_password {
                    // Migrate to hashed password
                    Self::set_admin_password(pool, password).await?;
                    return Ok(true);
                }
            } else {
                // Initialize with default password
                Self::set_admin_password(pool, "admin").await?;
                return Ok(password == "admin");
            }
            Ok(false)
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ManagedUser {
    pub id: i64,
    pub username: String,
    pub system_ip: String,
    pub is_valid: bool,
    pub date_added: DateTime<Utc>,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_config: Option<String>,
    pub pending_time_adjustment: Option<i64>,
    pub pending_time_operation: Option<String>,
}

impl ManagedUser {
    pub async fn get_all(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_user"
        )
        .fetch_all(pool)
        .await
    }
    
    pub async fn get_valid_users(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_user WHERE is_valid = true"
        )
        .fetch_all(pool)
        .await
    }
    
    pub async fn find_by_username_and_ip(pool: &SqlitePool, username: &str, system_ip: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_user WHERE username = ? AND system_ip = ?"
        )
        .bind(username)
        .bind(system_ip)
        .fetch_optional(pool)
        .await
    }
    
    pub async fn create(pool: &SqlitePool, username: &str, system_ip: &str) -> sqlx::Result<i64> {
        let result = sqlx::query(
            "INSERT INTO managed_user (username, system_ip, is_valid, date_added) VALUES (?, ?, false, ?)"
        )
        .bind(username)
        .bind(system_ip)
        .bind(Utc::now())
        .execute(pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }
    
    pub async fn update_validation(pool: &SqlitePool, id: i64, is_valid: bool, last_config: Option<&str>) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE managed_user SET is_valid = ?, last_checked = ?, last_config = ? WHERE id = ?"
        )
        .bind(is_valid)
        .bind(Utc::now())
        .bind(last_config)
        .bind(id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn clear_pending_adjustment(pool: &SqlitePool, id: i64) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE managed_user SET pending_time_adjustment = NULL, pending_time_operation = NULL WHERE id = ?"
        )
        .bind(id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn set_pending_adjustment(pool: &SqlitePool, id: i64, seconds: i64, operation: &str) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE managed_user SET pending_time_adjustment = ?, pending_time_operation = ? WHERE id = ?"
        )
        .bind(seconds)
        .bind(operation)
        .bind(id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn delete_by_id(pool: &SqlitePool, id: i64) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM managed_user WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        
        Ok(())
    }
    
    pub fn get_config_value(&self, key: &str) -> Option<serde_json::Value> {
        if let Some(ref config_str) = self.last_config {
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(config_str) {
                return config.get(key).cloned();
            }
        }
        None
    }
    
    pub async fn get_recent_usage(&self, pool: &SqlitePool, days: i32) -> sqlx::Result<HashMap<String, i64>> {
        let today = Utc::now().date_naive();
        let start_date = today - chrono::Duration::days(days as i64 - 1);
        
        let records = UserTimeUsage::get_by_user_and_date_range(pool, self.id, start_date, today).await?;
        
        let mut usage_dict = HashMap::new();
        
        // Initialize all days with 0
        for i in 0..days {
            let date = start_date + chrono::Duration::days(i as i64);
            usage_dict.insert(date.format("%Y-%m-%d").to_string(), 0);
        }
        
        // Fill in actual data
        for record in records {
            let date_str = record.date.format("%Y-%m-%d").to_string();
            usage_dict.insert(date_str, record.time_spent);
        }
        
        Ok(usage_dict)
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserTimeUsage {
    pub id: i64,
    pub user_id: i64,
    pub date: NaiveDate,
    pub time_spent: i64,
}

impl UserTimeUsage {
    pub async fn get_by_user_and_date_range(
        pool: &SqlitePool, 
        user_id: i64, 
        start_date: NaiveDate, 
        end_date: NaiveDate
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT id, user_id, date, time_spent FROM user_time_usage 
             WHERE user_id = ? AND date >= ? AND date <= ? ORDER BY date"
        )
        .bind(user_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await
    }
    
    pub async fn upsert(pool: &SqlitePool, user_id: i64, date: NaiveDate, time_spent: i64) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO user_time_usage (user_id, date, time_spent) VALUES (?, ?, ?)"
        )
        .bind(user_id)
        .bind(date)
        .bind(time_spent)
        .execute(pool)
        .await?;
        
        Ok(())
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserWeeklySchedule {
    pub id: i64,
    pub user_id: i64,
    pub monday_hours: f64,
    pub tuesday_hours: f64,
    pub wednesday_hours: f64,
    pub thursday_hours: f64,
    pub friday_hours: f64,
    pub saturday_hours: f64,
    pub sunday_hours: f64,
    pub is_synced: bool,
    pub last_synced: Option<DateTime<Utc>>,
    pub last_modified: DateTime<Utc>,
}

impl UserWeeklySchedule {
    pub async fn get_by_user_id(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Option<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT id, user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours, friday_hours, saturday_hours, sunday_hours, is_synced, last_synced, last_modified 
             FROM user_weekly_schedule WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
    }
    
    pub async fn create_or_update(pool: &SqlitePool, user_id: i64, schedule: &HashMap<String, f64>) -> sqlx::Result<()> {
        let now = Utc::now();
        
        sqlx::query(
            "INSERT OR REPLACE INTO user_weekly_schedule 
             (user_id, monday_hours, tuesday_hours, wednesday_hours, thursday_hours, friday_hours, saturday_hours, sunday_hours, is_synced, last_modified)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, false, ?)"
        )
        .bind(user_id)
        .bind(schedule.get("monday").unwrap_or(&0.0))
        .bind(schedule.get("tuesday").unwrap_or(&0.0))
        .bind(schedule.get("wednesday").unwrap_or(&0.0))
        .bind(schedule.get("thursday").unwrap_or(&0.0))
        .bind(schedule.get("friday").unwrap_or(&0.0))
        .bind(schedule.get("saturday").unwrap_or(&0.0))
        .bind(schedule.get("sunday").unwrap_or(&0.0))
        .bind(now)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn mark_synced(pool: &SqlitePool, user_id: i64) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE user_weekly_schedule SET is_synced = true, last_synced = ? WHERE user_id = ?"
        )
        .bind(Utc::now())
        .bind(user_id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub fn get_schedule_dict(&self) -> HashMap<String, f64> {
        let mut schedule = HashMap::new();
        schedule.insert("monday".to_string(), self.monday_hours);
        schedule.insert("tuesday".to_string(), self.tuesday_hours);
        schedule.insert("wednesday".to_string(), self.wednesday_hours);
        schedule.insert("thursday".to_string(), self.thursday_hours);
        schedule.insert("friday".to_string(), self.friday_hours);
        schedule.insert("saturday".to_string(), self.saturday_hours);
        schedule.insert("sunday".to_string(), self.sunday_hours);
        schedule
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserDailyTimeInterval {
    pub id: i64,
    pub user_id: i64,
    pub day_of_week: i32,
    pub start_hour: i32,
    pub start_minute: i32,
    pub end_hour: i32,
    pub end_minute: i32,
    pub is_enabled: bool,
    pub is_synced: bool,
    pub last_synced: Option<DateTime<Utc>>,
    pub last_modified: DateTime<Utc>,
}

impl UserDailyTimeInterval {
    pub async fn get_by_user_id(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT id, user_id, day_of_week, start_hour, start_minute, end_hour, end_minute, is_enabled, is_synced, last_synced, last_modified 
             FROM user_daily_time_interval WHERE user_id = ? ORDER BY day_of_week"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
    
    pub async fn get_unsynced_by_user_id(pool: &SqlitePool, user_id: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT id, user_id, day_of_week, start_hour, start_minute, end_hour, end_minute, is_enabled, is_synced, last_synced, last_modified 
             FROM user_daily_time_interval WHERE user_id = ? AND is_synced = false ORDER BY day_of_week"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
    
    pub async fn upsert(pool: &SqlitePool, user_id: i64, day_of_week: i32, interval: &Self) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO user_daily_time_interval 
             (user_id, day_of_week, start_hour, start_minute, end_hour, end_minute, is_enabled, is_synced, last_modified)
             VALUES (?, ?, ?, ?, ?, ?, ?, false, ?)"
        )
        .bind(user_id)
        .bind(day_of_week)
        .bind(interval.start_hour)
        .bind(interval.start_minute)
        .bind(interval.end_hour)
        .bind(interval.end_minute)
        .bind(interval.is_enabled)
        .bind(Utc::now())
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub async fn mark_synced(pool: &SqlitePool, id: i64) -> sqlx::Result<()> {
        sqlx::query(
            "UPDATE user_daily_time_interval SET is_synced = true, last_synced = ? WHERE id = ?"
        )
        .bind(Utc::now())
        .bind(id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    pub fn get_day_name(&self) -> &'static str {
        match self.day_of_week {
            1 => "Monday",
            2 => "Tuesday", 
            3 => "Wednesday",
            4 => "Thursday",
            5 => "Friday",
            6 => "Saturday",
            7 => "Sunday",
            _ => "Unknown",
        }
    }
    
    pub fn is_valid_interval(&self) -> bool {
        let start_minutes = self.start_hour * 60 + self.start_minute;
        let end_minutes = self.end_hour * 60 + self.end_minute;
        start_minutes < end_minutes && 
        start_minutes >= 0 && start_minutes < 1440 &&
        end_minutes >= 0 && end_minutes < 1440
    }
    
    pub fn get_time_range_string(&self) -> String {
        format!("{:02}:{:02}-{:02}:{:02}", 
                self.start_hour, self.start_minute,
                self.end_hour, self.end_minute)
    }
    
    pub fn to_timekpr_format(&self) -> Option<Vec<String>> {
        if !self.is_enabled {
            return None;
        }
        
        let mut result = Vec::new();
        
        if self.start_minute == 0 && self.end_minute == 0 {
            // Full hour intervals
            for h in self.start_hour..self.end_hour {
                result.push(h.to_string());
            }
        } else {
            // Handle partial hours
            if self.start_hour == self.end_hour {
                result.push(format!("{}[{}-{}]", self.start_hour, self.start_minute, self.end_minute));
            } else {
                // First hour
                if self.start_minute == 0 {
                    result.push(self.start_hour.to_string());
                } else {
                    result.push(format!("{}[{}-59]", self.start_hour, self.start_minute));
                }
                
                // Full hours in between
                for h in (self.start_hour + 1)..self.end_hour {
                    result.push(h.to_string());
                }
                
                // Last hour
                if self.end_minute > 0 {
                    result.push(format!("{}[0-{}]", self.end_hour, self.end_minute));
                }
            }
        }
        
        Some(result)
    }
}