use crate::app::competitions::ports::ICompetitionSpaceMemberPort;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SpaceMemberAdapter {
    space_repo: Arc<dyn ISpaceRepository>,
}

impl SpaceMemberAdapter {
    pub fn new(space_repo: Arc<dyn ISpaceRepository>) -> Self {
        Self { space_repo }
    }
}

#[async_trait]
impl ICompetitionSpaceMemberPort for SpaceMemberAdapter {
    async fn find_member_profile(&self, coach_id: &CoachId, space_id: &SpaceId) -> Option<SpaceProfile> {
        self.space_repo.find_member_profile(coach_id, space_id).await.ok().flatten()
    }
}
