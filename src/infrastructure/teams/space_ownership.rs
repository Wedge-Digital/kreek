//! `teams` répond sur ses équipes (carte 320).
//!
//! `team_proj` porte `space_id` : comparaison directe, pas de saut.
//!
//! Ce résolveur couvre aussi les quatre routes de `match_report` qui portent
//! `{team_id}` — la liste du middleware étant plate, un BC bénéficie des
//! résolveurs des autres sans les connaître.

use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::app::teams::ports::ITeamRepository;
use crate::web::middleware::space_scope::ISpaceOwnership;
use async_trait::async_trait;
use std::sync::Arc;

pub struct TeamSpaceOwnership {
    repo: Arc<dyn ITeamRepository>,
}

impl TeamSpaceOwnership {
    pub fn new(repo: Arc<dyn ITeamRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ISpaceOwnership for TeamSpaceOwnership {
    fn param(&self) -> &'static str {
        "team_id"
    }

    async fn space_of(&self, id: &str) -> Option<SpaceId> {
        match self.repo.find_space_id(id).await {
            Ok(Some(brut)) => SpaceId::try_new(&brut).ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("space_ownership teams {id} : {e:?}");
                None
            }
        }
    }
}
