use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use sqlx::SqlitePool;
use chrono::Utc;
use crate::ssh::SSHClient;
use crate::ManagedUser;

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
            println!("Background scheduler is already running");
            return;
        }
        *running = true;
        
        let pool = Arc::clone(&self.pool);
        let running_flag = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            println!("Background scheduler started");
            let mut interval = interval(Duration::from_secs(30)); // Run every 30 seconds
            
            loop {
                interval.tick().await;
                
                // Check if we should still be running
                {
                    let running = running_flag.read().await;
                    if !*running {
                        println!("Background scheduler stopped");
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

    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        println!("Background scheduler stop requested");
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
                    // Validate user and update their data
                    let ssh_client = SSHClient::new(&user.system_ip);
                    let (is_valid, _message, config) = ssh_client.validate_user(&user.username).await;
                    
                    let config_json = config.as_ref().map(|c| c.to_string());
                    
                    let _result = sqlx::query(
                        "UPDATE managed_users SET last_checked = ?, is_valid = ?, last_config = ? WHERE id = ?"
                    )
                    .bind(Utc::now())
                    .bind(is_valid)
                    .bind(&config_json)
                    .bind(user.id)
                    .execute(pool)
                    .await;

                    // Store today's usage data if available
                    if is_valid && config.is_some() {
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

                    if is_valid {
                        println!("Updated user: {} on {}", user.username, user.system_ip);
                    } else {
                        println!("Failed to update user: {} on {}", user.username, user.system_ip);
                    }
                    
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
                        let (success, message) = ssh_client.modify_time_left(&user.username, operation, *adjustment).await;
                        
                        if success {
                            // Clear pending adjustment
                            let _result = sqlx::query(
                                "UPDATE managed_users SET pending_time_adjustment = NULL, pending_time_operation = NULL, last_checked = ? WHERE id = ?"
                            )
                            .bind(Utc::now())
                            .bind(user.id)
                            .execute(pool)
                            .await;
                            
                            println!("Applied pending adjustment for {}: {}{} seconds - {}", user.username, operation, adjustment, message);
                        } else {
                            println!("Failed to apply pending adjustment for {}: {}", user.username, message);
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
        // Get users with unsynced schedules
        let users_with_schedules = sqlx::query!(
            "SELECT u.id, u.username, u.system_ip, u.is_valid,
                    s.monday_hours, s.tuesday_hours, s.wednesday_hours, s.thursday_hours, 
                    s.friday_hours, s.saturday_hours, s.sunday_hours
             FROM managed_users u 
             JOIN user_weekly_schedule s ON u.id = s.user_id 
             WHERE u.is_valid = 1 AND s.is_synced = 0"
        )
        .fetch_all(pool)
        .await;

        match users_with_schedules {
            Ok(schedules) => {
                for schedule in schedules {
                    println!("Syncing schedule for user {} on {}", schedule.username, schedule.system_ip);
                    
                    // Create schedule dict with non-null values only
                    let mut schedule_dict = std::collections::HashMap::new();
                    if let Some(hours) = schedule.monday_hours { schedule_dict.insert("monday".to_string(), hours); }
                    if let Some(hours) = schedule.tuesday_hours { schedule_dict.insert("tuesday".to_string(), hours); }
                    if let Some(hours) = schedule.wednesday_hours { schedule_dict.insert("wednesday".to_string(), hours); }
                    if let Some(hours) = schedule.thursday_hours { schedule_dict.insert("thursday".to_string(), hours); }
                    if let Some(hours) = schedule.friday_hours { schedule_dict.insert("friday".to_string(), hours); }
                    if let Some(hours) = schedule.saturday_hours { schedule_dict.insert("saturday".to_string(), hours); }
                    if let Some(hours) = schedule.sunday_hours { schedule_dict.insert("sunday".to_string(), hours); }
                    
                    let ssh_client = SSHClient::new(&schedule.system_ip);
                    let (success, message) = ssh_client.set_weekly_time_limits(&schedule.username, &schedule_dict).await;
                    
                    if success {
                        println!("Successfully synced schedule for {}: {}", schedule.username, message);
                        
                        // Mark as synced
                        let _result = sqlx::query(
                            "UPDATE user_weekly_schedule SET is_synced = 1, last_synced = ? WHERE user_id = ?"
                        )
                        .bind(Utc::now())
                        .bind(schedule.id)
                        .execute(pool)
                        .await;
                    } else {
                        println!("Failed to sync schedule for {}: {}", schedule.username, message);
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