use crate::app::competitions::domain::match_day::{
    MatchDay, MatchDayName, MatchDayPosition, MatchDayType, Pairing,
};
use crate::app::competitions::domain::match_day_repository_port::{
    IMatchDayRepository, LatestResultDto, MatchDayRepositoryError, NewPairingProjection,
    PairingDisplayDto,
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

    /// **Pas `list_from_projection`** : celui-ci lie `(season_id, curseur)`, et
    /// cette requête-ci lie `(season_id, team_id)`. Lui faire porter un
    /// troisième cas l'aurait rendu paramétrable, donc moins lisible que les
    /// six lignes qu'il économise.
    async fn list_team_matches(
        &self,
        season_id: &str,
        team_id: &str,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError> {
        let rows = sqlx::query_as::<_, ProjectionRow>(include_str!(
            "sql/match_days/list_team_matches.sql"
        ))
        .bind(season_id)
        .bind(team_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_latest_completed_results(
        &self,
        space_id: &str,
        limit: i64,
    ) -> Result<Vec<LatestResultDto>, MatchDayRepositoryError> {
        let rows = sqlx::query_as::<_, LatestResultRow>(include_str!(
            "sql/match_days/list_latest_results.sql"
        ))
        .bind(space_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(Into::into).collect())
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

#[derive(sqlx::FromRow)]
struct LatestResultRow {
    pairing_id: String,
    season_id: String,
    competition_id: String,
    competition_name: String,
    round_name: String,
    home_team_id: String,
    home_team_name: String,
    home_score: Option<i32>,
    away_team_id: String,
    away_team_name: String,
    away_score: Option<i32>,
    match_report_url: Option<String>,
    published_at: Option<time::OffsetDateTime>,
}

impl From<LatestResultRow> for LatestResultDto {
    fn from(r: LatestResultRow) -> Self {
        Self {
            pairing_id: r.pairing_id,
            season_id: r.season_id,
            competition_id: r.competition_id,
            competition_name: r.competition_name,
            round_name: r.round_name,
            home_team_id: r.home_team_id,
            home_team_name: r.home_team_name,
            home_score: r.home_score,
            away_team_id: r.away_team_id,
            away_team_name: r.away_team_name,
            away_score: r.away_score,
            match_report_url: r.match_report_url,
            published_at: r.published_at,
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn insert_competition(pool: &PgPool, id: &str, space_id: &str, name: &str) {
        sqlx::query(
            "INSERT INTO competitions (id, space_id, name, logo) VALUES ($1, $2, $3, 'logo.png')",
        )
        .bind(id)
        .bind(space_id)
        .bind(name)
        .execute(pool)
        .await
        .expect("insertion de la compétition de test");
    }

    async fn insert_season(pool: &PgPool, id: &str, competition_id: &str) {
        sqlx::query(
            "INSERT INTO competition_seasons (id, competition_id, name) VALUES ($1, $2, 'Saison 1')",
        )
        .bind(id)
        .bind(competition_id)
        .execute(pool)
        .await
        .expect("insertion de la saison de test");
    }

    /// Insère une ligne `completed` minimale dans la projection — les champs
    /// d'affichage hors du périmètre de `list_latest_completed_results`
    /// (roster, coach, initiales…) sont bouchés, seuls comptent `season_id`
    /// et `published_at` pour ce test.
    async fn insert_completed_result(
        pool: &PgPool,
        pairing_id: &str,
        season_id: &str,
        published_at: Option<time::OffsetDateTime>,
    ) {
        sqlx::query(
            "INSERT INTO competition_match_display_proj (
                pairing_id, season_id, round_id, round_name, round_position, round_day_type,
                home_team_id, home_team_name, home_roster_name, home_coach_name, home_initials,
                away_team_id, away_team_name, away_roster_name, away_coach_name, away_initials,
                match_status, home_score, away_score, published_at
            ) VALUES ($1, $2, 'r1', 'Journée 1', 1, 'fixed_date',
                'home', 'Home', 'Roster', 'Coach', 'HO',
                'away', 'Away', 'Roster', 'Coach', 'AW',
                'completed', 2, 1, $3)",
        )
        .bind(pairing_id)
        .bind(season_id)
        .bind(published_at)
        .execute(pool)
        .await
        .expect("insertion du résultat de test");
    }

    /// Un match de la projection, avec son camp, son statut et sa position.
    /// Les champs d'affichage sont bouchés : ce que ces tests vérifient est le
    /// `WHERE` et l'`ORDER BY`, pas le rendu.
    #[allow(clippy::too_many_arguments)]
    async fn insert_match(
        pool: &PgPool,
        pairing_id: &str,
        season_id: &str,
        home: &str,
        away: &str,
        statut: &str,
        position: i32,
    ) {
        sqlx::query(
            "INSERT INTO competition_match_display_proj (
                pairing_id, season_id, round_id, round_name, round_position, round_day_type,
                home_team_id, home_team_name, home_roster_name, home_coach_name, home_initials,
                away_team_id, away_team_name, away_roster_name, away_coach_name, away_initials,
                match_status
            ) VALUES ($1, $2, $3, $4, $5, 'fixed_date',
                $6, 'Home', 'Roster', 'Coach', 'HO',
                $7, 'Away', 'Roster', 'Coach', 'AW',
                $8)",
        )
        .bind(pairing_id)
        .bind(season_id)
        .bind(format!("r{position}"))
        .bind(format!("Journée {position}"))
        .bind(position)
        .bind(home)
        .bind(away)
        .bind(statut)
        .execute(pool)
        .await
        .expect("insertion du match de test");
    }

    /// **Une équipe peut jouer plusieurs saisons.** Sans ce filtre, « Journée 1 »
    /// reviendrait à chaque saison en groupes homonymes et les positions se
    /// répéteraient — c'est ce que la décision de saison courante évite.
    #[sqlx::test]
    async fn les_matchs_d_une_autre_saison_ne_remontent_pas(pool: PgPool) {
        insert_competition(&pool, "comp-a", "space-1", "Ligue A").await;
        insert_season(&pool, "saison-1", "comp-a").await;
        insert_season(&pool, "saison-2", "comp-a").await;
        insert_match(&pool, "p1", "saison-1", "A", "B", "completed", 1).await;
        insert_match(&pool, "p2", "saison-2", "A", "C", "completed", 1).await;

        let repo = MatchDayRepository::new(pool.clone());
        let rows = repo.list_team_matches("saison-1", "A").await.unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pairing_id, "p1");
    }

    /// Le `OR` sur les deux camps : un `WHERE home_team_id = $2` seul perdrait
    /// la moitié des matchs, et la moitié perdue serait celle des déplacements.
    #[sqlx::test]
    async fn un_match_a_domicile_et_un_a_l_exterieur_remontent(pool: PgPool) {
        insert_competition(&pool, "comp-a", "space-1", "Ligue A").await;
        insert_season(&pool, "saison-1", "comp-a").await;
        insert_match(&pool, "dom", "saison-1", "A", "B", "completed", 1).await;
        insert_match(&pool, "ext", "saison-1", "C", "A", "completed", 2).await;
        insert_match(&pool, "sans", "saison-1", "B", "C", "completed", 3).await;

        let repo = MatchDayRepository::new(pool.clone());
        let rows = repo.list_team_matches("saison-1", "A").await.unwrap();

        let ids: Vec<&str> = rows.iter().map(|r| r.pairing_id.as_str()).collect();
        assert_eq!(ids.len(), 2, "les deux camps, et eux seuls : {ids:?}");
        assert!(ids.contains(&"dom") && ids.contains(&"ext"));
    }

    /// **« Mon prochain match » est ce qu'un coach vient chercher.** Reprendre
    /// le `round_position DESC` de l'onglet compétition en incluant les matchs
    /// à venir mettrait le plus lointain en tête.
    #[sqlx::test]
    async fn le_prochain_match_est_en_tete(pool: PgPool) {
        insert_competition(&pool, "comp-a", "space-1", "Ligue A").await;
        insert_season(&pool, "saison-1", "comp-a").await;
        insert_match(&pool, "joue-1", "saison-1", "A", "B", "completed", 1).await;
        insert_match(&pool, "joue-2", "saison-1", "A", "C", "completed", 2).await;
        insert_match(&pool, "prochain", "saison-1", "A", "D", "upcoming", 3).await;
        insert_match(&pool, "lointain", "saison-1", "A", "E", "upcoming", 9).await;

        let repo = MatchDayRepository::new(pool.clone());
        let rows = repo.list_team_matches("saison-1", "A").await.unwrap();

        let ids: Vec<&str> = rows.iter().map(|r| r.pairing_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["prochain", "lointain", "joue-2", "joue-1"],
            "à venir du plus proche au plus lointain, puis joués du plus récent"
        );
    }

    /// La seconde moitié de l'ordre : un match en cours de saisie passe devant
    /// tout le reste, quelle que soit sa position dans le calendrier.
    #[sqlx::test]
    async fn un_match_en_cours_passe_devant_les_a_venir(pool: PgPool) {
        insert_competition(&pool, "comp-a", "space-1", "Ligue A").await;
        insert_season(&pool, "saison-1", "comp-a").await;
        // **Les positions contredisent l'ordre attendu**, et c'est délibéré :
        // avec un `round_position DESC` nu, ces trois-là sortiraient exactement
        // à l'envers. Une première rédaction leur avait donné des positions qui
        // coïncidaient avec le bon ordre — le test passait alors même sans le
        // `CASE` sur le statut, donc ne prouvait rien.
        insert_match(&pool, "joue", "saison-1", "A", "D", "completed", 9).await;
        insert_match(&pool, "a-venir", "saison-1", "A", "B", "upcoming", 5).await;
        insert_match(&pool, "en-cours", "saison-1", "A", "C", "in_progress", 1).await;

        let repo = MatchDayRepository::new(pool.clone());
        let rows = repo.list_team_matches("saison-1", "A").await.unwrap();

        let ids: Vec<&str> = rows.iter().map(|r| r.pairing_id.as_str()).collect();
        assert_eq!(ids, vec!["en-cours", "a-venir", "joue"]);
    }

    #[sqlx::test]
    async fn list_latest_completed_results_orders_across_competitions_and_respects_limit(
        pool: PgPool,
    ) {
        insert_competition(&pool, "comp-a", "space-1", "Ligue A").await;
        insert_competition(&pool, "comp-b", "space-1", "Ligue B").await;
        insert_season(&pool, "season-a", "comp-a").await;
        insert_season(&pool, "season-b", "comp-b").await;

        let older = time::OffsetDateTime::from_unix_timestamp(1_754_000_000).unwrap();
        let newer = time::OffsetDateTime::from_unix_timestamp(1_754_100_000).unwrap();
        insert_completed_result(&pool, "p-old", "season-a", Some(older)).await;
        insert_completed_result(&pool, "p-new", "season-b", Some(newer)).await;
        insert_completed_result(&pool, "p-null", "season-a", None).await;

        let repo = MatchDayRepository::new(pool.clone());
        let results = repo
            .list_latest_completed_results("space-1", 2)
            .await
            .unwrap();

        assert_eq!(results.len(), 2, "limit doit être respecté");
        assert_eq!(results[0].pairing_id, "p-new", "le plus récent d'abord");
        assert_eq!(results[0].competition_name, "Ligue B");
        assert_eq!(results[1].pairing_id, "p-old");
    }

    #[sqlx::test]
    async fn list_latest_completed_results_puts_null_published_at_last(pool: PgPool) {
        insert_competition(&pool, "comp-a", "space-1", "Ligue A").await;
        insert_season(&pool, "season-a", "comp-a").await;

        let dated = time::OffsetDateTime::from_unix_timestamp(1_754_000_000).unwrap();
        insert_completed_result(&pool, "p-dated", "season-a", Some(dated)).await;
        insert_completed_result(&pool, "p-null", "season-a", None).await;

        let repo = MatchDayRepository::new(pool.clone());
        let results = repo
            .list_latest_completed_results("space-1", 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].pairing_id, "p-dated");
        assert_eq!(results[1].pairing_id, "p-null");
    }

    #[sqlx::test]
    async fn list_latest_completed_results_only_includes_the_given_space(pool: PgPool) {
        insert_competition(&pool, "comp-a", "space-1", "Ligue A").await;
        insert_competition(&pool, "comp-b", "space-2", "Ligue B").await;
        insert_season(&pool, "season-a", "comp-a").await;
        insert_season(&pool, "season-b", "comp-b").await;
        insert_completed_result(&pool, "p-a", "season-a", None).await;
        insert_completed_result(&pool, "p-b", "season-b", None).await;

        let repo = MatchDayRepository::new(pool.clone());
        let results = repo
            .list_latest_completed_results("space-1", 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].pairing_id, "p-a");
    }
}
