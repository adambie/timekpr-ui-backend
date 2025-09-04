use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub struct SSHClient {
    hostname: String,
    username: String,
    key_path: String,
    port: u16,
}

impl SSHClient {
    pub fn new(hostname: String) -> Self {
        let username = "timekpr-remote".to_string();
        let key_path = Self::find_key_path();
        let port = 22;
        
        Self {
            hostname,
            username,
            key_path,
            port,
        }
    }
    
    fn find_key_path() -> String {
        let possible_paths = [
            "/app/ssh/timekpr_ui_key",
            "ssh/timekpr_ui_key",
            "./ssh/timekpr_ui_key",
        ];
        
        for path in &possible_paths {
            if Path::new(path).exists() {
                return path.to_string();
            }
        }
        
        "ssh/timekpr_ui_key".to_string()
    }
    
    pub fn check_ssh_keys_exist() -> bool {
        let possible_paths = [
            "/app/ssh/timekpr_ui_key",
            "ssh/timekpr_ui_key", 
            "./ssh/timekpr_ui_key",
        ];
        
        for path in &possible_paths {
            if Path::new(path).exists() {
                return true;
            }
        }
        
        false
    }
    
    pub async fn validate_user(&self, username: &str) -> Result<(bool, String, Option<HashMap<String, serde_json::Value>>)> {
        let command = format!("timekpra --userinfo {}", username);
        
        match self.execute_ssh_command(&command).await {
            Ok((exit_code, output, _stderr)) => {
                if output.contains(&format!("User \"{}\" configuration is not found", username)) {
                    return Ok((false, format!("User '{}' not found on system", username), None));
                }
                
                let config_dict = self.parse_timekpr_output(&output);
                Ok((true, output, Some(config_dict)))
            }
            Err(e) => Ok((false, format!("Connection error: {}", e), None)),
        }
    }
    
    pub async fn modify_time_left(&self, username: &str, operation: &str, seconds: i64) -> Result<(bool, String)> {
        if !matches!(operation, "+" | "-") {
            return Ok((false, "Invalid operation. Must be '+' or '-'".to_string()));
        }
        
        let command = format!("timekpra --settimeleft {} {} {}", username, operation, seconds);
        
        match self.execute_ssh_command(&command).await {
            Ok((exit_code, output, stderr)) => {
                if exit_code == 0 {
                    Ok((true, format!("Successfully modified time for {}: {}{} seconds", username, operation, seconds)))
                } else {
                    Ok((false, format!("Error modifying time: {}", stderr)))
                }
            }
            Err(e) => Ok((false, format!("Connection error: {}", e))),
        }
    }
    
    pub async fn set_weekly_time_limits(&self, username: &str, schedule: &HashMap<String, f64>) -> Result<(bool, String)> {
        let day_order = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
        let mut allowed_days = Vec::new();
        let mut time_limits = Vec::new();
        
        for (i, day) in day_order.iter().enumerate() {
            if let Some(&hours) = schedule.get(*day) {
                if hours > 0.0 {
                    allowed_days.push((i + 1).to_string());
                    time_limits.push(((hours * 3600.0) as i64).to_string());
                }
            }
        }
        
        if allowed_days.is_empty() {
            return Ok((false, "No days with time limits configured".to_string()));
        }
        
        // Set allowed days
        let allowed_days_string = allowed_days.join(";");
        let days_command = format!("timekpra --setalloweddays {} '{}'", username, allowed_days_string);
        
        if let Err(e) = self.execute_ssh_command(&days_command).await {
            let sudo_command = format!("sudo {}", days_command);
            if let Err(_) = self.execute_ssh_command(&sudo_command).await {
                return Ok((false, format!("Failed to set allowed days: {}", e)));
            }
        }
        
        // Set time limits
        let time_limit_string = time_limits.join(";");
        let limits_command = format!("timekpra --settimelimits {} '{}'", username, time_limit_string);
        
        if let Err(e) = self.execute_ssh_command(&limits_command).await {
            let sudo_command = format!("sudo {}", limits_command);
            if let Err(_) = self.execute_ssh_command(&sudo_command).await {
                return Ok((false, format!("Failed to set time limits: {}", e)));
            }
        }
        
        Ok((true, format!("Successfully configured daily time limits for {}. Days: {}, Limits: {:?}", username, allowed_days_string, time_limits)))
    }
    
