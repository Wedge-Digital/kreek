use crate::app::ranking::domain::ranking_line::{
    CumulativeTotals, DrawCount, LossCount, MatchScore, MatchesPlayed, RankingLine, RankingPoints,
    RankingRules, WinCount,
};
use crate::app::ranking::ports::{IRankingCompetitionPort, IRankingRepository, RankingLineRow};
use crate::app::shared_kernel::common_types::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::team::TeamId;
use chrono::{DateTime, Utc};

pub struct RecordMatchRankingCommand {
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub match_report_id: MatchReportId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub home_score: MatchScore,
    pub away_score: MatchScore,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum RecordMatchRankingError {
    RulesNotConfigured,
    Repository(String),
}

pub async fn execute(
    cmd: RecordMatchRankingCommand,
    repo: &dyn IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) -> Result<(), RecordMatchRankingError> {
    let season_id_str = cmd.season_id.to_string();

    let rules = competition_port
        .find_ranking_rules(&season_id_str)
        .await
        .map(to_domain_rules)
        .ok_or(RecordMatchRankingError::RulesNotConfigured)?;

    let home_previous = load_previous(repo, &season_id_str, &cmd.home_team_id.to_string()).await?;
    let away_previous = load_previous(repo, &season_id_str, &cmd.away_team_id.to_string()).await?;

    let home_outcome = RankingLine::derive_outcome(cmd.home_score, cmd.away_score);
    let away_outcome = RankingLine::derive_outcome(cmd.away_score, cmd.home_score);

    let home_line = RankingLine::record_match(
        home_previous, cmd.home_team_id, cmd.competition_id.clone(), cmd.season_id.clone(),
        cmd.round_id.clone(), cmd.match_report_id.clone(), cmd.published_at, home_outcome, &rules,
    );
    let away_line = RankingLine::record_match(
        away_previous, cmd.away_team_id, cmd.competition_id, cmd.season_id,
        cmd.round_id, cmd.match_report_id, cmd.published_at, away_outcome, &rules,
    );

    repo.insert_lines(&[home_line, away_line])
        .await
        .map_err(|e| RecordMatchRankingError::Repository(e.to_string()))
}

async fn load_previous(
    repo: &dyn IRankingRepository,
    season_id: &str,
    team_id: &str,
) -> Result<Option<CumulativeTotals>, RecordMatchRankingError> {
    let row = repo
        .find_latest_line(season_id, team_id)
        .await
        .map_err(|e| RecordMatchRankingError::Repository(e.to_string()))?;
    Ok(row.map(to_totals))
}

fn to_totals(row: RankingLineRow) -> CumulativeTotals {
    CumulativeTotals {
        matches_played: MatchesPlayed(row.matches_played),
        wins: WinCount(row.wins),
        draws: DrawCount(row.draws),
        losses: LossCount(row.losses),
        ranking_points: RankingPoints(row.ranking_points),
    }
}

fn to_domain_rules(info: crate::app::ranking::ports::RankingRulesInfo) -> RankingRules {
    RankingRules {
        win_points: RankingPoints(info.win_points),
        draw_points: RankingPoints(info.draw_points),
        lose_points: RankingPoints(info.lose_points),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::ports::{
        BonusRuleInfo, EnrolledTeamInfo, RankingRepositoryError, RankingRulesInfo,
    };
    use async_trait::async_trait;
    use std::sync::Mutex;

    fn no_bonus() -> BonusRuleInfo {
        BonusRuleInfo { activated: false, threshold: 0, points: 0 }
    }

    fn rules_info(win_points: u32, draw_points: u32, lose_points: u32) -> RankingRulesInfo {
        RankingRulesInfo {
            win_points,
            draw_points,
            lose_points,
            offensive: no_bonus(),
            defensive: no_bonus(),
            aggressive: no_bonus(),
        }
    }

    struct FakeCompetitionPort {
        rules: Option<RankingRulesInfo>,
    }

    #[async_trait]
    impl IRankingCompetitionPort for FakeCompetitionPort {
        async fn find_ranking_rules(&self, _: &str) -> Option<RankingRulesInfo> {
            self.rules.clone()
        }
        async fn find_enrolled_teams(&self, _: &str) -> Vec<EnrolledTeamInfo> {
            vec![]
        }
        async fn find_groups(&self, _: &str) -> Vec<crate::app::ranking::ports::RankingGroupInfo> {
            vec![]
        }
    }

    #[derive(Default)]
    struct FakeRepo {
        lines: Mutex<Vec<RankingLine>>,
    }

    #[async_trait]
    impl IRankingRepository for FakeRepo {
        async fn find_latest_line(&self, _: &str, team_id: &str) -> Result<Option<RankingLineRow>, RankingRepositoryError> {
            let lines = self.lines.lock().unwrap();
            Ok(lines.iter().rev().find(|l| l.team_id.to_string() == team_id).map(|l| RankingLineRow {
                team_id: l.team_id.to_string(),
                matches_played: l.matches_played.0,
                wins: l.wins.0,
                draws: l.draws.0,
                losses: l.losses.0,
                ranking_points: l.ranking_points.0,
            }))
        }
        async fn find_latest_lines_for_season(&self, _: &str) -> Result<Vec<RankingLineRow>, RankingRepositoryError> {
            Ok(vec![])
        }
        async fn insert_lines(&self, new_lines: &[RankingLine]) -> Result<(), RankingRepositoryError> {
            self.lines.lock().unwrap().extend_from_slice(new_lines);
            Ok(())
        }
    }

    fn sample_cmd(home_team_id: TeamId, away_team_id: TeamId) -> RecordMatchRankingCommand {
        RecordMatchRankingCommand {
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            match_report_id: MatchReportId::new(),
            home_team_id,
            away_team_id,
            home_score: MatchScore(2),
            away_score: MatchScore(1),
            published_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn returns_error_and_writes_nothing_when_rules_not_configured() {
        let repo = FakeRepo::default();
        let port = FakeCompetitionPort { rules: None };
        let cmd = sample_cmd(TeamId::new(), TeamId::new());

        let result = execute(cmd, &repo, &port).await;

        assert!(matches!(result, Err(RecordMatchRankingError::RulesNotConfigured)));
        assert!(repo.lines.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn records_both_lines_for_a_team_without_history() {
        let repo = FakeRepo::default();
        let port = FakeCompetitionPort {
            rules: Some(rules_info(3, 1, 0)),
        };
        let home = TeamId::new();
        let away = TeamId::new();
        let cmd = sample_cmd(home.clone(), away.clone());

        execute(cmd, &repo, &port).await.unwrap();

        let lines = repo.lines.lock().unwrap();
        assert_eq!(lines.len(), 2);
        let home_line = lines.iter().find(|l| l.team_id == home).unwrap();
        let away_line = lines.iter().find(|l| l.team_id == away).unwrap();
        assert_eq!(home_line.ranking_points.0, 3); // victoire 2-1
        assert_eq!(away_line.ranking_points.0, 0); // défaite
        assert_eq!(home_line.matches_played.0, 1);
    }

    #[tokio::test]
    async fn accumulates_on_top_of_existing_history() {
        let repo = FakeRepo::default();
        let port = FakeCompetitionPort {
            rules: Some(rules_info(3, 1, 0)),
        };
        let home = TeamId::new();
        let away = TeamId::new();

        execute(sample_cmd(home.clone(), away.clone()), &repo, &port).await.unwrap();
        execute(sample_cmd(home.clone(), away.clone()), &repo, &port).await.unwrap();

        let lines = repo.lines.lock().unwrap();
        let home_lines: Vec<_> = lines.iter().filter(|l| l.team_id == home).collect();
        let last_home_line = home_lines.last().unwrap();
        assert_eq!(last_home_line.matches_played.0, 2);
        assert_eq!(last_home_line.ranking_points.0, 6); // 2 victoires
        assert_eq!(lines.len(), 4); // 2 matchs x 2 équipes, jamais écrasées
    }
}
