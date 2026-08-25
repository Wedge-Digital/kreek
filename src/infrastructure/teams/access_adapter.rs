//! Les deux droits d'administration que `teams` ne peut pas décider seul.
//!
//! `teams` sait qui possède une équipe — `Team` porte `coach_id`. Il ne sait
//! rien des administrateurs d'espace ni de compétition, qui vivent dans
//! `spaces` et `competitions`. Cet adapter est le seul endroit qui connaisse
//! les deux, et il vit dans l'infrastructure pour cette raison.
//!
//! Les deux corps sont repris de `infrastructure/players/` — `space_member_adapter`
//! et `competition_admin_adapter` — dont ils interrogent les mêmes dépôts. La
//! règle d'autorisation est ainsi écrite deux fois dans le projet, ce que la
//! carte 389 assume comme le prix de la souveraineté entre BCs.

use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::spaces::domain::space_repository_port::space_repository_port::ISpaceRepository;
use crate::app::teams::ports::ITeamAccessPort;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TeamAccessAdapter {
    space_repo: Arc<dyn ISpaceRepository>,
    competition_repo: Arc<dyn ICompetitionRepository>,
}

impl TeamAccessAdapter {
    pub fn new(
        space_repo: Arc<dyn ISpaceRepository>,
        competition_repo: Arc<dyn ICompetitionRepository>,
    ) -> Self {
        Self {
            space_repo,
            competition_repo,
        }
    }
}

#[async_trait]
impl ITeamAccessPort for TeamAccessAdapter {
    async fn is_space_admin(&self, coach_id: &CoachId, space_id: &SpaceId) -> bool {
        matches!(
            self.space_repo
                .find_member_profile(coach_id, space_id)
                .await
                .ok()
                .flatten(),
            Some(SpaceProfile::SpaceAdmin)
        )
    }

    /// Un dépôt en échec rend `false` : le bouton disparaît plutôt que
    /// d'apparaître à tort. L'écriture reste gardée par `can_spend_spp` de
    /// toute façon — le pire cas est un administrateur privé de son
    /// raccourci, jamais un visiteur qui gagne un droit.
    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
        coach_name: &str,
    ) -> bool {
        let Ok(id) = CompetitionId::try_new(competition_id) else {
            return false;
        };
        match self.competition_repo.find_base_info(&id).await {
            Ok(Some(info)) => {
                info.admin_ids.iter().any(|x| x == coach_id)
                    || info.admin_names.iter().any(|x| x == coach_name)
            }
            _ => false,
        }
    }
}
