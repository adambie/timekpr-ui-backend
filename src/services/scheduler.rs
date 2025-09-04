use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::{
    database::models::{ManagedUser, UserTimeUsage, UserWeeklySchedule, UserDailyTimeInterval},
    services::ssh::SSHClient,
};

#[derive(Clone)]
pub struct BackgroundScheduler {
    pool: SqlitePool,
    running: Arc<RwLock<bool>>,
}

impl BackgroundScheduler {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    pub async fn start(&self) {
        let mut running = self.running.write().await;
        if *running {
            tracing::info!("Background scheduler already running");
            return;
        }
        
        *running = true;
        drop(running);
        
        let scheduler = self.clone();
        tokio::spawn(async move {
            scheduler.run_loop().await;
        });
        
        tracing::info!("Background scheduler started");
    }
    
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        tracing::info!("Background scheduler stopped");
    }
    
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    async fn run_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        while *self.running.read().await {
            interval.tick().await;
            
            if let Err(e) = self.update_all_users().await {
                tracing::error!("Error in background update: {}", e);
            }
        }
        
        tracing::info!("Background scheduler loop ended");
    }
    
    async fn update_all_users(&self) -> anyhow::Result<()> {
        let users = ManagedUser::get_all(&self.pool).await?;
        tracing::info!("Processing {} users in background update", users.len());
        
        for user in users {
            if let Err(e) = self.update_user(&user).await {
                tracing::error!("Error updating user {}: {}", user.username, e);
            }
        }
        
        Ok(())
    }
    
    async fn update_user(&self, user: &ManagedUser) -> anyhow::Result<()> {
        tracing::info!("Processing user: {} @ {}", user.username, user.system_ip);
        
        let ssh_client = SSHClient::new(user.system_ip.clone());
        
        // Handle pending time adjustments
        if let (Some(adjustment), Some(operation)) = (&user.pending_time_adjustment, &user.pending_time_operation) {
            tracing::info!("Attempting to apply pending time adjustment for {}: {}{} seconds", user.username, operation, adjustment);
            
            match ssh_client.modify_time_left(&user.username, operation, *adjustment).await {
                Ok((true, _message)) => {
                    tracing::info!("Successfully applied pending time adjustment for {}", user.username);
                    ManagedUser::clear_pending_adjustment(&self.pool, user.id).await?;
                }
                Ok((false, message)) => {
                    tracing::warn!("Failed to apply pending time adjustment for {}: {}", user.username, message);
                }
                Err(e) => {
                    tracing::error!("SSH error for pending adjustment: {}", e);
                }
            }
        }
        
        // Handle pending weekly schedule sync
        if let Ok(Some(schedule)) = UserWeeklySchedule::get_by_user_id(&self.pool, user.id).await {
            if !schedule.is_synced {
                tracing::info!("Attempting to sync weekly schedule for {}", user.username);
                
                let schedule_dict = schedule.get_schedule_dict();
                match ssh_client.set_weekly_time_limits(&user.username, &schedule_dict).await {
                    Ok((true, _message)) => {
                        tracing::info!("Successfully synced weekly schedule for {}", user.username);
                        UserWeeklySchedule::mark_synced(&self.pool, user.id).await?;
                    }
                    Ok((false, message)) => {
                        tracing::warn!("Failed to sync weekly schedule for {}: {}", user.username, message);
                    }
                    Err(e) => {
                        tracing::error!("SSH error for weekly schedule sync: {}", e);
                    }
                }
            }
        }
        
        // Handle pending time interval syncs
        if let Ok(unsynced_intervals) = UserDailyTimeInterval::get_unsynced_by_user_id(&self.pool, user.id).await {
            if !unsynced_intervals.is_empty() {
                tracing::info!("Attempting to sync {} time intervals for {}", unsynced_intervals.len(), user.username);
                
                let all_intervals = UserDailyTimeInterval::get_by_user_id(&self.pool, user.id).await?;
                let mut intervals_dict = std::collections::HashMap::new();
                for interval in all_intervals {
                    intervals_dict.insert(interval.day_of_week, interval);
                }
                
                match ssh_client.set_allowed_hours(&user.username, &intervals_dict).await {
                    Ok((true, _message)) => {
                        tracing::info!("Successfully synced time intervals for {}", user.username);
                        for interval in unsynced_intervals {
                            UserDailyTimeInterval::mark_synced(&self.pool, interval.id).await?;
                        }
                    }
                    Ok((false, message)) => {
                        tracing::warn!("Failed to sync time intervals for {}: {}", user.username, message);
                    }
                    Err(e) => {
                        tracing::error!("SSH error for time intervals sync: {}", e);
                    }
                }
            }
        }
        
        // Update user info
        match ssh_client.validate_user(&user.username).await {
            Ok((is_valid, _message, config)) => {
                if is_valid && config.is_some() {
                    let config_json = serde_json::to_string(&config.as_ref().unwrap()).unwrap_or_default();
                    ManagedUser::update_validation(&self.pool, user.id, is_valid, Some(&config_json)).await?;
                    
                    // Update today's usage
                    if let Some(time_spent) = config.as_ref().and_then(|c| c.get("TIME_SPENT_DAY")).and_then(|v| v.as_i64()) {
                        let today = Utc::now().date_naive();
                        UserTimeUsage::upsert(&self.pool, user.id, today, time_spent).await?;
                    }
                } else {
                    // Just update last_checked time for connection failures
                    sqlx::query("UPDATE managed_user SET last_checked = ? WHERE id = ?")
                        .bind(Utc::now())
                        .bind(user.id)
                        .execute(&self.pool)
                        .await?;
                }
            }
            Err(e) => {
                tracing::error!("SSH validation error for {}: {}", user.username, e);
                // Update last_checked time even for failures
                sqlx::query("UPDATE managed_user SET last_checked = ? WHERE id = ?")
                    .bind(Utc::now())
                    .bind(user.id)
                    .execute(&self.pool)
                    .await?;
            }
        }
        
        Ok(())
    }
}