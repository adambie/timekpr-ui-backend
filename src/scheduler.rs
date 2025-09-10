use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use sqlx::SqlitePool;
use chrono::Utc;
use crate::ssh::SSHClient;
use crate::models::ManagedUser;
pub struct BackgroundScheduler {
    pool: Arc<SqlitePool>,
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl BackgroundScheduler {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: Arc::new(pool),
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            return;
        }
        *running = true;
        
        let pool = Arc::clone(&self.pool);
        let running_flag = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Run every 30 seconds
            
            loop {
                interval.tick().await;
                
                // Check if we should still be running
                {
                    let running = running_flag.read().await;
                    if !*running {
                        break;
                    }
                }
                
                // Update user data
                Self::update_users_task(&pool).await;
                
                // Process pending time adjustments
                Self::process_pending_adjustments(&pool).await;
                
                // Sync pending schedule changes
                Self::sync_pending_schedules(&pool).await;
            }
        });
    }


    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    async fn update_users_task(pool: &SqlitePool) {
        // Get all managed users
        let users = sqlx::query_as::<_, ManagedUser>(
            "SELECT * FROM managed_users WHERE is_valid = 1"
        )
        .fetch_all(pool)
        .await;

        match users {
            Ok(users) => {
                for user in users {
                    // Try to get user data but don't change validation status
                    let ssh_client = SSHClient::new(&user.system_ip);
                    let (is_reachable, _message, config) = ssh_client.validate_user(&user.username).await;
                    
                    if is_reachable {
                        // Only update data if connection was successful - don't change is_valid status
                        let config_json = config.as_ref().map(|c| c.to_string());
                        
                        let _result = sqlx::query(
                            "UPDATE managed_users SET last_checked = ?, last_config = ? WHERE id = ?"
                        )
                        .bind(Utc::now())
                        .bind(&config_json)
                        .bind(user.id)
                        .execute(pool)
                        .await;
                    } else {
                        // Just update last_checked timestamp to show we tried, but don't invalidate user
                        let _result = sqlx::query(
                            "UPDATE managed_users SET last_checked = ? WHERE id = ?"
                        )
                        .bind(Utc::now())
                        .bind(user.id)
                        .execute(pool)
                        .await;
                    }

                    // Store today's usage data if available
                    if is_reachable && config.is_some() {
                        if let Some(time_spent) = config.as_ref().unwrap().get("TIME_SPENT_DAY").and_then(|v| v.as_i64()) {
                            let today = Utc::now().date_naive();
                            let _usage_result = sqlx::query(
                                "INSERT OR REPLACE INTO user_time_usage (user_id, date, time_spent) VALUES (?, ?, ?)"
                            )
                            .bind(user.id)
                            .bind(today)
                            .bind(time_spent)
                            .execute(pool)
                            .await;
                        }
                    }

                    // Silent background updates - only log errors if needed
                    
                    // Small delay between users to avoid overwhelming SSH connections
                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch users for background update: {}", e);
            }
        }
    }

    async fn process_pending_adjustments(pool: &SqlitePool) {
        // Get users with pending time adjustments
        let users = sqlx::query_as::<_, ManagedUser>(
            "SELECT * FROM managed_users WHERE pending_time_adjustment IS NOT NULL AND pending_time_operation IS NOT NULL"
        )
        .fetch_all(pool)
        .await;

        match users {
            Ok(users) => {
                for user in users {
                    if let (Some(adjustment), Some(operation)) = (&user.pending_time_adjustment, &user.pending_time_operation) {
                        let ssh_client = SSHClient::new(&user.system_ip);
                        let (success, _message) = ssh_client.modify_time_left(&user.username, operation, *adjustment).await;
                        
                        if success {
                            // Clear pending adjustment
                            let _result = sqlx::query(
                                "UPDATE managed_users SET pending_time_adjustment = NULL, pending_time_operation = NULL, last_checked = ? WHERE id = ?"
                            )
                            .bind(Utc::now())
                            .bind(user.id)
                            .execute(pool)
                            .await;
                        }
                    }
                    
                    // Small delay between operations
                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch users with pending adjustments: {}", e);
            }
        }
    }

    async fn sync_pending_schedules(pool: &SqlitePool) {
        // Get users with unsynced schedules (including time intervals)
        let users_with_schedules = sqlx::query!(
            "SELECT u.id, u.username, u.system_ip, u.is_valid,
                    s.monday_hours, s.tuesday_hours, s.wednesday_hours, s.thursday_hours, 
                    s.friday_hours, s.saturday_hours, s.sunday_hours,
                    s.monday_start_time, s.monday_end_time,
                    s.tuesday_start_time, s.tuesday_end_time,
                    s.wednesday_start_time, s.wednesday_end_time,
                    s.thursday_start_time, s.thursday_end_time,
                    s.friday_start_time, s.friday_end_time,
                    s.saturday_start_time, s.saturday_end_time,
                    s.sunday_start_time, s.sunday_end_time
             FROM managed_users u 
             JOIN user_weekly_schedule s ON u.id = s.user_id 
             WHERE u.is_valid = 1 AND s.is_synced = 0"
        )
        .fetch_all(pool)
        .await;

        match users_with_schedules {
            Ok(schedules) => {
                for schedule in schedules {
                    let ssh_client = SSHClient::new(&schedule.system_ip);
                    
                    // Create time limits dict with non-null values only
                    let mut schedule_dict = std::collections::HashMap::new();
                    if let Some(hours) = schedule.monday_hours { schedule_dict.insert("monday".to_string(), hours); }
                    if let Some(hours) = schedule.tuesday_hours { schedule_dict.insert("tuesday".to_string(), hours); }
                    if let Some(hours) = schedule.wednesday_hours { schedule_dict.insert("wednesday".to_string(), hours); }
                    if let Some(hours) = schedule.thursday_hours { schedule_dict.insert("thursday".to_string(), hours); }
                    if let Some(hours) = schedule.friday_hours { schedule_dict.insert("friday".to_string(), hours); }
                    if let Some(hours) = schedule.saturday_hours { schedule_dict.insert("saturday".to_string(), hours); }
                    if let Some(hours) = schedule.sunday_hours { schedule_dict.insert("sunday".to_string(), hours); }
                    
                    // Create time intervals dict
                    let mut intervals_dict = std::collections::HashMap::new();
                    if let (Some(start), Some(end)) = (&schedule.monday_start_time, &schedule.monday_end_time) {
                        intervals_dict.insert("monday".to_string(), (start.clone(), end.clone()));
                    }
                    if let (Some(start), Some(end)) = (&schedule.tuesday_start_time, &schedule.tuesday_end_time) {
                        intervals_dict.insert("tuesday".to_string(), (start.clone(), end.clone()));
                    }
                    if let (Some(start), Some(end)) = (&schedule.wednesday_start_time, &schedule.wednesday_end_time) {
                        intervals_dict.insert("wednesday".to_string(), (start.clone(), end.clone()));
                    }
                    if let (Some(start), Some(end)) = (&schedule.thursday_start_time, &schedule.thursday_end_time) {
                        intervals_dict.insert("thursday".to_string(), (start.clone(), end.clone()));
                    }
                    if let (Some(start), Some(end)) = (&schedule.friday_start_time, &schedule.friday_end_time) {
                        intervals_dict.insert("friday".to_string(), (start.clone(), end.clone()));
                    }
                    if let (Some(start), Some(end)) = (&schedule.saturday_start_time, &schedule.saturday_end_time) {
                        intervals_dict.insert("saturday".to_string(), (start.clone(), end.clone()));
                    }
                    if let (Some(start), Some(end)) = (&schedule.sunday_start_time, &schedule.sunday_end_time) {
                        intervals_dict.insert("sunday".to_string(), (start.clone(), end.clone()));
                    }
                    
                    // Set time limits first
                    let (limits_success, limits_message) = ssh_client.set_weekly_time_limits(&schedule.username, &schedule_dict).await;
                    
                    // Set allowed hours/intervals
                    let (hours_success, hours_message) = ssh_client.set_weekly_allowed_hours(&schedule.username, &intervals_dict).await;
                    
                    // Consider success if both operations succeed
                    let success = limits_success && hours_success;
                    
                    if success {
                        println!("Schedule sync successful for {}: {}, {}", schedule.username, limits_message, hours_message);
                        // Mark as synced
                        match sqlx::query(
                            "UPDATE user_weekly_schedule SET is_synced = 1, last_synced = ? WHERE user_id = ?"
                        )
                        .bind(Utc::now())
                        .bind(schedule.id)
                        .execute(pool)
                        .await {
                            Ok(result) => {
                                if result.rows_affected() == 0 {
                                    eprintln!("Warning: No schedule found to mark as synced for user_id {}", schedule.id);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to mark schedule as synced for user_id {}: {}", schedule.id, e);
                            }
                        }
                    } else {
                        // Log what failed
                        let mut error_parts = Vec::new();
                        if !limits_success {
                            error_parts.push(format!("Time limits: {}", limits_message));
                        }
                        if !hours_success {
                            error_parts.push(format!("Allowed hours: {}", hours_message));
                        }
                        println!("Schedule sync failed for {}: {}", schedule.username, error_parts.join(", "));
                    }
                    
                    // Small delay between syncs
                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch schedules for sync: {}", e);
            }
        }
    }
}