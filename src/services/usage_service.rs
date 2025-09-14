use crate::models::ServiceError;
use crate::repositories::UsageRepository;
use chrono::Utc;
use std::sync::Arc;

pub struct UsageService {
    repository: Arc<dyn UsageRepository>,
}

impl UsageService {
    pub fn new(repository: Arc<dyn UsageRepository>) -> Self {
        Self { repository }
    }

    pub async fn store_daily_usage(&self, user_id: i64, time_spent: i64) -> Result<(), ServiceError> {
        let today = Utc::now().date_naive();
        self.repository.store_daily_usage(user_id, today, time_spent).await
    }
}