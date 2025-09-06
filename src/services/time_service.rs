use crate::models::{TimeModification, ServiceError, ManagedUser};
use crate::repositories::UserRepository;
use crate::ssh::SSHClient;
use chrono::Utc;
use serde_json;
use std::sync::Arc;

pub struct TimeService {
    user_repository: Arc<dyn UserRepository>,
}

impl TimeService {
    pub fn new(user_repository: Arc<dyn UserRepository>) -> Self {
        Self { user_repository }
    }

    pub async fn modify_time(&self, modification: TimeModification) -> Result<TimeModificationResult, ServiceError> {
        // Get user from repository
        let user = self.user_repository.find_by_id(modification.user_id).await?
            .ok_or_else(|| ServiceError::NotFound("User not found".to_string()))?;

        // Try to apply the time modification via SSH
        let ssh_client = SSHClient::new(&user.system_ip);
        let (success, message) = ssh_client.modify_time_left(&user.username, &modification.operation, modification.seconds).await;
        
        if success {
            // Command succeeded, update user info and clear pending adjustments
            let ssh_client = SSHClient::new(&user.system_ip);
            let (is_valid, _, config) = ssh_client.validate_user(&user.username).await;
            
            if is_valid {
                let config_json = config.map(|c| c.to_string());
                let updated_user = ManagedUser {
                    last_checked: Some(Utc::now()),
                    last_config: config_json,
                    pending_time_adjustment: None,
                    pending_time_operation: None,
                    ..user.clone()
                };
                self.user_repository.save(&updated_user).await?;
            }
            
            println!("Applied time adjustment: {}{}s for user {} - {}", 
                modification.operation, modification.seconds, user.username, message);
            
            Ok(TimeModificationResult {
                success: true,
                message,
                username: user.username,
                pending: false,
            })
        } else {
            // Command failed, store as pending adjustment
            self.user_repository
                .update_pending_time_adjustment(modification.user_id, &modification.operation, modification.seconds)
                .await?;

            println!("Queued time adjustment: {}{}s for user {} - SSH failed: {}", 
                modification.operation, modification.seconds, user.username, message);
            
            Ok(TimeModificationResult {
                success: true,
                message: format!("Computer seems to be offline. Time adjustment of {}{}s has been queued and will be applied when the computer comes online.", 
                    modification.operation, modification.seconds),
                username: user.username,
                pending: true,
            })
        }
    }

    pub async fn get_user_usage(&self, user_id: i64) -> Result<UsageData, ServiceError> {
        let user = self.user_repository.find_by_id(user_id).await?
            .ok_or_else(|| ServiceError::NotFound("User not found".to_string()))?;

        // TODO: This should be moved to a usage repository
        // For now, keeping the existing logic but will refactor later
        let mut usage_data = Vec::new();
        
        for i in 0..7 {
            let date = Utc::now().date_naive() - chrono::Duration::days(6 - i);
            
            // TODO: Replace direct SQL with repository call
            // let time_spent = self.usage_repository.get_time_spent(user_id, date).await?;
            let time_spent = 0; // Simplified for now
            
            usage_data.push(serde_json::json!({
                "date": date.to_string(),
                "hours": (time_spent as f64) / 3600.0
            }));
        }

        Ok(UsageData {
            username: user.username,
            usage_data,
        })
    }
}

#[derive(serde::Serialize)]
pub struct TimeModificationResult {
    pub success: bool,
    pub message: String,
    pub username: String,
    pub pending: bool,
}

#[derive(serde::Serialize)]
pub struct UsageData {
    pub username: String,
    pub usage_data: Vec<serde_json::Value>,
}