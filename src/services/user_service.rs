use crate::models::{ManagedUser, ServiceError, UserData, AdminUserData};
use crate::repositories::UserRepository;
use crate::ssh::SSHClient;
use chrono::Utc;
use serde_json;
use std::sync::Arc;

pub struct UserService {
    repository: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(repository: Arc<dyn UserRepository>) -> Self {
        Self { repository }
    }

    pub async fn add_user(&self, username: String, system_ip: String) -> Result<String, ServiceError> {
        // Business logic: Check if user already exists
        let existing_users = self.repository.find_all().await?;
        for user in &existing_users {
            if user.username == username && user.system_ip == system_ip {
                return Err(ServiceError::ValidationError(
                    format!("User {} on {} already exists", username, system_ip)
                ));
            }
        }

        // Validate user with SSH and timekpr
        let ssh_client = SSHClient::new(&system_ip);
        let (is_valid, message, config) = ssh_client.validate_user(&username).await;
        
        let config_json = config.map(|c| c.to_string());

        // Create new user
        let new_user = ManagedUser {
            id: 0, // Will be set by database
            username: username.clone(),
            system_ip: system_ip.clone(),
            is_valid,
            date_added: Some(Utc::now()),
            last_checked: Some(Utc::now()),
            last_config: config_json,
            pending_time_adjustment: None,
            pending_time_operation: None,
        };

        self.repository.save(&new_user).await?;

        if is_valid {
            println!("Added and validated user: {} on {} - {}", username, system_ip, message);
            Ok(format!("User {} added and validated successfully", username))
        } else {
            println!("Added user: {} on {} but validation failed: {}", username, system_ip, message);
            Ok(format!("User {} added but validation failed: {}", username, message))
        }
    }

    pub async fn validate_user(&self, user_id: i64) -> Result<String, ServiceError> {
        let user = self.repository.find_by_id(user_id).await?
            .ok_or_else(|| ServiceError::NotFound("User not found".to_string()))?;

        // Validate with SSH and timekpr
        let ssh_client = SSHClient::new(&user.system_ip);
        let (is_valid, message, config) = ssh_client.validate_user(&user.username).await;
        
        let config_json = config.map(|c| c.to_string());
        
        let updated_user = ManagedUser {
            is_valid,
            last_checked: Some(Utc::now()),
            last_config: config_json,
            ..user
        };

        self.repository.save(&updated_user).await?;

        if is_valid {
            println!("Validated user: {} - {}", updated_user.username, message);
            Ok("User validation completed successfully".to_string())
        } else {
            println!("Validation failed for user: {} - {}", updated_user.username, message);
            Ok(format!("Validation failed: {}", message))
        }
    }

    pub async fn delete_user(&self, user_id: i64) -> Result<String, ServiceError> {
        let user = self.repository.find_by_id(user_id).await?
            .ok_or_else(|| ServiceError::NotFound("User not found".to_string()))?;

        let username = user.username.clone();
        self.repository.delete(user_id).await?;

        println!("Deleted user with id: {}", user_id);
        Ok(format!("User {} deleted successfully", username))
    }

    pub async fn get_dashboard_users(&self) -> Result<Vec<UserData>, ServiceError> {
        let users = self.repository.find_all_valid().await?;
        let mut user_data = Vec::new();

        for user in users {
            let time_left_formatted = if let Some(config_str) = &user.last_config {
                // Parse the JSON config to get actual time left
                if let Ok(config) = serde_json::from_str::<serde_json::Value>(config_str) {
                    if let Some(time_left) = config.get("TIME_LEFT_DAY").and_then(|v| v.as_i64()) {
                        let hours = time_left / 3600;
                        let minutes = (time_left % 3600) / 60;
                        format!("{}h {}m", hours, minutes)
                    } else {
                        "No limit set".to_string()
                    }
                } else {
                    "Unknown".to_string()
                }
            } else {
                "Unknown".to_string()
            };

            let last_checked_str = user.last_checked
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Never".to_string());

            let pending_adjustment = if let (Some(adjustment), Some(operation)) = 
                (&user.pending_time_adjustment, &user.pending_time_operation) {
                Some(format!("{}{} minutes", operation, adjustment / 60))
            } else {
                None
            };

            // TODO: Check for unsynced schedule changes via schedule service
            let pending_schedule = false; // Simplified for now

            println!("User {}: time_left_formatted = '{}', config = {:?}", user.username, time_left_formatted, user.last_config);
            
            user_data.push(UserData {
                id: user.id,
                username: user.username,
                system_ip: user.system_ip,
                time_left: time_left_formatted,
                last_checked: last_checked_str,
                pending_adjustment,
                pending_schedule,
            });
        }

        Ok(user_data)
    }

    pub async fn get_admin_users(&self) -> Result<Vec<AdminUserData>, ServiceError> {
        let users = self.repository.find_all().await?;
        let user_data = users
            .into_iter()
            .map(|user| {
                let last_checked_str = user.last_checked
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Never".to_string());

                AdminUserData {
                    id: user.id,
                    username: user.username,
                    system_ip: user.system_ip,
                    is_valid: user.is_valid,
                    last_checked: last_checked_str,
                }
            })
            .collect();

        Ok(user_data)
    }
}