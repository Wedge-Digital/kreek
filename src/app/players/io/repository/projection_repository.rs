use crate::app::players::domain::player::TeamId;
use crate::app::players::ports::{
    AcquiredSkillProjection, IPlayerProjectionRepository, PlayerProjection, RepositoryError,
};
use async_trait::async_trait;
use sqlx::{PgPool, Row};

pub struct PgPlayerProjectionRepository {
    pool: PgPool,
}

impl PgPlayerProjectionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IPlayerProjectionRepository for PgPlayerProjectionRepository {
    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<PlayerProjection>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT player_id, team_id, space_id, position_name, roster_line_id,
                    personal_name, jersey, base_skills, acquired_skills, spp, value_kpo
             FROM players_projection
             WHERE team_id = $1
             ORDER BY jersey NULLS LAST, player_id",
        )
        .bind(&team_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        rows.iter()
            .map(|r| {
                let base_skills: Vec<String> =
                    serde_json::from_value(r.get::<serde_json::Value, _>("base_skills"))
                        .map_err(RepositoryError::Deserialization)?;
                let acquired_skills: Vec<AcquiredSkillProjection> =
                    serde_json::from_value(r.get::<serde_json::Value, _>("acquired_skills"))
                        .map_err(RepositoryError::Deserialization)?;

                Ok(PlayerProjection {
                    player_id:      r.get("player_id"),
                    team_id:        r.get("team_id"),
                    space_id:       r.get("space_id"),
                    position_name:  r.get("position_name"),
                    roster_line_id: r.get("roster_line_id"),
                    personal_name:  r.get("personal_name"),
                    jersey:         r.get("jersey"),
                    base_skills,
                    acquired_skills,
                    spp:            r.get("spp"),
                    value_kpo:      r.get("value_kpo"),
                })
            })
            .collect()
    }
}
