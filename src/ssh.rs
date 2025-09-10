use std::process::Command;
use std::path::Path;
use serde_json::Value;

pub struct SSHClient {
    hostname: String,
}

impl SSHClient {
    pub fn new(hostname: &str) -> Self {
        Self {
            hostname: hostname.to_string(),
        }
    }
    
    pub fn check_ssh_key_exists() -> bool {
        Self::find_ssh_key_path().is_some()
    }
    
    pub fn find_ssh_key_path() -> Option<String> {
        let basic_paths = [
            "ssh/timekpr_ui_key",
            "./ssh/timekpr_ui_key",
            "/app/ssh/timekpr_ui_key",
        ];
        
        // Check basic paths first
        for path in &basic_paths {
            if Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
        
        // Check home directory keys
        if let Some(home) = dirs::home_dir() {
            let id_rsa = home.join(".ssh/id_rsa");
            let id_ed25519 = home.join(".ssh/id_ed25519");
            
            if id_rsa.exists() {
                return Some(id_rsa.to_string_lossy().to_string());
            }
            if id_ed25519.exists() {
                return Some(id_ed25519.to_string_lossy().to_string());
            }
        }
        
        None
    }

    pub async fn validate_user(&self, username: &str) -> (bool, String, Option<Value>) {
        // Find SSH key path
        let key_path = match Self::find_ssh_key_path() {
            Some(path) => {
                println!("Using SSH key: {}", path);
                path
            },
            None => {
                return (false, "SSH key not found. Please configure SSH keys for passwordless authentication.".to_string(), None);
            }
        };
        
        // For now, use system SSH command instead of russh library for simplicity
        let target_host = format!("timekpr-remote@{}", self.hostname);
        let command = format!("timekpra --userinfo {}", username);
        
        println!("Running SSH command: ssh -i {} -o ConnectTimeout=5 -o StrictHostKeyChecking=no -o BatchMode=yes -o PasswordAuthentication=no {} {}", 
                 key_path, target_host, command);
        
        let output = Command::new("ssh")
            .args(&[
                "-i", &key_path,
                "-o", "ConnectTimeout=5",
                "-o", "StrictHostKeyChecking=no",
                "-o", "BatchMode=yes",
                "-o", "PasswordAuthentication=no",
                &target_host,
                &command
            ])
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    
                    // Parse actual timekpr output into structured data
                    let mut config = serde_json::json!({
                        "USERNAME": username,
                        "raw_output": stdout.trim()
                    });
                    
                    // Parse timekpr output for time values (use ACTUAL_ values for current state)
                    let output_lines: Vec<&str> = stdout.lines().collect();
                    for line in output_lines {
                        if line.contains("ACTUAL_TIME_LEFT_DAY") {
                            if let Some(value_str) = line.split(':').nth(1) {
                                if let Ok(seconds) = value_str.trim().parse::<i64>() {
                                    config["TIME_LEFT_DAY"] = serde_json::Value::Number(seconds.into());
                                }
                            }
                        } else if line.contains("ACTUAL_TIME_SPENT_DAY") {
                            if let Some(value_str) = line.split(':').nth(1) {
                                if let Ok(seconds) = value_str.trim().parse::<i64>() {
                                    config["TIME_SPENT_DAY"] = serde_json::Value::Number(seconds.into());
                                }
                            }
                        }
                        // Add more parsing for other timekpr fields as needed
                    }
                    
                    // If no time data was parsed, set defaults for testing
                    if !config.as_object().unwrap().contains_key("TIME_LEFT_DAY") {
                        config["TIME_LEFT_DAY"] = serde_json::Value::Number(7200.into()); // 2 hours default
                    }
                    if !config.as_object().unwrap().contains_key("TIME_SPENT_DAY") {
                        config["TIME_SPENT_DAY"] = serde_json::Value::Number(1800.into()); // 30 minutes default
                    }

                    (true, format!("User {} validated successfully", username), Some(config))
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    println!("SSH validation failed - stderr: {}", stderr);
                    let error_msg = if stderr.contains("Permission denied") || 
                                      stderr.contains("publickey") {
                        "SSH key authentication failed. Please ensure SSH keys are properly configured.".to_string()
                    } else {
                        format!("Validation failed: {}", stderr.trim())
                    };
                    (false, error_msg, None)
                }
            }
            Err(e) => {
                // Check if it's an SSH key issue
                let error_msg = if e.to_string().contains("Permission denied") || 
                                  e.to_string().contains("publickey") ||
                                  e.to_string().contains("No such file") {
                    "SSH key authentication failed. Please ensure SSH keys are properly configured.".to_string()
                } else {
                    format!("SSH connection failed: {}", e)
                };
                (false, error_msg, None)
            }
        }
    }

    pub async fn modify_time_left(&self, username: &str, operation: &str, seconds: i64) -> (bool, String) {
        // Find SSH key path
        let key_path = match Self::find_ssh_key_path() {
            Some(path) => path,
            None => {
                return (false, "SSH key not found. Please configure SSH keys for passwordless authentication.".to_string());
            }
        };
        
        let target_host = format!("timekpr-remote@{}", self.hostname);
        let command = format!("timekpra --settimeleft {} {} {}", username, operation, seconds);
        
        println!("Running SSH command: ssh -i {} -o ConnectTimeout=5 -o StrictHostKeyChecking=no -o BatchMode=yes -o PasswordAuthentication=no {} {}", 
                 key_path, target_host, command);
        
        let output = Command::new("ssh")
            .args(&[
                "-i", &key_path,
                "-o", "ConnectTimeout=5",
                "-o", "StrictHostKeyChecking=no",
                "-o", "BatchMode=yes",
                "-o", "PasswordAuthentication=no",
                &target_host,
                &command
            ])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                
                println!("SSH command status: {}", result.status.success());
                println!("SSH stdout: {}", stdout.trim());
                if !stderr.is_empty() {
                    println!("SSH stderr: {}", stderr.trim());
                }
                
                if result.status.success() {
                    (true, format!("Time adjustment applied: {}{}s for {}", operation, seconds, username))
                } else {
                    (false, format!("Command failed: {}", stderr.trim()))
                }
            }
            Err(e) => {
                let error_msg = if e.to_string().contains("Permission denied") || 
                                  e.to_string().contains("publickey") {
                    "SSH key authentication failed. Please ensure SSH keys are properly configured.".to_string()
                } else {
                    format!("SSH connection failed: {}", e)
                };
                (false, error_msg)
            }
        }
    }

    pub async fn set_weekly_allowed_hours(&self, username: &str, intervals: &std::collections::HashMap<String, (String, String)>) -> (bool, String) {
        // Find SSH key path
        let key_path = match Self::find_ssh_key_path() {
            Some(path) => path,
            None => {
                return (false, "SSH key not found. Please configure SSH keys for passwordless authentication.".to_string());
            }
        };
        
        let target_host = format!("timekpr-remote@{}", self.hostname);
        
        // Days: 1=Monday, 2=Tuesday, ..., 7=Sunday
        let days = [
            ("monday", 1), ("tuesday", 2), ("wednesday", 3), ("thursday", 4),
            ("friday", 5), ("saturday", 6), ("sunday", 7)
        ];
        
        let mut success_count = 0;
        let mut errors = Vec::new();
        
        for (day_name, day_num) in &days {
            if let Some((start_time, end_time)) = intervals.get(*day_name) {
                // Parse time format "HH:MM" to hours
                if let (Ok(start_hour), Ok(end_hour)) = (Self::parse_time_to_hour(start_time), Self::parse_time_to_hour(end_time)) {
                    // Create hour range (start inclusive, end exclusive)
                    // For example: 7:00-17:00 means hours 7,8,9,10,11,12,13,14,15,16 (not including 17)
                    let mut hours = Vec::new();
                    let mut current = start_hour;
                    while current < end_hour {
                        hours.push(current.to_string());
                        current += 1;
                        if current > 23 { break; }
                    }
                    
                    if !hours.is_empty() {
                        let hours_string = hours.join(";");
                        let command = format!("timekpra --setallowedhours {} {} '{}'", username, day_num, hours_string);
                        
                        println!("Running SSH allowed hours command: ssh -i {} -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o BatchMode=yes -o PasswordAuthentication=no {} \"{}\"",
                                 key_path, target_host, command);
                        
                        let output = Command::new("ssh")
                            .args(&[
                                "-i", &key_path,
                                "-o", "ConnectTimeout=10",
                                "-o", "StrictHostKeyChecking=no",
                                "-o", "BatchMode=yes", 
                                "-o", "PasswordAuthentication=no",
                                &target_host,
                                &command
                            ])
                            .output();
                        
                        match output {
                            Ok(result) => {
                                let stdout = String::from_utf8_lossy(&result.stdout);
                                let stderr = String::from_utf8_lossy(&result.stderr);
                                
                                println!("SSH allowed hours command status for {}: {}", day_name, result.status.success());
                                println!("SSH stdout: {}", stdout.trim());
                                if !stderr.is_empty() {
                                    println!("SSH stderr: {}", stderr.trim());
                                }
                                
                                if result.status.success() {
                                    success_count += 1;
                                    println!("Successfully set allowed hours for {}: {}-{}", day_name, start_time, end_time);
                                } else {
                                    errors.push(format!("{}: {}", day_name, stderr.trim()));
                                }
                            }
                            Err(e) => {
                                errors.push(format!("{}: SSH connection failed: {}", day_name, e));
                            }
                        }
                    }
                } else {
                    errors.push(format!("{}: Invalid time format", day_name));
                }
            } else {
                // Set full day access (0-23 hours) when no interval specified
                let full_day_hours: Vec<String> = (0..24).map(|h| h.to_string()).collect();
                let hours_string = full_day_hours.join(";");
                let command = format!("timekpra --setallowedhours {} {} '{}'", username, day_num, hours_string);
                
                let output = Command::new("ssh")
                    .args(&[
                        "-i", &key_path,
                        "-o", "ConnectTimeout=10",
                        "-o", "StrictHostKeyChecking=no",
                        "-o", "BatchMode=yes",
                        "-o", "PasswordAuthentication=no",
                        &target_host,
                        &command
                    ])
                    .output();
                
                match output {
                    Ok(result) => {
                        if result.status.success() {
                            success_count += 1;
                            println!("Set full day access for {}", day_name);
                        } else {
                            let stderr = String::from_utf8_lossy(&result.stderr);
                            errors.push(format!("{}: {}", day_name, stderr.trim()));
                        }
                    }
                    Err(e) => {
                        errors.push(format!("{}: SSH connection failed: {}", day_name, e));
                    }
                }
            }
            
            // Small delay between days to avoid overwhelming SSH connections
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        if success_count > 0 {
            let message = if errors.is_empty() {
                format!("Successfully set allowed hours for {} for all 7 days", username)
            } else {
                format!("Partially successful: {} days configured, {} errors: {}", success_count, errors.len(), errors.join(", "))
            };
            (true, message)
        } else {
            (false, format!("Failed to set allowed hours: {}", errors.join(", ")))
        }
    }
    
    fn parse_time_to_hour(time_str: &str) -> Result<u8, ()> {
        // Parse "HH:MM" format to just the hour
        if let Some(hour_str) = time_str.split(':').next() {
            if let Ok(hour) = hour_str.parse::<u8>() {
                if hour <= 23 {
                    return Ok(hour);
                }
            }
        }
        Err(())
    }

    pub async fn set_weekly_time_limits(&self, username: &str, schedule: &std::collections::HashMap<String, f64>) -> (bool, String) {
        // Find SSH key path
        let key_path = match Self::find_ssh_key_path() {
            Some(path) => path,
            None => {
                return (false, "SSH key not found. Please configure SSH keys for passwordless authentication.".to_string());
            }
        };
        
        let target_host = format!("timekpr-remote@{}", self.hostname);
        
        let days = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
        
        // Step 1: Set allowed days (days with time limits > 0)
        let mut allowed_days = Vec::new();
        let mut time_limits = Vec::new();
        
        for (i, day) in days.iter().enumerate() {
            if let Some(hours) = schedule.get(*day) {
                if *hours > 0.0 {
                    allowed_days.push((i + 1).to_string()); // 1=Monday, 7=Sunday
                    let seconds = (*hours * 3600.0) as i64;
                    time_limits.push(seconds.to_string());
                }
            }
        }
        
        if allowed_days.is_empty() {
            return (false, "No days with time limits > 0 configured".to_string());
        }
        
        // First set allowed days
        let allowed_days_str = allowed_days.join(";");
        let days_command = format!("timekpra --setalloweddays {} '{}'", username, allowed_days_str);
        
        println!("Running SSH setalloweddays command: ssh -i {} -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o BatchMode=yes -o PasswordAuthentication=no {} \"{}\"",
                 key_path, target_host, days_command);
        
        let days_output = Command::new("ssh")
            .args(&[
                "-i", &key_path,
                "-o", "ConnectTimeout=10",
                "-o", "StrictHostKeyChecking=no",
                "-o", "BatchMode=yes",
                "-o", "PasswordAuthentication=no",
                &target_host,
                &days_command
            ])
            .output();
        
        match days_output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                
                println!("SSH setalloweddays command status: {}", result.status.success());
                println!("SSH stdout: {}", stdout.trim());
                if !stderr.is_empty() {
                    println!("SSH stderr: {}", stderr.trim());
                }
                
                if !result.status.success() {
                    return (false, format!("Failed to set allowed days: {}", stderr.trim()));
                }
            }
            Err(e) => {
                return (false, format!("SSH connection failed for setalloweddays: {}", e));
            }
        }
        
        // Step 2: Set time limits for the allowed days
        let time_limits_str = time_limits.join(";");
        let full_command = format!("timekpra --settimelimits {} '{}'", username, time_limits_str);
        
        println!("Running SSH schedule command: ssh -i {} -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o BatchMode=yes -o PasswordAuthentication=no {} \"{}\"", 
                 key_path, target_host, full_command);
        
        let output = Command::new("ssh")
            .args(&[
                "-i", &key_path,
                "-o", "ConnectTimeout=10",
                "-o", "StrictHostKeyChecking=no", 
                "-o", "BatchMode=yes",
                "-o", "PasswordAuthentication=no",
                &target_host,
                &full_command
            ])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                
                println!("SSH schedule command status: {}", result.status.success());
                println!("SSH stdout: {}", stdout.trim());
                if !stderr.is_empty() {
                    println!("SSH stderr: {}", stderr.trim());
                }
                
                if result.status.success() {
                    (true, format!("Weekly time limits applied for {}: Days: {}, Limits: {}", username, allowed_days_str, time_limits_str))
                } else {
                    (false, format!("Time limits command failed: {}", stderr.trim()))
                }
            }
            Err(e) => {
                let error_msg = if e.to_string().contains("Permission denied") || 
                                  e.to_string().contains("publickey") {
                    "SSH key authentication failed. Please ensure SSH keys are properly configured.".to_string()
                } else {
                    format!("SSH connection failed: {}", e)
                };
                (false, error_msg)
            }
        }
    }

}