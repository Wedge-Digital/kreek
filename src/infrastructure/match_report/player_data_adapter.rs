use crate::app::match_report::ports::{IPlayerDataPort, PositionCountDto};
use crate::app::players::ports::{IPlayerProjectionRepository, IPlayerRepository};
use async_trait::async_trait;
use std::sync::Arc;

pub struct PlayerDataAdapter {
    player_projection_repo: Arc<dyn IPlayerProjectionRepository>,
    /// L'historique des SPP dépensés ne vit que dans le flux d'événements — la
    /// projection ne le porte pas.
    player_repo: Arc<dyn IPlayerRepository>,
}

impl PlayerDataAdapter {
    pub fn new(
        player_projection_repo: Arc<dyn IPlayerProjectionRepository>,
        player_repo: Arc<dyn IPlayerRepository>,
    ) -> Self {
        Self {
            player_projection_repo,
            player_repo,
        }
    }
}

#[async_trait]
impl IPlayerDataPort for PlayerDataAdapter {
    /// Les joueurs alignables, pas l'effectif total : un blessé qui rate le
    /// prochain match laisse une place que doit combler un journalier.
    async fn count_available_players(&self, team_id: &str) -> Result<usize, String> {
        use crate::app::players::domain::player::TeamId;
        let tid = TeamId(team_id.to_string());
        self.player_projection_repo
            .count_available_by_team_id(&tid)
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_player_display(&self, player_id: &str) -> Option<String> {
        let player = self
            .player_projection_repo
            .find_by_id(player_id)
            .await
            .ok()??;
        let label = if !player.personal_name.is_empty() {
            player.personal_name.clone()
        } else {
            player.position_name.clone()
        };
        let jersey = player.jersey.map(|j| format!(" #{j}")).unwrap_or_default();
        Some(format!("{}{}", label, jersey))
    }

    async fn find_player_position(&self, player_id: &str) -> Option<String> {
        let player = self
            .player_projection_repo
            .find_by_id(player_id)
            .await
            .ok()??;
        Some(player.position_name)
    }

    async fn has_spent_spp_since_match(
        &self,
        team_id: &str,
        match_report_id: &str,
    ) -> Result<bool, String> {
        use crate::app::players::domain::player::TeamId;
        self.player_repo
            .has_spent_spp_since_match(&TeamId(team_id.to_string()), match_report_id)
            .await
            .map_err(|e| e.to_string())
    }

    async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto> {
        use crate::app::players::domain::player::TeamId;
        let tid = TeamId(team_id.to_string());
        let players = self
            .player_projection_repo
            .find_by_team_id(&tid)
            .await
            .unwrap_or_default();
        let mut counts: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
        for p in &players {
            *counts.entry(p.roster_line_id.clone()).or_insert(0) += 1;
        }
        counts
            .into_iter()
            .map(|(position_uid, count)| PositionCountDto {
                position_uid,
                count,
            })
            .collect()
    }
}
