use crate::app::competitions::domain::match_day::{
    MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing,
};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError,
};
use crate::app::shared_kernel::common_types::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::date_string::DateString;
use crate::app::shared_kernel::team::TeamId;
use async_trait::async_trait;
use sqlx::PgPool;

fn db_err(e: sqlx::Error) -> MatchDayRepositoryError {
    MatchDayRepositoryError::Database(e.to_string())
}

fn data_err(msg: &str) -> MatchDayRepositoryError {
    MatchDayRepositoryError::Database(msg.to_string())
}

fn parse_match_day(
    id: String,
    season_id: String,
    name: String,
    day_type: String,
    date_start: Option<String>,
    date_end: Option<String>,
    position: i32,
    pairings: Vec<Pairing>,
) -> Result<MatchDay, MatchDayRepositoryError> {
    Ok(MatchDay {
        id: MatchId::try_new(&id).map_err(|_| data_err("invalid match day id"))?,
        season_id: SeasonId::try_new(&season_id).map_err(|_| data_err("invalid season id"))?,
        name: MatchDayName::try_new(name).map_err(|_| data_err("invalid match day name"))?,
        day_type: MatchDayType::from_str(&day_type),
        date_start: date_start
            .map(|s| DateString::try_new(s).map_err(|_| data_err("invalid date_start")))
            .transpose()?,
        date_end: date_end
            .map(|s| DateString::try_new(s).map_err(|_| data_err("invalid date_end")))
            .transpose()?,
        position: MatchDayPosition::try_new(position)
            .map_err(|_| data_err("invalid position"))?,
        pairings,
    })
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
            result.push(parse_match_day(
                d.id, d.season_id, d.name, d.day_type, d.date_start, d.date_end, d.position,
                pairings,
            )?);
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
        Ok(Some(parse_match_day(
            d.id, d.season_id, d.name, d.day_type, d.date_start, d.date_end, d.position,
            pairings,
        )?))
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
        .bind(match_day.id.to_string())
        .bind(match_day.season_id.to_string())
        .bind(match_day.name.as_ref())
        .bind(match_day.day_type.as_str())
        .bind(match_day.date_start.as_ref().map(|d| d.as_ref()))
        .bind(match_day.date_end.as_ref().map(|d| d.as_ref()))
        .bind(match_day.position.into_inner())
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
        .bind(pairing.id.to_string())
        .bind(match_day_id)
        .bind(pairing.home_team_id.to_string())
        .bind(pairing.away_team_id.to_string())
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
                 ON CONFLICT (season_id, position) DO UPDATE SET
                   name = EXCLUDED.name,
                   day_type = EXCLUDED.day_type,
                   date_start = EXCLUDED.date_start,
                   date_end = EXCLUDED.date_end",
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

        let mut pairings = Vec::with_capacity(rows.len());
        for r in rows {
            pairings.push(Pairing {
                id: PairingId::try_new(&r.id).map_err(|_| data_err("invalid pairing id"))?,
                home_team_id: TeamId::try_new(&r.home_team_id)
                    .map_err(|_| data_err("invalid home_team_id"))?,
                away_team_id: TeamId::try_new(&r.away_team_id)
                    .map_err(|_| data_err("invalid away_team_id"))?,
            });
        }
        Ok(pairings)
    }
}
