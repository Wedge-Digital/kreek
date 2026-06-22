use crate::app::competitions::domain::match_day::{MatchDay, MatchDayType, Pairing};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: sqlx::Error) -> MatchDayRepositoryError {
    MatchDayRepositoryError::Database(e.to_string())
}

pub struct MatchDayRepository {
    pool: PgPool,
}

impl MatchDayRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl IMatchDayRepository for MatchDayRepository {
    async fn find_by_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<MatchDay>, MatchDayRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct DayRow {
            id: String,
            season_id: String,
            name: String,
            day_type: String,
            date_start: Option<String>,
            date_end: Option<String>,
            position: i32,
        }

        let day_rows = sqlx::query_as::<_, DayRow>(
            "SELECT id, season_id, name, day_type, date_start, date_end, position
             FROM competition_match_days
             WHERE season_id = $1
             ORDER BY position, name",
        )
        .bind(season_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        let mut result = Vec::with_capacity(day_rows.len());
        for d in day_rows {
            let pairings = self.load_pairings(&d.id).await?;
            result.push(MatchDay {
                id: d.id,
                season_id: d.season_id,
                name: d.name,
                day_type: MatchDayType::from_str(&d.day_type),
                date_start: d.date_start,
                date_end: d.date_end,
                position: d.position,
                pairings,
            });
        }

        Ok(result)
    }

    async fn find_by_id(
        &self,
        match_day_id: &str,
    ) -> Result<Option<MatchDay>, MatchDayRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct DayRow {
            id: String,
            season_id: String,
            name: String,
            day_type: String,
            date_start: Option<String>,
            date_end: Option<String>,
            position: i32,
        }

        let row = sqlx::query_as::<_, DayRow>(
            "SELECT id, season_id, name, day_type, date_start, date_end, position
             FROM competition_match_days WHERE id = $1",
        )
        .bind(match_day_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        let Some(d) = row else { return Ok(None) };

        let pairings = self.load_pairings(&d.id).await?;
        Ok(Some(MatchDay {
            id: d.id,
            season_id: d.season_id,
            name: d.name,
            day_type: MatchDayType::from_str(&d.day_type),
            date_start: d.date_start,
            date_end: d.date_end,
            position: d.position,
            pairings,
        }))
    }

    async fn save_match_day(
        &self,
        match_day: &MatchDay,
    ) -> Result<(), MatchDayRepositoryError> {
        sqlx::query(
            "INSERT INTO competition_match_days (id, season_id, name, day_type, date_start, date_end, position)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (id) DO UPDATE SET
               name = EXCLUDED.name,
               day_type = EXCLUDED.day_type,
               date_start = EXCLUDED.date_start,
               date_end = EXCLUDED.date_end,
               position = EXCLUDED.position",
        )
        .bind(&match_day.id)
        .bind(&match_day.season_id)
        .bind(&match_day.name)
        .bind(match_day.day_type.as_str())
        .bind(&match_day.date_start)
        .bind(&match_day.date_end)
        .bind(match_day.position)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete_match_day(
        &self,
        match_day_id: &str,
    ) -> Result<(), MatchDayRepositoryError> {
        sqlx::query("DELETE FROM competition_match_days WHERE id = $1")
            .bind(match_day_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn save_pairing(
        &self,
        match_day_id: &str,
        pairing: &Pairing,
    ) -> Result<(), MatchDayRepositoryError> {
        sqlx::query(
            "INSERT INTO competition_match_day_pairings (id, match_day_id, home_team_id, away_team_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&pairing.id)
        .bind(match_day_id)
        .bind(&pairing.home_team_id)
        .bind(&pairing.away_team_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete_pairing(
        &self,
        pairing_id: &str,
    ) -> Result<(), MatchDayRepositoryError> {
        sqlx::query("DELETE FROM competition_match_day_pairings WHERE id = $1")
            .bind(pairing_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn clear_pairings(
        &self,
        match_day_id: &str,
    ) -> Result<(), MatchDayRepositoryError> {
        sqlx::query("DELETE FROM competition_match_day_pairings WHERE match_day_id = $1")
            .bind(match_day_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn clear_all_pairings(
        &self,
        season_id: &str,
    ) -> Result<(), MatchDayRepositoryError> {
        sqlx::query(
            "DELETE FROM competition_match_day_pairings
             WHERE match_day_id IN (SELECT id FROM competition_match_days WHERE season_id = $1)",
        )
        .bind(season_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn ensure_match_days_from_structure(
        &self,
        season_id: &str,
        entries: &[(String, String, String, Option<String>, Option<String>)],
    ) -> Result<(), MatchDayRepositoryError> {
        for (i, (id, name, day_type, date_start, date_end)) in entries.iter().enumerate() {
            sqlx::query(
                "INSERT INTO competition_match_days (id, season_id, name, day_type, date_start, date_end, position)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(id)
            .bind(season_id)
            .bind(name)
            .bind(day_type)
            .bind(date_start)
            .bind(date_end)
            .bind(i as i32)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        }
        Ok(())
    }
}

impl MatchDayRepository {
    async fn load_pairings(
        &self,
        match_day_id: &str,
    ) -> Result<Vec<Pairing>, MatchDayRepositoryError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: String,
            home_team_id: String,
            away_team_id: String,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT id, home_team_id, away_team_id
             FROM competition_match_day_pairings
             WHERE match_day_id = $1",
        )
        .bind(match_day_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows
            .into_iter()
            .map(|r| Pairing {
                id: r.id,
                home_team_id: r.home_team_id,
                away_team_id: r.away_team_id,
            })
            .collect())
    }
}
