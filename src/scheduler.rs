use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};
use crate::ssh::SSHClient;
use crate::services::{UserService, UsageService, ScheduleService};

pub struct BackgroundScheduler {
    user_service: Arc<UserService>,
    usage_service: Arc<UsageService>,
    schedule_service: Arc<ScheduleService>,
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl BackgroundScheduler {
    pub fn new(user_service: Arc<UserService>, usage_service: Arc<UsageService>, schedule_service: Arc<ScheduleService>) -> Self {
        Self {
            user_service,
            usage_service,
            schedule_service,
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            return;
        }
        *running = true;
        
        let user_service = Arc::clone(&self.user_service); 
        let usage_service = Arc::clone(&self.usage_service);
        let schedule_service = Arc::clone(&self.schedule_service);
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
                Self::update_users_task(&user_service, &usage_service).await;
                
                // Process pending time adjustments
                Self::process_pending_adjustments(&user_service).await;
                
                // Sync pending schedule changes
                Self::sync_pending_schedules(&user_service, &schedule_service).await;
            }
        });
    }


    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    async fn update_users_task(user_service: &UserService, usage_service: &UsageService) {
        let users = user_service.get_valid_users().await;

        match users {
            Ok(users) => {
                for user in users {
                    let ssh_client = SSHClient::new(&user.system_ip);
                    let (is_reachable, _message, config) = ssh_client.validate_user(&user.username).await;
                    
                    if is_reachable {
                        // Update user data with config
                        let config_json = config.as_ref().map(|c| c.to_string());
                        let _ = user_service.update_background_data(user.id, config_json).await;

                        // Store usage data if available
                        if let Some(config) = &config {
                            if let Some(time_spent) = config.get("TIME_SPENT_DAY").and_then(|v| v.as_i64()) {
                                let _ = usage_service.store_daily_usage(user.id, time_spent).await;
                            }
                        }
                    } else {
                        // Just update last_checked timestamp
                        let _ = user_service.update_last_checked(user.id).await;
                    }

                    sleep(Duration::from_millis(100)).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch users for background update: {}", e);
            }
        }
    }

    async fn process_pending_adjustments(user_service: &UserService) {
        // Get users with pending time adjustments
        let users = user_service.get_users_pending().await;

        match users {
            Ok(users) => {
                for user in users {
                    if let (Some(adjustment), Some(operation)) = (&user.pending_time_adjustment, &user.pending_time_operation) {
                        let ssh_client = SSHClient::new(&user.system_ip);
                        let (success, _message) = ssh_client.modify_time_left(&user.username, operation, *adjustment).await;
                        
                        if success {
                            // Clear pending adjustment
                            let _ = user_service.clear_pending_adjustements(user.id).await;
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

    async fn sync_pending_schedules(user_service: &UserService, schedule_service: &ScheduleService) {
        let unsynced_schedules = schedule_service.get_unsynced_schedules().await;
        
        match unsynced_schedules {
            Ok(schedules) => {
                for schedule in schedules {
                    // Get user data for this schedule
                    if let Ok(Some(user)) = user_service.find_by_id(schedule.user_id).await {
                        // Only sync for valid users
                        if user.is_valid {
                            let ssh_client = SSHClient::new(&user.system_ip);
                            
                            // Use service method to prepare sync data
                            let (schedule_dict, intervals_dict) = schedule_service.prepare_sync_data(&schedule);
                            
                            // Sync operations
                            let (limits_success, limits_message) = ssh_client.set_weekly_time_limits(&user.username, &schedule_dict).await;
                            let (hours_success, hours_message) = ssh_client.set_weekly_allowed_hours(&user.username, &intervals_dict).await;
                            
                            let success = limits_success && hours_success;
                            
                            if success {
                                println!("Schedule sync successful for {}: {}, {}", user.username, limits_message, hours_message);
                                let _ = schedule_service.mark_as_synced(schedule.user_id).await;
                            } else {
                                // Log what failed
                                let mut error_parts = Vec::new();
                                if !limits_success {
                                    error_parts.push(format!("Time limits: {}", limits_message));
                                }
                                if !hours_success {
                                    error_parts.push(format!("Allowed hours: {}", hours_message));
                                }
                                println!("Schedule sync failed for {}: {}", user.username, error_parts.join(", "));
                            }
                            
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to fetch unsynced schedules: {}", e);
            }
        }
    }
}