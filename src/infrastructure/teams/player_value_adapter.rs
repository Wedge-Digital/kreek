use crate::app::teams::ports::{IPlayerValuePort, PlayerValueDto};
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PlayerValueAdapter {
    pool: PgPool,
}

impl PlayerValueAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Seul `Available` rend un joueur disponible ; `MissingNextGame`, `Retired` et
/// `Dead` ne le sont pas. C'est ici, et nulle part ailleurs, que le vocabulaire
/// de `players` est traduit — `teams` ne connaît que le booléen.
fn is_available(participation_status: &str) -> bool {
    participation_status == "Available"
}

#[async_trait]
impl IPlayerValuePort for PlayerValueAdapter {
    async fn find_valued_players(&self, team_id: &str) -> Vec<PlayerValueDto> {
        let rows: Vec<(String, i32, String)> = sqlx::query_as(
            "SELECT player_id, value_kpo, participation_status \
             FROM players_proj WHERE team_id = $1",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows.into_iter()
            .map(|(player_id, value_kpo, status)| PlayerValueDto {
                player_id,
                value_kpo: value_kpo.max(0) as u32,
                available_for_next_match: is_available(&status),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seul_available_rend_un_joueur_disponible() {
        assert!(is_available("Available"));
        for indisponible in ["MissingNextGame", "Retired", "Dead"] {
            assert!(!is_available(indisponible), "{indisponible}");
        }
    }
}
