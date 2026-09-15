//! Le layout demande à `competitions` si un espace interdit le hors-calendrier.
//!
//! **Seul fichier de la chaîne à importer le BC source.** `src/web/` ne connaît
//! que son propre trait ; c'est ici que le nom `competitions` apparaît, et nulle
//! part ailleurs côté layout.
//!
//! # Une requête, pas une boucle
//!
//! La question porte sur toutes les compétitions d'un espace. La poser
//! compétition par compétition — lister, puis pour chacune sa dernière saison,
//! puis ses options — ferait 2N+1 allers-retours à **chaque rendu de menu**,
//! c'est-à-dire à chaque navigation HTMX. Le dépôt y répond en une requête.

use crate::app::competitions::domain::season_repository_port::ISeasonRepository;
use crate::web::ports::IHorsCalendrierPort;
use async_trait::async_trait;
use std::sync::Arc;

pub struct HorsCalendrierAdapter {
    season_repo: Arc<dyn ISeasonRepository>,
}

impl HorsCalendrierAdapter {
    pub fn new(season_repo: Arc<dyn ISeasonRepository>) -> Self {
        Self { season_repo }
    }
}

#[async_trait]
impl IHorsCalendrierPort for HorsCalendrierAdapter {
    async fn un_espace_interdit(&self, space_id: &str) -> bool {
        match self
            .season_repo
            .espace_interdit_hors_calendrier(space_id)
            .await
        {
            Ok(interdit) => interdit,
            // `false` sur erreur : un menu amputé par une panne de base est plus
            // déroutant qu'une entrée qui mènera à un refus. La garde serveur,
            // elle, ne se relâche pas — c'est elle qui protège, pas le menu.
            Err(e) => {
                tracing::error!("hors_calendrier_adapter: espace {space_id}: {e}");
                false
            }
        }
    }
}
