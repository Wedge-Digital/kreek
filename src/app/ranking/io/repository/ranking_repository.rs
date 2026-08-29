use crate::app::ranking::domain::ranking_line::RankingLine;
use crate::app::ranking::ports::{
    IRankingRepository, ManualPointRow, RankingLineFullRow, RankingLineRow, RankingRepositoryError,
};
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::sulid::SUlid;
use async_trait::async_trait;
use sqlx::PgPool;
use std::collections::HashMap;

pub struct PgRankingRepository {
    pool: PgPool,
}

impl PgRankingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db_err(e: sqlx::Error) -> RankingRepositoryError {
    RankingRepositoryError::Database(e.to_string())
}

struct Row {
    team_id: String,
    matches_played: i32,
    wins: i32,
    draws: i32,
    losses: i32,
    ranking_points: i32,
    bonus_points: i32,
    td_for: i32,
    td_against: i32,
    casualties: i32,
    fouls: i32,
    completions: i32,
}

/// Le miroir de `RankingLineFullRow` : les mêmes colonnes, plus le contexte.
#[derive(sqlx::FromRow)]
struct FullRow {
    team_id: String,
    competition_id: String,
    season_id: String,
    round_id: String,
    match_report_id: String,
    recorded_at: time::OffsetDateTime,
    matches_played: i32,
    wins: i32,
    draws: i32,
    losses: i32,
    ranking_points: i32,
    bonus_points: i32,
    td_for: i32,
    td_against: i32,
    casualties: i32,
    fouls: i32,
    completions: i32,
}

impl TryFrom<FullRow> for RankingLineFullRow {
    type Error = RankingRepositoryError;

    fn try_from(r: FullRow) -> Result<Self, Self::Error> {
        Ok(RankingLineFullRow {
            team_id: TeamId::try_new(&r.team_id).map_err(|e| {
                RankingRepositoryError::MalformedRow(format!("team_id « {} » : {e}", r.team_id))
            })?,
            competition_id: decoder("competition_id", &r.competition_id, CompetitionId::try_new)?,
            season_id: decoder("season_id", &r.season_id, SeasonId::try_new)?,
            round_id: decoder("round_id", &r.round_id, RoundId::try_new)?,
            match_report_id: decoder(
                "match_report_id",
                &r.match_report_id,
                MatchReportId::try_new,
            )?,
            // sqlx est compilé avec `time`, le domaine parle `chrono` : la
            // conversion a lieu ici, comme à l'insertion et en sens inverse.
            recorded_at: chrono::DateTime::from_timestamp_nanos(
                (r.recorded_at.unix_timestamp_nanos()) as i64,
            ),
            matches_played: r.matches_played as u32,
            wins: r.wins as u32,
            draws: r.draws as u32,
            losses: r.losses as u32,
            ranking_points: r.ranking_points as u32,
            bonus_points: r.bonus_points as u32,
            td_for: r.td_for as u32,
            td_against: r.td_against as u32,
            casualties: r.casualties as u32,
            fouls: r.fouls as u32,
            completions: r.completions as u32,
        })
    }
}

/// Décode un identifiant stocké en `TEXT`, en nommant la colonne fautive.
///
/// Sans lui, chaque champ portait son propre `map_err` — cinq fois la même
/// phrase, et l'occasion d'en oublier un.
fn decoder<T, E: std::fmt::Display>(
    colonne: &str,
    brut: &str,
    parse: impl Fn(&str) -> Result<T, E>,
) -> Result<T, RankingRepositoryError> {
    parse(brut)
        .map_err(|e| RankingRepositoryError::MalformedRow(format!("{colonne} « {brut} » : {e}")))
}

/// Faillible, contrairement à un `From` : `team_id` est stocké en `TEXT` et doit
/// être décodé vers un ULID. Une ligne illisible fait échouer la lecture entière
/// plutôt que de disparaître du classement — cf. `MalformedRow`.
impl TryFrom<Row> for RankingLineRow {
    type Error = RankingRepositoryError;

    fn try_from(r: Row) -> Result<Self, Self::Error> {
        Ok(RankingLineRow {
            team_id: TeamId::try_new(&r.team_id).map_err(|e| {
                RankingRepositoryError::MalformedRow(format!("team_id « {} » : {e}", r.team_id))
            })?,
            matches_played: r.matches_played as u32,
            wins: r.wins as u32,
            draws: r.draws as u32,
            losses: r.losses as u32,
            ranking_points: r.ranking_points as u32,
            bonus_points: r.bonus_points as u32,
            td_for: r.td_for as u32,
            td_against: r.td_against as u32,
            casualties: r.casualties as u32,
            fouls: r.fouls as u32,
            completions: r.completions as u32,
        })
    }
}

