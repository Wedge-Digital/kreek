use crate::app::match_report::ports::ICoachDataPort;
use crate::app::shared_kernel::common_types::CoachId;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::ISpaceUserCacheRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CoachDataAdapter {
    user_cache_repo: Arc<dyn ISpaceUserCacheRepository>,
}

impl CoachDataAdapter {
    pub fn new(user_cache_repo: Arc<dyn ISpaceUserCacheRepository>) -> Self {
        Self { user_cache_repo }
    }
}

#[async_trait]
impl ICoachDataPort for CoachDataAdapter {
    async fn find_coach_name(&self, coach_id: &str) -> Option<String> {
        let id = CoachId::try_new(coach_id).ok()?;
        let user = self.user_cache_repo.find_user_by_id(&id).await.ok()?;
        Some(user.name.to_string())
    }
}
