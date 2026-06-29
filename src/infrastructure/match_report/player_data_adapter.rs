use crate::app::match_report::ports::IPlayerDataPort;
use crate::app::players::ports::IPlayerProjectionRepository;
use async_trait::async_trait;
use std::sync::Arc;

pub struct PlayerDataAdapter {
    player_projection_repo: Arc<dyn IPlayerProjectionRepository>,
}

impl PlayerDataAdapter {
    pub fn new(player_projection_repo: Arc<dyn IPlayerProjectionRepository>) -> Self {
        Self { player_projection_repo }
    }
}

#[async_trait]
impl IPlayerDataPort for PlayerDataAdapter {
    async fn count_available_players(&self, team_id: &str) -> Result<usize, String> {
        use crate::app::players::domain::player::TeamId;
        let tid = TeamId(team_id.to_string());
        let players = self.player_projection_repo.find_by_team_id(&tid).await.map_err(|e| e.to_string())?;
        Ok(players.len())
    }

    async fn find_player_display(&self, player_id: &str) -> Option<String> {
        let player = self.player_projection_repo.find_by_id(player_id).await.ok()??;
        let label = if !player.personal_name.is_empty() {
            player.personal_name.clone()
        } else {
            player.position_name.clone()
        };
        let jersey = player.jersey.map(|j| format!(" #{j}")).unwrap_or_default();
        Some(format!("{}{}", label, jersey))
    }

    async fn find_player_position(&self, player_id: &str) -> Option<String> {
        let player = self.player_projection_repo.find_by_id(player_id).await.ok()??;
        Some(player.position_name)
    }
}
