use crate::app::competitions::ports::ICompetitionSpaceMemberPort;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
use crate::app::shared_kernel::identity::space_name::SpaceName;
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
    async fn find_member_profile(
        &self,
        coach_id: &CoachId,
        space_id: &SpaceId,
    ) -> Option<SpaceProfile> {
        self.space_repo
            .find_member_profile(coach_id, space_id)
            .await
            .ok()
            .flatten()
    }

    /// Les espaces dont l'identifiant ou le nom sont refusés par leur value
    /// object sont **écartés**, pas fatals : c'est une page de test, et faire
    /// tomber tout le sélecteur sur une ligne douteuse en base rendrait les
    /// autres widgets intestables. Le code d'origine y faisait un `expect("")`
    /// — un panic sans message, sur des données non maîtrisées.
    async fn find_all_spaces(&self) -> Vec<SpaceDefinition> {
        self.space_repo
            .find_all()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| {
                Some(SpaceDefinition {
                    id: SpaceId::try_new(&s.id).ok()?,
                    name: SpaceName::try_new(&s.name).ok()?,
                })
            })
            .collect()
    }
}
