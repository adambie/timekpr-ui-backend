pub mod auth;
pub mod dashboard;
pub mod schedule;
pub mod system;
pub mod time;
pub mod users;

// Re-export all handler functions for easy importing
pub use auth::*;
pub use dashboard::*;
pub use schedule::*;
pub use system::*;
pub use time::*;
pub use users::*;