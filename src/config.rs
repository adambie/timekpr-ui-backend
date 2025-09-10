use utoipa::OpenApi;
use crate::models::*;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "TimeKpr UI API",
        version = "0.1.0",
        description = "REST API for TimeKpr UI - Remote management for Timekpr-nExT parental control software. Authentication uses JWT tokens via Bearer authorization header."
    ),
    servers(
        (url = "http://localhost:5000", description = "Local development server")
    ),
    paths(
        crate::handlers::auth::login_api,
        crate::handlers::auth::logout_api,
        crate::handlers::auth::change_password_api,
        crate::handlers::dashboard::dashboard_api,
        crate::handlers::dashboard::admin_api,
        crate::handlers::users::add_user_api,
        crate::handlers::users::validate_user,
        crate::handlers::users::delete_user,
        crate::handlers::time::modify_time,
        crate::handlers::time::get_user_usage,
        crate::handlers::schedule::update_schedule_api,
        crate::handlers::schedule::get_schedule_sync_status,
        crate::handlers::system::get_task_status,
        crate::handlers::system::get_ssh_status
    ),
    components(
        schemas(
            LoginForm,
            AddUserForm,
            ModifyTimeForm,
            PasswordChangeForm,
            ScheduleUpdateForm,
            ManagedUser,
            ApiResponse,
            LoginResponse,
            UserData,
            DashboardResponse,
            AdminUserData,
            AdminResponse,
            ModifyTimeResponse,
            UsageData,
            UsageResponse,
            TaskStatusData,
            TaskStatusResponse,
            ScheduleWithIntervals,
            WeeklyHours,
            WeeklyTimeIntervals,
            TimeInterval,
            ScheduleSyncResponse,
            SshStatusResponse,
            ErrorResponse
        )
    )
)]
pub struct ApiDoc;