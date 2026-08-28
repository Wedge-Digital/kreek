//! `competitions` demande à `ranking` de rejouer un classement.
//!
//! Seul cet adaptateur connaît les deux BCs : `competitions` ne voit qu'un
//! trait, `ranking` ne sait pas qui l'appelle. Si les deux étaient déployés
//! séparément un jour, c'est ce fichier qu'on remplacerait par un appel réseau.

use crate::app::competitions::ports::{IRankingRecomputePort, RecomputeReportDto};
use crate::app::ranking::ports::{IRankingCompetitionPort, IRankingRepository};
use crate::app::ranking::use_cases::recompute_season_ranking_use_case;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct RankingRecomputeAdapter {
    repository: Arc<dyn IRankingRepository>,
    competition_port: Arc<dyn IRankingCompetitionPort>,
}

impl RankingRecomputeAdapter {
    pub fn new(
        repository: Arc<dyn IRankingRepository>,
        competition_port: Arc<dyn IRankingCompetitionPort>,
    ) -> Self {
        Self {
            repository,
            competition_port,
        }
    }
}

#[async_trait]
impl IRankingRecomputePort for RankingRecomputeAdapter {
    async fn recompute_season(&self, season_id: &str) -> Result<RecomputeReportDto, String> {
        // Un identifiant mal formé n'atteint pas `ranking` : il vient du chemin,
        // déjà contrôlé, et le laisser passer produirait une saison vide plutôt
        // qu'une erreur.
        let season = SeasonId::try_new(season_id)
            .map_err(|e| format!("identifiant de saison « {season_id} » : {e}"))?;

        let rapport = recompute_season_ranking_use_case::execute(
            &season,
            self.repository.as_ref(),
            self.competition_port.as_ref(),
        )
        .await
        .map_err(|e| format!("{e:?}"))?;

        Ok(RecomputeReportDto {
            matches_replayed: rapport.matches_replayed,
            teams: rapport.teams,
        })
    }
}
