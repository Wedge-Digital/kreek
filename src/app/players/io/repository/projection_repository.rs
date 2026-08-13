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
                    personal_name, jersey, base_skills, acquired_skills, spp, value_kpo,
                    participation_status, ma_delta, st_delta, ag_delta, pa_delta, av_delta
             FROM players_proj
             WHERE team_id = $1 AND membership = 'Active'
             -- L'ordre choisi par le coach prime ; un joueur jamais réordonné
             -- (`display_order` nul) retombe derrière, trié par maillot.
             ORDER BY display_order NULLS LAST, jersey NULLS LAST, player_id",
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
                    player_id: r.get("player_id"),
                    team_id: r.get("team_id"),
                    space_id: r.get("space_id"),
                    position_name: r.get("position_name"),
                    roster_line_id: r.get("roster_line_id"),
                    personal_name: r.get("personal_name"),
                    ma_delta: r.get("ma_delta"),
                    st_delta: r.get("st_delta"),
                    ag_delta: r.get("ag_delta"),
                    pa_delta: r.get("pa_delta"),
                    av_delta: r.get("av_delta"),
                    jersey: r.get("jersey"),
                    base_skills,
                    acquired_skills,
                    spp: r.get("spp"),
                    value_kpo: r.get("value_kpo"),
                    participation_status: r.get("participation_status"),
                })
            })
            .collect()
    }

    async fn find_space_id(&self, player_id: &str) -> Result<Option<String>, RepositoryError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT space_id FROM players_proj WHERE player_id = $1")
                .bind(player_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(RepositoryError::Database)?;
        Ok(row.map(|r| r.0))
    }

    async fn find_by_id(
        &self,
        player_id: &str,
    ) -> Result<Option<PlayerProjection>, RepositoryError> {
        let row = sqlx::query(
            "SELECT player_id, team_id, space_id, position_name, roster_line_id,
                    personal_name, jersey, base_skills, acquired_skills, spp, value_kpo,
                    participation_status, ma_delta, st_delta, ag_delta, pa_delta, av_delta
             FROM players_proj WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        row.map(|r| {
            let base_skills: Vec<String> =
                serde_json::from_value(r.get::<serde_json::Value, _>("base_skills"))
                    .map_err(RepositoryError::Deserialization)?;
            let acquired_skills: Vec<AcquiredSkillProjection> =
                serde_json::from_value(r.get::<serde_json::Value, _>("acquired_skills"))
                    .map_err(RepositoryError::Deserialization)?;
            Ok(PlayerProjection {
                player_id: r.get("player_id"),
                team_id: r.get("team_id"),
                space_id: r.get("space_id"),
                position_name: r.get("position_name"),
                roster_line_id: r.get("roster_line_id"),
                personal_name: r.get("personal_name"),
                ma_delta: r.get("ma_delta"),
                st_delta: r.get("st_delta"),
                ag_delta: r.get("ag_delta"),
                pa_delta: r.get("pa_delta"),
                av_delta: r.get("av_delta"),
                jersey: r.get("jersey"),
                base_skills,
                acquired_skills,
                spp: r.get("spp"),
                value_kpo: r.get("value_kpo"),
                participation_status: r.get("participation_status"),
            })
        })
        .transpose()
    }

    /// Compté en base plutôt qu'en mémoire : l'appelant ne veut qu'un nombre,
    /// pas de quoi charger tout l'effectif pour le filtrer ensuite.
    async fn jerseys_by_team_id(&self, team_id: &TeamId) -> Result<Vec<u16>, RepositoryError> {
        let lignes: Vec<(Option<i16>,)> = sqlx::query_as(
            "SELECT jersey FROM players_proj WHERE team_id = $1 AND membership = 'Active'",
        )
        .bind(&team_id.0)
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(lignes
            .into_iter()
            .filter_map(|(j,)| j)
            .filter(|j| *j > 0)
            .map(|j| j as u16)
            .collect())
    }

    async fn count_available_by_team_id(&self, team_id: &TeamId) -> Result<usize, RepositoryError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM players_proj \
             WHERE team_id = $1 AND membership = 'Active' \
               AND participation_status = 'Available'",
        )
        .bind(&team_id.0)
        .fetch_one(&self.pool)
        .await
        .map_err(RepositoryError::Database)?;

        Ok(row.0.max(0) as usize)
    }
}
