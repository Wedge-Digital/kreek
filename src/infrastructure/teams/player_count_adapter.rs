use crate::app::teams::ports::IPlayerCountPort;
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PlayerCountAdapter {
    pool: PgPool,
}

impl PlayerCountAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IPlayerCountPort for PlayerCountAdapter {
    async fn count_for_team(&self, team_id: &str) -> u32 {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM players_proj WHERE team_id = $1",
        )
        .bind(team_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or((0,));

        row.0 as u32
    }
}
