//! `competitions` répond sur ses compétitions et ses saisons (carte 318).
//!
//! Deux résolveurs et non un seul : la liste du middleware est **par
//! ressource**, pas par BC. Un chemin qui porte les deux identifiants est donc
//! contrôlé deux fois — ce qui est exactement voulu, une saison d'une autre
//! compétition étant un cas aussi illicite qu'une compétition d'un autre
//! espace.
//!
//! `ranking` n'apporte rien : ses deux routes portent `{competition_id}` et
//! `{season_id}`, donc les résolveurs d'ici les couvrent. C'est ce qui a fait
//! voyager les deux BCs sur la même carte.

use crate::app::competitions::domain::competition_repository_port::ICompetitionRepository;
use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::web::middleware::space_scope::ISpaceOwnership;
use async_trait::async_trait;
use std::sync::Arc;

pub struct CompetitionSpaceOwnership {
    repo: Arc<dyn ICompetitionRepository>,
}

impl CompetitionSpaceOwnership {
    pub fn new(repo: Arc<dyn ICompetitionRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ISpaceOwnership for CompetitionSpaceOwnership {
    fn param(&self) -> &'static str {
        "competition_id"
    }

    /// Un identifiant mal formé rend `None`, donc `404` : il ne désigne aucune
    /// compétition, et le distinguer d'une compétition étrangère renseignerait
    /// qui sonde.
    async fn space_of(&self, id: &str) -> Option<SpaceId> {
        let competition_id = CompetitionId::try_new(id).ok()?;
        match self.repo.find_space_id(&competition_id).await {
            Ok(Some(brut)) => SpaceId::try_new(&brut).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("space_ownership competitions {id} : {e:?}");
                None
            }
        }
    }
}

pub struct SeasonSpaceOwnership {
    repo: Arc<dyn ISeasonRepository>,
}

impl SeasonSpaceOwnership {
    pub fn new(repo: Arc<dyn ISeasonRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ISpaceOwnership for SeasonSpaceOwnership {
    fn param(&self) -> &'static str {
        "season_id"
    }

    /// La saison n'a pas d'espace en propre : le repository fait le saut par
    /// `competition_id`, en une jointure.
    async fn space_of(&self, id: &str) -> Option<SpaceId> {
        let season_id = SeasonId::try_new(id).ok()?;
        match self.repo.find_space_id(&season_id).await {
            Ok(Some(brut)) => SpaceId::try_new(&brut).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("space_ownership seasons {id} : {e:?}");
                None
            }
        }
    }
}
