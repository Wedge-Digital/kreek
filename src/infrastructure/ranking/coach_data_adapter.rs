//! Le nom d'un commissaire, pour la liste des points manuels (carte 548).
//!
//! **Copié de `infrastructure/match_report/coach_data_adapter.rs`**, seul
//! l'import du port changeant : la règle 5 du CLAUDE.md interdit de réécrire un
//! code qu'on déplace ou qu'on reprend. Les deux adapters lisent le même dépôt et
//! feront le même travail ; ce sont leurs BCs qui restent séparés.

use crate::app::ranking::ports::ICoachDataPort;
use crate::app::shared_kernel::identity::ids::CoachId;
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
