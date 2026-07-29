use crate::app::competitions::domain::match_day::{
    MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing,
};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, MatchDayRepositoryError, NewPairingProjection, PairingDisplayDto,
};
use crate::app::shared_kernel::bloodbowl::date_string::DateString;
use crate::app::shared_kernel::bloodbowl::ids::{MatchId, PairingId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::common::initials::initials;
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
        position: MatchDayPosition::try_new(position).map_err(|_| data_err("invalid position"))?,
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
                d.id,
                d.season_id,
                d.name,
                d.day_type,
                d.date_start,
                d.date_end,
                d.position,
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
            d.id,
            d.season_id,
            d.name,
            d.day_type,
            d.date_start,
            d.date_end,
            d.position,
            pairings,
        )?))
    }

    async fn save_match_day(&self, match_day: &MatchDay) -> Result<(), MatchDayRepositoryError> {
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

    async fn delete_match_day(&self, match_day_id: &str) -> Result<(), MatchDayRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        // Cascade DB sur competition_match_day_pairings, mais pas sur la
        // projection (pas de FK) — nettoyage explicite dans la même tx.
        sqlx::query("DELETE FROM competition_match_days WHERE id = $1")
            .bind(match_day_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx::query("DELETE FROM competition_match_display_proj WHERE round_id = $1")
            .bind(match_day_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn find_pairing_id(
        &self,
        match_day_id: &str,
        home_team_id: &str,
        away_team_id: &str,
    ) -> Result<Option<String>, MatchDayRepositoryError> {
        sqlx::query_scalar(
            "SELECT id FROM competition_match_day_pairings
             WHERE match_day_id = $1 AND home_team_id = $2 AND away_team_id = $3
             LIMIT 1",
        )
        .bind(match_day_id)
        .bind(home_team_id)
        .bind(away_team_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)
    }

    async fn save_pairing(
        &self,
        match_day_id: &str,
        pairing: &Pairing,
        projection: &NewPairingProjection,
    ) -> Result<(), MatchDayRepositoryError> {
        let pairing_id = pairing.id.to_string();
        let home_team_id = pairing.home_team_id.to_string();
        let away_team_id = pairing.away_team_id.to_string();
        let home_initials = initials(&projection.home_team_name);
        let away_initials = initials(&projection.away_team_name);

        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query(
            "INSERT INTO competition_match_day_pairings (id, match_day_id, home_team_id, away_team_id)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&pairing_id)
        .bind(match_day_id)
        .bind(&home_team_id)
        .bind(&away_team_id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        sqlx::query(
            r#"INSERT INTO competition_match_display_proj (
                pairing_id, season_id, round_id, round_name, round_position,
                round_date_start, round_date_end, round_day_type,
                home_team_id, home_team_name, home_roster_name, home_coach_name, home_logo_url, home_initials,
                away_team_id, away_team_name, away_roster_name, away_coach_name, away_logo_url, away_initials,
                match_status
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20,
                'upcoming'
            ) ON CONFLICT (pairing_id) DO NOTHING"#,
        )
        .bind(&pairing_id)
        .bind(&projection.season_id)
        .bind(match_day_id)
        .bind(&projection.round_name)
        .bind(projection.round_position)
        .bind(&projection.round_date_start)
        .bind(&projection.round_date_end)
        .bind(&projection.round_day_type)
        .bind(&home_team_id)
        .bind(&projection.home_team_name)
        .bind(&projection.home_roster_name)
        .bind(&projection.home_coach_name)
        .bind(&projection.home_logo_url)
        .bind(&home_initials)
        .bind(&away_team_id)
        .bind(&projection.away_team_name)
        .bind(&projection.away_roster_name)
        .bind(&projection.away_coach_name)
        .bind(&projection.away_logo_url)
        .bind(&away_initials)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn delete_pairing(&self, pairing_id: &str) -> Result<(), MatchDayRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        sqlx::query("DELETE FROM competition_match_day_pairings WHERE id = $1")
            .bind(pairing_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx::query("DELETE FROM competition_match_display_proj WHERE pairing_id = $1")
            .bind(pairing_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn list_resultats(
        &self,
        season_id: &str,
        cursor_position: Option<i32>,
        limit_rounds: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
        list_from_projection(
            &self.pool,
            season_id,
            cursor_position,
            limit_rounds,
            include_str!("sql/match_days/list_resultats.sql"),
        )
        .await
    }

    async fn list_calendrier(
        &self,
        season_id: &str,
        cursor_position: Option<i32>,
        limit_rounds: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
        list_from_projection(
            &self.pool,
            season_id,
            cursor_position,
            limit_rounds,
            include_str!("sql/match_days/list_calendrier.sql"),
        )
        .await
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

#[derive(sqlx::FromRow)]
struct ProjectionRow {
    pairing_id: String,
    round_id: String,
    round_name: String,
    round_position: i32,
    round_date_start: Option<String>,
    round_date_end: Option<String>,
    round_day_type: String,
    home_team_id: String,
    home_team_name: String,
    home_roster_name: String,
    home_coach_name: String,
    home_logo_url: Option<String>,
    home_initials: String,
    away_team_id: String,
    away_team_name: String,
    away_roster_name: String,
    away_coach_name: String,
    away_logo_url: Option<String>,
    away_initials: String,
    match_status: String,
    home_score: Option<i32>,
    away_score: Option<i32>,
    home_casualties: Option<i32>,
    away_casualties: Option<i32>,
    match_report_url: Option<String>,
}

impl From<ProjectionRow> for PairingDisplayDto {
    fn from(r: ProjectionRow) -> Self {
        Self {
            pairing_id: r.pairing_id,
            round_id: r.round_id,
            round_name: r.round_name,
            round_position: r.round_position,
            round_date_start: r.round_date_start,
            round_date_end: r.round_date_end,
            round_day_type: r.round_day_type,
            home_team_id: r.home_team_id,
            home_team_name: r.home_team_name,
            home_roster_name: r.home_roster_name,
            home_coach_name: r.home_coach_name,
            home_logo_url: r.home_logo_url,
            home_initials: r.home_initials,
            away_team_id: r.away_team_id,
            away_team_name: r.away_team_name,
            away_roster_name: r.away_roster_name,
            away_coach_name: r.away_coach_name,
            away_logo_url: r.away_logo_url,
            away_initials: r.away_initials,
            match_status: r.match_status,
            home_score: r.home_score,
            away_score: r.away_score,
            home_casualties: r.home_casualties,
            away_casualties: r.away_casualties,
            match_report_url: r.match_report_url,
        }
    }
}

async fn list_from_projection(
    pool: &PgPool,
    season_id: &str,
    cursor_position: Option<i32>,
    _limit_rounds: u32,
    sql: &str,
) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
    let rows = sqlx::query_as::<_, ProjectionRow>(sql)
        .bind(season_id)
        .bind(cursor_position)
        .fetch_all(pool)
        .await
        .map_err(db_err)?;

    Ok(rows.into_iter().map(Into::into).collect())
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