    pub async fn set_allowed_hours(&self, username: &str, intervals: &HashMap<i32, crate::database::models::UserDailyTimeInterval>) -> Result<(bool, String)> {
        let mut success_count = 0;
        let mut error_messages = Vec::new();
        
        for day_num in 1..=7 {
            if let Some(interval) = intervals.get(&day_num) {
                if interval.is_enabled && interval.is_valid_interval() {
                    if let Some(hour_specs) = interval.to_timekpr_format() {
                        let hour_string = hour_specs.join(";");
                        let command = format!("timekpra --setallowedhours {} {} '{}'", username, day_num, hour_string);
                        
                        match self.execute_ssh_command(&command).await {
                            Ok((exit_code, _output, stderr)) => {
                                if exit_code != 0 {
                                    let sudo_command = format!("sudo {}", command);
                                    match self.execute_ssh_command(&sudo_command).await {
                                        Ok((exit_code, _output, stderr)) => {
                                            if exit_code != 0 {
                                                error_messages.push(format!("{}: {}", interval.get_day_name(), stderr));
                                                continue;
                                            }
                                        }
                                        Err(e) => {
                                            error_messages.push(format!("{}: Connection error: {}", interval.get_day_name(), e));
                                            continue;
                                        }
                                    }
                                }
                                success_count += 1;
                            }
                            Err(e) => {
                                error_messages.push(format!("{}: Connection error: {}", interval.get_day_name(), e));
                            }
                        }
                    }
                } else {
                    // Set full day access for disabled intervals
                    let full_day_hours: Vec<String> = (0..24).map(|h| h.to_string()).collect();
                    let hour_string = full_day_hours.join(";");
                    let command = format!("timekpra --setallowedhours {} {} '{}'", username, day_num, hour_string);
                    
                    if self.execute_ssh_command(&command).await.is_ok() {
                        success_count += 1;
                    }
                }
            }
        }
        
        if success_count > 0 {
            Ok((true, format!("Successfully configured allowed hours for {}. Days configured: {}/7", username, success_count)))
        } else {
            Ok((false, format!("Failed to configure allowed hours: {}", error_messages.join("; "))))
        }
    }
    
    async fn execute_ssh_command(&self, command: &str) -> Result<(i32, String, String)> {
        if !Path::new(&self.key_path).exists() {
            return Err(anyhow!("SSH private key not found at {}", self.key_path));
        }
        
        let output = Command::new("ssh")
            .args(&[
                "-i", &self.key_path,
                "-o", "StrictHostKeyChecking=no",
                "-o", "ConnectTimeout=10",
                &format!("{}@{}", self.username, self.hostname),
                command,
            ])
            .output()
            .map_err(|e| anyhow!("Failed to execute SSH command: {}", e))?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        
        Ok((exit_code, stdout, stderr))
    }
    
    fn parse_timekpr_output(&self, output: &str) -> HashMap<String, serde_json::Value> {
        let mut config = HashMap::new();
        
        for line in output.lines() {
            if let Some((key, value)) = line.split_once(": ") {
                let key = key.trim();
                let value = value.trim();
                
                if value.is_empty() {
                    continue;
                }
                
                let json_value = if value.chars().all(|c| c.is_ascii_digit()) {
                    serde_json::Value::Number(value.parse::<i64>().unwrap().into())
                } else if value.contains(';') {
                    let items: Vec<serde_json::Value> = value
                        .split(';')
                        .map(|item| {
                            if item.chars().all(|c| c.is_ascii_digit()) {
                                serde_json::Value::Number(item.parse::<i64>().unwrap().into())
                            } else {
                                serde_json::Value::String(item.to_string())
                            }
                        })
                        .collect();
                    serde_json::Value::Array(items)
                } else if value.eq_ignore_ascii_case("true") {
                    serde_json::Value::Bool(true)
                } else if value.eq_ignore_ascii_case("false") {
                    serde_json::Value::Bool(false)
                } else {
                    serde_json::Value::String(value.to_string())
                };
                
                config.insert(key.to_string(), json_value);
            }
        }
        
        config
    }
}