#[async_trait]
impl IRankingRepository for PgRankingRepository {
    async fn find_latest_line(
        &self,
        season_id: &str,
        team_id: &str,
    ) -> Result<Option<RankingLineRow>, RankingRepositoryError> {
        let row = sqlx::query_as!(
            Row,
            r#"SELECT team_id, matches_played, wins, draws, losses, ranking_points, bonus_points,
                      td_for, td_against, casualties, fouls, completions
               FROM ranking_lines
               WHERE season_id = $1 AND team_id = $2
               ORDER BY sequence DESC
               LIMIT 1"#,
            season_id,
            team_id,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        row.map(RankingLineRow::try_from).transpose()
    }

    async fn find_latest_lines_for_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<RankingLineRow>, RankingRepositoryError> {
        let rows = sqlx::query_as!(
            Row,
            r#"SELECT DISTINCT ON (team_id) team_id, matches_played, wins, draws, losses,
                      ranking_points, bonus_points, td_for, td_against, casualties,
                      fouls, completions
               FROM ranking_lines
               WHERE season_id = $1
               ORDER BY team_id, sequence DESC"#,
            season_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        rows.into_iter().map(RankingLineRow::try_from).collect()
    }

    async fn delete_lines_for_match(
        &self,
        match_report_id: &str,
    ) -> Result<(), RankingRepositoryError> {
        sqlx::query!(
            "DELETE FROM ranking_lines WHERE match_report_id = $1",
            match_report_id,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn insert_lines(&self, lines: &[RankingLine]) -> Result<(), RankingRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;
        for line in lines {
            insert_line_in_tx(&mut tx, line).await?;
        }
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn find_all_lines_for_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<RankingLineFullRow>, RankingRepositoryError> {
        let rows = sqlx::query_as!(
            FullRow,
            r#"SELECT team_id, competition_id, season_id, round_id, match_report_id,
                      recorded_at, matches_played, wins, draws, losses, ranking_points,
                      bonus_points, td_for, td_against, casualties, fouls, completions
               FROM ranking_lines
               WHERE season_id = $1
               ORDER BY sequence ASC"#,
            season_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        rows.into_iter().map(RankingLineFullRow::try_from).collect()
    }

    async fn replace_lines_for_season(
        &self,
        season_id: &str,
        lines: &[RankingLine],
    ) -> Result<(), RankingRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query!("DELETE FROM ranking_lines WHERE season_id = $1", season_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for line in lines {
            insert_line_in_tx(&mut tx, line).await?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    // ── Points manuels (carte 450) ────────────────────────────────────────────

    async fn find_manual_totals_for_season(
        &self,
        season_id: &str,
    ) -> Result<HashMap<String, i32>, RankingRepositoryError> {
        // `SUM` rend `numeric` et vaut `NULL` sur un groupe vide — impossible
        // ici puisque `GROUP BY` n'engendre pas de groupe sans ligne, mais sqlx
        // ne le sait pas. Le `::int` et le `!` le lui disent.
        let rows = sqlx::query!(
            r#"SELECT team_id, SUM(points)::int AS "total!"
               FROM ranking__manual_points
               WHERE season_id = $1
               GROUP BY team_id"#,
            season_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows.into_iter().map(|r| (r.team_id, r.total)).collect())
    }

    async fn list_manual_points(
        &self,
        season_id: &str,
    ) -> Result<Vec<ManualPointRow>, RankingRepositoryError> {
        // Équipe puis date : la page groupe par équipe, et à l'intérieur d'un
        // groupe on lit les décisions dans l'ordre où elles ont été prises.
        let rows = sqlx::query!(
            r#"SELECT id, team_id, points, reason, awarded_by, awarded_at
               FROM ranking__manual_points
               WHERE season_id = $1
               ORDER BY team_id, awarded_at, id"#,
            season_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(rows
            .into_iter()
            .map(|r| ManualPointRow {
                id: r.id,
                team_id: r.team_id,
                points: r.points,
                reason: r.reason,
                awarded_by: r.awarded_by,
                awarded_at: r.awarded_at,
            })
            .collect())
    }

    async fn insert_manual_points(
        &self,
        season_id: &str,
        team_id: &str,
        points: i32,
        reason: &str,
        awarded_by: &str,
    ) -> Result<(), RankingRepositoryError> {
        sqlx::query!(
            r#"INSERT INTO ranking__manual_points
                   (season_id, team_id, points, reason, awarded_by)
               VALUES ($1, $2, $3, $4, $5)"#,
            season_id,
            team_id,
            points,
            reason,
            awarded_by,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete_manual_points(
        &self,
        id: i64,
        season_id: &str,
    ) -> Result<u64, RankingRepositoryError> {
        let r = sqlx::query!(
            "DELETE FROM ranking__manual_points WHERE id = $1 AND season_id = $2",
            id,
            season_id,
        )
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(r.rows_affected())
    }
}

/// L'insertion d'une ligne, partagée par `insert_lines` et
/// `replace_lines_for_season`.
///
/// Extraite plutôt que dupliquée : les dix-huit colonnes se modifient ensemble,
/// et deux copies divergeraient le jour où l'une gagne une colonne.
async fn insert_line_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    line: &RankingLine,
) -> Result<(), RankingRepositoryError> {
    let id = SUlid::new().to_string();
    // sqlx est compilé avec la feature `time`, pas `chrono` — conversion
    // nécessaire au point de persistance (le domaine reste en chrono::DateTime<Utc>).
    let recorded_at = time::OffsetDateTime::from_unix_timestamp_nanos(
        line.recorded_at.timestamp_nanos_opt().unwrap_or(0) as i128,
    )
    .map_err(|e| RankingRepositoryError::Database(e.to_string()))?;
    sqlx::query!(
        r#"INSERT INTO ranking_lines (
            id, competition_id, season_id, round_id, match_report_id, team_id,
            recorded_at, matches_played, wins, draws, losses, ranking_points,
            bonus_points, td_for, td_against, casualties, fouls, completions
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                  $15, $16, $17, $18)"#,
        id,
        line.competition_id.to_string(),
        line.season_id.to_string(),
        line.round_id.to_string(),
        line.match_report_id.to_string(),
        line.team_id.to_string(),
        recorded_at,
        line.matches_played.0 as i32,
        line.wins.0 as i32,
        line.draws.0 as i32,
        line.losses.0 as i32,
        line.ranking_points.0 as i32,
        line.bonus_points.0 as i32,
        line.td_for.0 as i32,
        line.td_against.0 as i32,
        line.casualties.0 as i32,
        line.fouls.0 as i32,
        line.completions.0 as i32,
    )
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::ranking_line::{
        CasualtiesTotal, CompletionsMade, DrawCount, FoulsCommitted, LossCount, MatchesPlayed,
        RankingPoints, TdAgainst, TdFor, WinCount,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use chrono::Utc;

    #[allow(clippy::too_many_arguments)]
    fn sample_line(
        team_id: &TeamId,
        season_id: &SeasonId,
        matches_played: u32,
        wins: u32,
        draws: u32,
        losses: u32,
        points: u32,
        bonus: u32,
    ) -> RankingLine {
        RankingLine {
            team_id: team_id.clone(),
            competition_id: CompetitionId::new(),
            season_id: season_id.clone(),
            round_id: RoundId::new(),
            match_report_id: MatchReportId::new(),
            recorded_at: Utc::now(),
            matches_played: MatchesPlayed(matches_played),
            wins: WinCount(wins),
            draws: DrawCount(draws),
            losses: LossCount(losses),
            ranking_points: RankingPoints(points),
            bonus_points: RankingPoints(bonus),
            td_for: TdFor(0),
            td_against: TdAgainst(0),
            casualties: CasualtiesTotal(0),
            fouls: FoulsCommitted(0),
            completions: CompletionsMade(0),
        }
    }

    /// Renseigne les compteurs de départage sur une ligne — séparé de
    /// `sample_line` pour ne pas lui ajouter cinq paramètres de plus.
    fn with_counters(mut line: RankingLine, counters: [u32; 5]) -> RankingLine {
        let [td_for, td_against, casualties, fouls, completions] = counters;
        line.td_for = TdFor(td_for);
        line.td_against = TdAgainst(td_against);
        line.casualties = CasualtiesTotal(casualties);
        line.fouls = FoulsCommitted(fouls);
        line.completions = CompletionsMade(completions);
        line
    }

    #[sqlx::test]
    async fn insert_lines_writes_both_lines_of_a_match_atomically(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season_id = SeasonId::new();
        let home = TeamId::new();
        let away = TeamId::new();

        let home_line = sample_line(&home, &season_id, 1, 1, 0, 0, 4, 1);
        let away_line = sample_line(&away, &season_id, 1, 0, 0, 1, 0, 0);
        repo.insert_lines(&[home_line, away_line]).await.unwrap();

        let home_row = repo
            .find_latest_line(&season_id.to_string(), &home.to_string())
            .await
            .unwrap();
        let away_row = repo
            .find_latest_line(&season_id.to_string(), &away.to_string())
            .await
            .unwrap();

        // La part bonus fait l'aller-retour en base, distincte du total qui la contient.
        let home_row = home_row.unwrap();
        assert_eq!(home_row.ranking_points, 4);
        assert_eq!(home_row.bonus_points, 1);
        let away_row = away_row.unwrap();
        assert_eq!(away_row.ranking_points, 0);
        assert_eq!(away_row.bonus_points, 0);
    }

    #[sqlx::test]
    async fn find_latest_line_follows_insertion_order_not_recorded_at(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season_id = SeasonId::new();
        let team_id = TeamId::new();

        let mut first = sample_line(&team_id, &season_id, 1, 1, 0, 0, 3, 0);
        first.recorded_at = Utc::now() + chrono::Duration::hours(1); // horodatage "futur"
        repo.insert_lines(&[first]).await.unwrap();

        let mut second = sample_line(&team_id, &season_id, 2, 1, 0, 1, 3, 0);
        second.recorded_at = Utc::now() - chrono::Duration::hours(1); // horodatage "passé", inséré après
        repo.insert_lines(&[second]).await.unwrap();

        // La ligne insérée en second doit faire foi, même si son recorded_at est antérieur —
        // c'est l'ordre d'enregistrement (sequence) qui compte, jamais le timestamp seul.
        let latest = repo
            .find_latest_line(&season_id.to_string(), &team_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(latest.matches_played, 2);
        assert_eq!(latest.losses, 1);
    }

    #[sqlx::test]
    async fn find_latest_lines_for_season_returns_one_row_per_team(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season_id = SeasonId::new();
        let team_a = TeamId::new();
        let team_b = TeamId::new();

        repo.insert_lines(&[sample_line(&team_a, &season_id, 1, 1, 0, 0, 3, 0)])
            .await
            .unwrap();
        repo.insert_lines(&[sample_line(&team_a, &season_id, 2, 1, 1, 0, 4, 0)])
            .await
            .unwrap();
        repo.insert_lines(&[sample_line(&team_b, &season_id, 1, 0, 0, 1, 0, 0)])
            .await
            .unwrap();

        let rows = repo
            .find_latest_lines_for_season(&season_id.to_string())
            .await
            .unwrap();

        assert_eq!(rows.len(), 2);
        let row_a = rows.iter().find(|r| r.team_id == team_a).unwrap();
        assert_eq!(row_a.matches_played, 2); // la dernière ligne de team_a, pas la première
        let row_b = rows.iter().find(|r| r.team_id == team_b).unwrap();
        assert_eq!(row_b.matches_played, 1);
    }

    /// Aller-retour des cinq compteurs de départage par les deux SELECT. Cinq
    /// valeurs distinctes : une colonne permutée à l'INSERT ou au SELECT compile
    /// et renvoie des nombres plausibles — seul l'écart entre elles le révèle.
    #[sqlx::test]
    async fn tiebreak_counters_round_trip_through_both_selects(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season_id = SeasonId::new();
        let team_id = TeamId::new();

        let line = with_counters(
            sample_line(&team_id, &season_id, 1, 1, 0, 0, 3, 0),
            [7, 3, 5, 2, 9],
        );
        repo.insert_lines(&[line]).await.unwrap();

        let row = repo
            .find_latest_line(&season_id.to_string(), &team_id.to_string())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            [
                row.td_for,
                row.td_against,
                row.casualties,
                row.fouls,
                row.completions
            ],
            [7, 3, 5, 2, 9]
        );

        let rows = repo
            .find_latest_lines_for_season(&season_id.to_string())
            .await
            .unwrap();
        let row = rows.first().unwrap();
        assert_eq!(
            [
                row.td_for,
                row.td_against,
                row.casualties,
                row.fouls,
                row.completions
            ],
            [7, 3, 5, 2, 9]
        );
    }
    // ── delete_lines_for_match — compensation d'une dépublication ────────────

    /// Force le `match_report_id` d'une ligne, `sample_line` en tirant un
    /// aléatoire à chaque appel.
    fn for_match(mut line: RankingLine, match_report_id: MatchReportId) -> RankingLine {
        line.match_report_id = match_report_id;
        line
    }

    #[sqlx::test]
    async fn delete_lines_for_match_supprime_les_deux_lignes_du_match(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (season_id, home, away) = (SeasonId::new(), TeamId::new(), TeamId::new());
        let mr = MatchReportId::new();
        repo.insert_lines(&[
            for_match(sample_line(&home, &season_id, 1, 1, 0, 0, 4, 1), mr),
            for_match(sample_line(&away, &season_id, 1, 0, 0, 1, 0, 0), mr),
        ])
        .await
        .unwrap();

        repo.delete_lines_for_match(&mr.to_string()).await.unwrap();

        assert!(repo
            .find_latest_line(&season_id.to_string(), &home.to_string())
            .await
            .unwrap()
            .is_none());
        assert!(repo
            .find_latest_line(&season_id.to_string(), &away.to_string())
            .await
            .unwrap()
            .is_none());
    }

    /// Le cœur du raisonnement de la compensation : les lignes portant des
    /// **cumuls**, supprimer celles du dernier match fait remonter celles du
    /// précédent, qui portent déjà l'état d'avant. Aucun recalcul nécessaire.
    #[sqlx::test]
    async fn supprimer_le_dernier_match_fait_remonter_les_cumuls_du_precedent(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (season_id, team) = (SeasonId::new(), TeamId::new());
        let (premier, second) = (MatchReportId::new(), MatchReportId::new());

        // Journée 1 : victoire — 1 match, 3 points.
        repo.insert_lines(&[for_match(
            sample_line(&team, &season_id, 1, 1, 0, 0, 3, 0),
            premier,
        )])
        .await
        .unwrap();
        // Journée 2 : seconde victoire — cumul à 2 matchs, 6 points.
        repo.insert_lines(&[for_match(
            sample_line(&team, &season_id, 2, 2, 0, 0, 6, 0),
            second,
        )])
        .await
        .unwrap();

        repo.delete_lines_for_match(&second.to_string())
            .await
            .unwrap();

        let latest = repo
            .find_latest_line(&season_id.to_string(), &team.to_string())
            .await
            .unwrap()
            .expect("la ligne de la journée 1 doit redevenir la dernière");
        assert_eq!(latest.matches_played, 1);
        assert_eq!(latest.wins, 1);
        assert_eq!(latest.ranking_points, 3);
    }

    /// Règle 11 : la compensation doit pouvoir être rejouée.
    #[sqlx::test]
    async fn un_second_appel_ne_supprime_rien_et_n_echoue_pas(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (season_id, team) = (SeasonId::new(), TeamId::new());
        let mr = MatchReportId::new();
        repo.insert_lines(&[for_match(
            sample_line(&team, &season_id, 1, 1, 0, 0, 3, 0),
            mr,
        )])
        .await
        .unwrap();

        repo.delete_lines_for_match(&mr.to_string()).await.unwrap();
        repo.delete_lines_for_match(&mr.to_string()).await.unwrap();

        assert!(repo
            .find_latest_line(&season_id.to_string(), &team.to_string())
            .await
            .unwrap()
            .is_none());
    }

    #[sqlx::test]
    async fn les_lignes_d_un_autre_match_ne_sont_pas_touchees(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (season_id, team) = (SeasonId::new(), TeamId::new());
        let (garde, cible) = (MatchReportId::new(), MatchReportId::new());
        repo.insert_lines(&[for_match(
            sample_line(&team, &season_id, 1, 1, 0, 0, 3, 0),
            garde,
        )])
        .await
        .unwrap();
        repo.insert_lines(&[for_match(
            sample_line(&team, &season_id, 2, 2, 0, 0, 6, 0),
            cible,
        )])
        .await
        .unwrap();

        repo.delete_lines_for_match(&cible.to_string())
            .await
            .unwrap();

        let latest = repo
            .find_latest_line(&season_id.to_string(), &team.to_string())
            .await
            .unwrap();
        assert!(latest.is_some(), "la ligne de l'autre match doit subsister");
    }

    /// L'index unique posé par cette carte : sans suppression préalable, un
    /// rejeu doublerait les points de l'équipe. Il échoue désormais bruyamment.
    #[sqlx::test]
    async fn reinserer_le_meme_match_sans_supprimer_est_rejete(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (season_id, team) = (SeasonId::new(), TeamId::new());
        let mr = MatchReportId::new();
        let line = || for_match(sample_line(&team, &season_id, 1, 1, 0, 0, 3, 0), mr);

        repo.insert_lines(&[line()]).await.unwrap();
        assert!(
            repo.insert_lines(&[line()]).await.is_err(),
            "l'index unique doit refuser un doublon (match_report_id, team_id)"
        );
    }

    /// Le cycle complet : dépublier puis republier laisse une seule paire de
    /// lignes, pas deux.
    #[sqlx::test]
    async fn depublier_puis_republier_ne_laisse_qu_une_paire(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (season_id, home, away) = (SeasonId::new(), TeamId::new(), TeamId::new());
        let mr = MatchReportId::new();
        let pair = || {
            vec![
                for_match(sample_line(&home, &season_id, 1, 1, 0, 0, 3, 0), mr),
                for_match(sample_line(&away, &season_id, 1, 0, 0, 1, 0, 0), mr),
            ]
        };

        repo.insert_lines(&pair()).await.unwrap();
        repo.delete_lines_for_match(&mr.to_string()).await.unwrap();
        repo.insert_lines(&pair()).await.unwrap();

        let lines = repo
            .find_latest_lines_for_season(&season_id.to_string())
            .await
            .unwrap();
        assert_eq!(lines.len(), 2, "une ligne par équipe, pas quatre");
    }

    // ── Rejeu d'une saison (carte 418) ──────────────────────────────────────

    /// L'ordre de lecture est celui de `sequence`, **pas** de `recorded_at`.
    ///
    /// Le dépôt teste déjà pourquoi ailleurs : une ligne peut porter un
    /// horodatage antérieur à celle qui la précède, et c'est l'ordre
    /// d'enregistrement qui fait foi. Rejouer dans l'ordre des horodatages
    /// lirait ces lignes à l'envers, et la différence de deux cumuls rendrait un
    /// écart négatif — une erreur, sur des données pourtant saines.
    #[sqlx::test]
    async fn find_all_lines_for_season_suit_la_sequence_et_non_l_horodatage(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (team_id, season_id) = (TeamId::new(), SeasonId::new());

        let mut premiere = sample_line(&team_id, &season_id, 1, 1, 0, 0, 3, 0);
        premiere.recorded_at = Utc::now() + chrono::Duration::hours(1); // "futur"
        repo.insert_lines(&[premiere]).await.unwrap();

        let mut seconde = sample_line(&team_id, &season_id, 2, 1, 0, 1, 3, 0);
        seconde.recorded_at = Utc::now() - chrono::Duration::hours(1); // "passé"
        repo.insert_lines(&[seconde]).await.unwrap();

        let lues = repo
            .find_all_lines_for_season(&season_id.to_string())
            .await
            .unwrap();

        assert_eq!(lues.len(), 2);
        assert_eq!(
            (lues[0].matches_played, lues[1].matches_played),
            (1, 2),
            "l'ordre suit sequence, pas recorded_at"
        );
    }

    #[sqlx::test]
    async fn replace_lines_for_season_remplace_tout(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let (team_id, season_id) = (TeamId::new(), SeasonId::new());
        repo.insert_lines(&[
            sample_line(&team_id, &season_id, 1, 1, 0, 0, 3, 0),
            sample_line(&team_id, &season_id, 2, 2, 0, 0, 6, 0),
        ])
        .await
        .unwrap();

        let neuve = sample_line(&team_id, &season_id, 1, 1, 0, 0, 5, 0);
        repo.replace_lines_for_season(&season_id.to_string(), &[neuve])
            .await
            .unwrap();

        let lues = repo
            .find_all_lines_for_season(&season_id.to_string())
            .await
            .unwrap();
        assert_eq!(lues.len(), 1, "les deux anciennes ont disparu");
        assert_eq!(lues[0].ranking_points, 5);
    }

    /// **Un échec en cours ne laisse pas de saison à moitié rejouée.**
    ///
    /// L'échec est provoqué par un déclencheur, et non par un doublon
    /// d'identifiant : les `id` sont engendrés dans `insert_line_in_tx`, aucun
    /// appelant ne peut en imposer un. La base éphémère de `sqlx::test` rend le
    /// déclencheur sans effet sur le reste de la suite.
    ///
    /// Ce que le test refuserait sans la transaction unique : un `delete` suivi
    /// d'un `insert` laisserait ici la saison **sans aucune ligne**, le premier
    /// ayant réussi et le second échoué.
    #[sqlx::test]
    async fn replace_lines_for_season_est_atomique(pool: PgPool) {
        let repo = PgRankingRepository::new(pool.clone());
        let (team_id, season_id) = (TeamId::new(), SeasonId::new());
        repo.insert_lines(&[sample_line(&team_id, &season_id, 1, 1, 0, 0, 3, 0)])
            .await
            .unwrap();

        // Le déclencheur refuse toute ligne à 42 points — la deuxième des trois.
        // Deux requêtes séparées : une instruction préparée n'en accepte qu'une.
        sqlx::query(
            r#"CREATE FUNCTION refuser_42() RETURNS trigger AS $$
               BEGIN
                 IF NEW.ranking_points = 42 THEN
                   RAISE EXCEPTION 'refus de test';
                 END IF;
                 RETURN NEW;
               END $$ LANGUAGE plpgsql"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"CREATE TRIGGER t_refuser_42 BEFORE INSERT ON ranking_lines
                 FOR EACH ROW EXECUTE FUNCTION refuser_42()"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let issue = repo
            .replace_lines_for_season(
                &season_id.to_string(),
                &[
                    sample_line(&team_id, &season_id, 1, 1, 0, 0, 5, 0),
                    sample_line(&team_id, &season_id, 2, 1, 0, 1, 42, 0),
                    sample_line(&team_id, &season_id, 3, 2, 0, 1, 8, 0),
                ],
            )
            .await;

        assert!(issue.is_err(), "l'insertion devait échouer");
        let lues = repo
            .find_all_lines_for_season(&season_id.to_string())
            .await
            .unwrap();
        assert_eq!(lues.len(), 1, "l'ancienne ligne est toujours là : {lues:?}");
        assert_eq!(
            lues[0].ranking_points, 3,
            "c'est bien l'ancienne, pas une des nouvelles"
        );
    }

    // ── Points manuels (carte 450) ────────────────────────────────────────────

    async fn attribuer(
        repo: &PgRankingRepository,
        season: &SeasonId,
        team: &TeamId,
        points: i32,
        motif: &str,
    ) {
        repo.insert_manual_points(
            &season.to_string(),
            &team.to_string(),
            points,
            motif,
            "DevCoach",
        )
        .await
        .unwrap();
    }

    #[sqlx::test]
    async fn insert_puis_totals_somme_les_lignes(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season = SeasonId::new();
        let equipe = TeamId::new();
        let autre = TeamId::new();

        attribuer(&repo, &season, &equipe, 3, "forfait adverse").await;
        attribuer(&repo, &season, &equipe, -1, "retard de feuille").await;
        attribuer(&repo, &season, &autre, 2, "rattrapage").await;

        let totaux = repo
            .find_manual_totals_for_season(&season.to_string())
            .await
            .unwrap();

        assert_eq!(totaux.get(&equipe.to_string()), Some(&2), "3 - 1");
        assert_eq!(totaux.get(&autre.to_string()), Some(&2));
        assert_eq!(totaux.len(), 2);
    }

    /// Une équipe sans ligne est **absente** de la carte, elle n'y figure pas à
    /// zéro. C'est ce qui rend le `unwrap_or(0)` du service légitime plutôt que
    /// défensif.
    #[sqlx::test]
    async fn une_equipe_sans_ligne_est_absente_des_totaux(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season = SeasonId::new();
        let sans_ligne = TeamId::new();

        attribuer(&repo, &season, &TeamId::new(), 3, "forfait").await;

        let totaux = repo
            .find_manual_totals_for_season(&season.to_string())
            .await
            .unwrap();

        assert!(!totaux.contains_key(&sans_ligne.to_string()));
    }

    #[sqlx::test]
    async fn list_rend_les_lignes_ordonnees(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season = SeasonId::new();
        // Deux équipes dont l'ordre alphabétique des identifiants est connu :
        // sans cela le test affirmerait un tri qu'il ne contrôle pas.
        let (mut a, mut b) = (TeamId::new(), TeamId::new());
        if a.to_string() > b.to_string() {
            std::mem::swap(&mut a, &mut b);
        }

        attribuer(&repo, &season, &b, 1, "seconde équipe").await;
        attribuer(&repo, &season, &a, 2, "première ligne").await;
        attribuer(&repo, &season, &a, 3, "seconde ligne").await;

        let lignes = repo.list_manual_points(&season.to_string()).await.unwrap();

        let ordre: Vec<(&str, i32)> = lignes
            .iter()
            .map(|l| (l.team_id.as_str(), l.points))
            .collect();
        assert_eq!(
            ordre,
            vec![
                (a.to_string().as_str(), 2),
                (a.to_string().as_str(), 3),
                (b.to_string().as_str(), 1)
            ],
            "équipe puis date"
        );
        assert_eq!(lignes[0].reason.as_deref(), Some("première ligne"));
        assert_eq!(lignes[0].awarded_by, "DevCoach");
    }

    /// **Le test du `AND season_id`.**
    ///
    /// `space_scope` ne résout pas `{point_id}` : sans la saison au `WHERE`, un
    /// identifiant deviné supprimerait la ligne d'une autre compétition. Le
    /// contrôle vit dans la requête plutôt que dans le use case, parce qu'un
    /// contrôle applicatif s'écrit puis s'oublie.
    #[sqlx::test]
    async fn delete_d_une_autre_saison_ne_supprime_rien(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let sienne = SeasonId::new();
        let etrangere = SeasonId::new();
        attribuer(&repo, &sienne, &TeamId::new(), 3, "sanction").await;
        let id = repo.list_manual_points(&sienne.to_string()).await.unwrap()[0].id;

        let supprimees = repo
            .delete_manual_points(id, &etrangere.to_string())
            .await
            .unwrap();

        assert_eq!(supprimees, 0);
        assert_eq!(
            repo.list_manual_points(&sienne.to_string())
                .await
                .unwrap()
                .len(),
            1,
            "la ligne d'origine doit être intacte"
        );
    }

    #[sqlx::test]
    async fn delete_deux_fois_rend_zero_la_seconde(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season = SeasonId::new();
        attribuer(&repo, &season, &TeamId::new(), 3, "sanction").await;
        let id = repo.list_manual_points(&season.to_string()).await.unwrap()[0].id;

        assert_eq!(
            repo.delete_manual_points(id, &season.to_string())
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            repo.delete_manual_points(id, &season.to_string())
                .await
                .unwrap(),
            0,
            "idempotent : rien à supprimer n'est pas une erreur"
        );
    }

    /// **Le cas passant qu'on croirait interdit.** Deux fois trois points à la
    /// même équipe, ce sont deux décisions et deux motifs — pas un doublon.
    #[sqlx::test]
    async fn deux_lignes_identiques_sont_acceptees(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season = SeasonId::new();
        let equipe = TeamId::new();

        attribuer(&repo, &season, &equipe, 3, "forfait au tour 2").await;
        attribuer(&repo, &season, &equipe, 3, "forfait au tour 5").await;

        let totaux = repo
            .find_manual_totals_for_season(&season.to_string())
            .await
            .unwrap();
        assert_eq!(totaux.get(&equipe.to_string()), Some(&6));
        assert_eq!(
            repo.list_manual_points(&season.to_string())
                .await
                .unwrap()
                .len(),
            2
        );
    }

    /// Les lignes d'une autre saison n'entrent dans aucune des deux lectures.
    #[sqlx::test]
    async fn les_lectures_sont_cloisonnees_par_saison(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let sienne = SeasonId::new();
        let voisine = SeasonId::new();
        let equipe = TeamId::new();

        attribuer(&repo, &sienne, &equipe, 3, "chez elle").await;
        attribuer(&repo, &voisine, &equipe, 50, "chez la voisine").await;

        let totaux = repo
            .find_manual_totals_for_season(&sienne.to_string())
            .await
            .unwrap();
        assert_eq!(
            totaux.get(&equipe.to_string()),
            Some(&3),
            "50 ne doit pas fuir"
        );
        assert_eq!(
            repo.list_manual_points(&sienne.to_string())
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
