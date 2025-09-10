use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct ScheduleUpdateForm {
    pub user_id: i64,
    pub monday: f64,
    pub tuesday: f64,
    pub wednesday: f64,
    pub thursday: f64,
    pub friday: f64,
    pub saturday: f64,
    pub sunday: f64,
    
    // Time intervals for each day (format: "HH:MM")
    pub monday_start_time: Option<String>,
    pub monday_end_time: Option<String>,
    
    pub tuesday_start_time: Option<String>,
    pub tuesday_end_time: Option<String>,
    
    pub wednesday_start_time: Option<String>,
    pub wednesday_end_time: Option<String>,
    
    pub thursday_start_time: Option<String>,
    pub thursday_end_time: Option<String>,
    
    pub friday_start_time: Option<String>,
    pub friday_end_time: Option<String>,
    
    pub saturday_start_time: Option<String>,
    pub saturday_end_time: Option<String>,
    
    pub sunday_start_time: Option<String>,
    pub sunday_end_time: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, ToSchema)]
pub struct AddUserForm {
    pub username: String,
    pub system_ip: String,
}

#[derive(Deserialize, ToSchema)]
pub struct ModifyTimeForm {
    pub user_id: i64,
    pub operation: String,
    pub seconds: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct PasswordChangeForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}