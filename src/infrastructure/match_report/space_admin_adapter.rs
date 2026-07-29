use crate::app::match_report::ports::ISpaceAdminPort;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use async_trait::async_trait;
use std::sync::Arc;

/// Implémente `ISpaceAdminPort` en interrogeant le BC `spaces`. Seul ce fichier
/// d'infrastructure connaît le BC source — le BC `match_report` ne voit que le
/// port.
pub struct SpaceAdminAdapter {
    space_repository: Arc<dyn ISpaceRepository>,
}

impl SpaceAdminAdapter {
    pub fn new(space_repository: Arc<dyn ISpaceRepository>) -> Self {
        Self { space_repository }
    }
}

#[async_trait]
impl ISpaceAdminPort for SpaceAdminAdapter {
    /// Un identifiant mal formé ou une erreur de lecture valent « pas admin » :
    /// un contrôle d'accès échoue toujours fermé.
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool {
        let (Ok(coach_id), Ok(space_id)) = (CoachId::try_new(user_id), SpaceId::try_new(space_id))
        else {
            return false;
        };
        matches!(
            self.space_repository
                .find_member_profile(&coach_id, &space_id)
                .await,
            Ok(Some(SpaceProfile::SpaceAdmin))
        )
    }
}
