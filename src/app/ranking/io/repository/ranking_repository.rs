use crate::app::ranking::domain::ranking_line::RankingLine;
use crate::app::ranking::ports::{IRankingRepository, RankingLineRow, RankingRepositoryError};
use crate::app::shared_kernel::sulid::SUlid;
use crate::app::shared_kernel::team::TeamId;
use async_trait::async_trait;
use sqlx::PgPool;

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

    async fn insert_lines(&self, lines: &[RankingLine]) -> Result<(), RankingRepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        for line in lines {
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
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::ranking_line::{
        CasualtiesTotal, CompletionsMade, DrawCount, FoulsCommitted, LossCount, MatchesPlayed,
        RankingPoints, TdAgainst, TdFor, WinCount,
    };
    use crate::app::shared_kernel::common_types::{CompetitionId, MatchReportId, RoundId, SeasonId};
    use crate::app::shared_kernel::team::TeamId;
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

        let home_row = repo.find_latest_line(&season_id.to_string(), &home.to_string()).await.unwrap();
        let away_row = repo.find_latest_line(&season_id.to_string(), &away.to_string()).await.unwrap();

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
        let latest = repo.find_latest_line(&season_id.to_string(), &team_id.to_string()).await.unwrap().unwrap();
        assert_eq!(latest.matches_played, 2);
        assert_eq!(latest.losses, 1);
    }

    #[sqlx::test]
    async fn find_latest_lines_for_season_returns_one_row_per_team(pool: PgPool) {
        let repo = PgRankingRepository::new(pool);
        let season_id = SeasonId::new();
        let team_a = TeamId::new();
        let team_b = TeamId::new();

        repo.insert_lines(&[sample_line(&team_a, &season_id, 1, 1, 0, 0, 3, 0)]).await.unwrap();
        repo.insert_lines(&[sample_line(&team_a, &season_id, 2, 1, 1, 0, 4, 0)]).await.unwrap();
        repo.insert_lines(&[sample_line(&team_b, &season_id, 1, 0, 0, 1, 0, 0)]).await.unwrap();

        let rows = repo.find_latest_lines_for_season(&season_id.to_string()).await.unwrap();

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

        let line = with_counters(sample_line(&team_id, &season_id, 1, 1, 0, 0, 3, 0), [7, 3, 5, 2, 9]);
        repo.insert_lines(&[line]).await.unwrap();

        let row = repo.find_latest_line(&season_id.to_string(), &team_id.to_string()).await.unwrap().unwrap();
        assert_eq!([row.td_for, row.td_against, row.casualties, row.fouls, row.completions], [7, 3, 5, 2, 9]);

        let rows = repo.find_latest_lines_for_season(&season_id.to_string()).await.unwrap();
        let row = rows.first().unwrap();
        assert_eq!([row.td_for, row.td_against, row.casualties, row.fouls, row.completions], [7, 3, 5, 2, 9]);
    }
}
