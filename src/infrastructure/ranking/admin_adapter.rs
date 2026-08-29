//! L'adapter d'autorisation du BC `ranking` — le seul endroit qui sache que les
//! droits viennent de `competitions` et de `spaces`.
//!
//! `ranking` ne connaît que son trait : si les deux BCs étaient un jour déployés
//! à part, seul ce fichier changerait.

use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::ranking::ports::IRankingAdminPort;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RankingAdminAdapter {
    competition_repo: Arc<dyn ICompetitionRepository>,
    space_repo: Arc<dyn ISpaceRepository>,
}

impl RankingAdminAdapter {
    pub fn new(
        competition_repo: Arc<dyn ICompetitionRepository>,
        space_repo: Arc<dyn ISpaceRepository>,
    ) -> Self {
        Self {
            competition_repo,
            space_repo,
        }
    }
}

#[async_trait]
impl IRankingAdminPort for RankingAdminAdapter {
    /// **Par l'identifiant seul, jamais par le nom.**
    ///
    /// `CompetitionBaseInfo` porte les deux listes, et `require_admin_access`
    /// consulte les deux — un héritage de l'import legacy, où des
    /// administrateurs sont désignés par leur pseudonyme. Ici la source est une
    /// session authentifiée : l'identifiant est toujours présent et ne souffre
    /// pas d'homonymie, contrairement à un nom de coach.
    async fn is_competition_admin(&self, user_id: &str, competition_id: &str) -> bool {
        let Ok(id) = CompetitionId::try_new(competition_id) else {
            return false;
        };
        match self.competition_repo.find_base_info(&id).await {
            Ok(Some(info)) => info.admin_ids.iter().any(|a| a == user_id),
            _ => false,
        }
    }

    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool {
        let (Ok(coach), Ok(space)) = (CoachId::try_new(user_id), SpaceId::try_new(space_id)) else {
            return false;
        };
        matches!(
            self.space_repo.find_member_profile(&coach, &space).await,
            Ok(Some(SpaceProfile::SpaceAdmin))
        )
    }
}
