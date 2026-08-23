use crate::app::competitions::ports::{ICompetitionSpaceMemberPort, SpaceMemberDto};
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::shared_kernel::identity::space_definition::SpaceDefinition;
use crate::app::shared_kernel::identity::space_name::SpaceName;
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::spaces::domain::space_repository_port::user_cache_repository_port::ISpaceUserCacheRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SpaceMemberAdapter {
    space_repo: Arc<dyn ISpaceRepository>,
    /// Le cache utilisateur de `spaces` est le **seul** dépôt qui porte les
    /// adresses e-mail. Les trois tables `competitions__*_cache` en contiennent
    /// aussi, et ne sont ni lues ni écrites nulle part : les brancher ferait un
    /// envoi silencieusement vide.
    user_cache: Arc<dyn ISpaceUserCacheRepository>,
}

impl SpaceMemberAdapter {
    pub fn new(
        space_repo: Arc<dyn ISpaceRepository>,
        user_cache: Arc<dyn ISpaceUserCacheRepository>,
    ) -> Self {
        Self {
            space_repo,
            user_cache,
        }
    }
}

#[async_trait]
impl ICompetitionSpaceMemberPort for SpaceMemberAdapter {
    /// Une lecture en échec rend une liste vide, jamais une erreur : le seul
    /// appelant est l'envoi de notifications, et un cron qui s'arrête sur une
    /// saison bloquerait toutes les suivantes. L'absence de destinataire est
    /// journalisée par le journal d'envois (R1), qui la distingue d'un envoi.
    async fn list_space_members(&self, space_id: &SpaceId) -> Vec<SpaceMemberDto> {
        self.user_cache
            .list_members_for_space(space_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|u| SpaceMemberDto {
                coach_id: u.id.to_string(),
                coach_name: u.name.to_string(),
                email: u.email.into_inner(),
            })
            .collect()
    }

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
