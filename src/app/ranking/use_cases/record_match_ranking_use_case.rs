use crate::app::ranking::domain::ranking_line::{
    AggressiveBonusRule, BonusActivated, CasualtiesInflicted, CasualtiesTotal, CompletionsMade,
    CumulativeTotals, DefensiveBonusRule, DrawCount, FoulsCommitted, LossCount, MatchContext,
    MatchScore, MatchStats, MatchesPlayed, MaxTdConceded, MinCasualties, MinTd, OffensiveBonusRule,
    RankingLine, RankingPoints, RankingRules, TdAgainst, TdFor, WinCount,
};
use crate::app::ranking::ports::{IRankingCompetitionPort, IRankingRepository, RankingLineRow};
use crate::app::shared_kernel::common_types::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::team::TeamId;
use chrono::{DateTime, Utc};

/// Stats d'une équipe sur un match. Regroupées pour que le croisement
/// home/away se lise d'un coup d'œil : une inversion entre deux champs à
/// préfixe (`home_score` utilisé pour l'équipe away) compile sans broncher.
pub struct TeamMatchStats {
    pub score: MatchScore,
    pub casualties: CasualtiesInflicted,
    pub fouls: FoulsCommitted,
    pub completions: CompletionsMade,
}

pub struct RecordMatchRankingCommand {
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub match_report_id: MatchReportId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub home: TeamMatchStats,
    pub away: TeamMatchStats,
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

    let home_line = record_home(&cmd, home_previous, &rules);
    let away_line = record_away(&cmd, away_previous, &rules);

    repo.insert_lines(&[home_line, away_line])
        .await
        .map_err(|e| RecordMatchRankingError::Repository(e.to_string()))
}

fn record_home(
    cmd: &RecordMatchRankingCommand,
    previous: Option<CumulativeTotals>,
    rules: &RankingRules,
) -> RankingLine {
    let ctx = context_for(cmd, cmd.home_team_id.clone());
    // Les TD se croisent entre les deux équipes ; les sorties non — elles sont
    // propres à l'équipe qui les a infligées.
    let stats = MatchStats {
        own_td: cmd.home.score,
        opponent_td: cmd.away.score,
        casualties_inflicted: cmd.home.casualties,
        fouls: cmd.home.fouls,
        completions: cmd.home.completions,
    };
    RankingLine::record_match(previous, ctx, stats, rules)
}

fn record_away(
    cmd: &RecordMatchRankingCommand,
    previous: Option<CumulativeTotals>,
    rules: &RankingRules,
) -> RankingLine {
    let ctx = context_for(cmd, cmd.away_team_id.clone());
    let stats = MatchStats {
        own_td: cmd.away.score,
        opponent_td: cmd.home.score,
        casualties_inflicted: cmd.away.casualties,
        fouls: cmd.away.fouls,
        completions: cmd.away.completions,
    };
    RankingLine::record_match(previous, ctx, stats, rules)
}

fn context_for(cmd: &RecordMatchRankingCommand, team_id: TeamId) -> MatchContext {
    MatchContext {
        team_id,
        competition_id: cmd.competition_id.clone(),
        season_id: cmd.season_id.clone(),
        round_id: cmd.round_id.clone(),
        match_report_id: cmd.match_report_id.clone(),
        recorded_at: cmd.published_at,
    }
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
        // Sans ce report, le cumul des bonus repartirait de zéro à chaque match.
        bonus_points: RankingPoints(row.bonus_points),
        td_for: TdFor(row.td_for),
        td_against: TdAgainst(row.td_against),
        casualties: CasualtiesTotal(row.casualties),
        fouls: FoulsCommitted(row.fouls),
        completions: CompletionsMade(row.completions),
    }
}

fn to_domain_rules(info: crate::app::ranking::ports::RankingRulesInfo) -> RankingRules {
    RankingRules {
        win_points: RankingPoints(info.win_points),
        draw_points: RankingPoints(info.draw_points),
        lose_points: RankingPoints(info.lose_points),
        offensive_bonus: OffensiveBonusRule {
            activated: BonusActivated(info.offensive.activated),
            min_td: MinTd(info.offensive.threshold),
            points: RankingPoints(info.offensive.points),
        },
        defensive_bonus: DefensiveBonusRule {
            activated: BonusActivated(info.defensive.activated),
            max_td_conceded: MaxTdConceded(info.defensive.threshold),
            points: RankingPoints(info.defensive.points),
        },
        aggressive_bonus: AggressiveBonusRule {
            activated: BonusActivated(info.aggressive.activated),
            min_casualties: MinCasualties(info.aggressive.threshold),
            points: RankingPoints(info.aggressive.points),
        },
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
            // Aucun départage configuré : le calcul des points n'en dépend pas.
            tiebreakers: vec![],
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
                bonus_points: l.bonus_points.0,
                td_for: l.td_for.0,
                td_against: l.td_against.0,
                casualties: l.casualties.0,
                fouls: l.fouls.0,
                completions: l.completions.0,
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

    fn team_stats(score: u8, casualties: u32, fouls: u32, completions: u32) -> TeamMatchStats {
        TeamMatchStats {
            score: MatchScore(score),
            casualties: CasualtiesInflicted(casualties),
            fouls: FoulsCommitted(fouls),
            completions: CompletionsMade(completions),
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
            home: team_stats(2, 0, 0, 0),
            away: team_stats(1, 0, 0, 0),
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

    #[tokio::test]
    async fn activated_bonus_is_added_to_the_team_points() {
        let repo = FakeRepo::default();
        let mut info = rules_info(3, 1, 0);
        info.aggressive = BonusRuleInfo { activated: true, threshold: 1, points: 2 };
        let port = FakeCompetitionPort { rules: Some(info) };
        let home = TeamId::new();
        let away = TeamId::new();
        let mut cmd = sample_cmd(home.clone(), away.clone());
        cmd.home.casualties = CasualtiesInflicted(3); // > 1 → bonus agressif

        execute(cmd, &repo, &port).await.unwrap();

        let lines = repo.lines.lock().unwrap();
        let home_line = lines.iter().find(|l| l.team_id == home).unwrap();
        let away_line = lines.iter().find(|l| l.team_id == away).unwrap();
        assert_eq!(home_line.ranking_points.0, 5); // victoire 2-1 (3) + bonus (2)
        assert_eq!(away_line.ranking_points.0, 0); // défaite, aucune sortie
    }

    /// Verrou du croisement `home`/`away` : les TD s'échangent entre les deux
    /// équipes, les sorties **non** — elles restent celles de l'équipe qui les a
    /// infligées. Une inversion compile sans broncher et produit des lignes
    /// plausibles ; ce test est le seul filet.
    #[tokio::test]
    async fn team_stats_cross_over_for_touchdowns_but_not_for_casualties() {
        let repo = FakeRepo::default();
        // Bonus agressif à seuil 2 : seule une équipe à 3 sorties le décroche.
        let mut info = rules_info(3, 1, 0);
        info.aggressive = BonusRuleInfo { activated: true, threshold: 2, points: 7 };
        let port = FakeCompetitionPort { rules: Some(info) };

        let home = TeamId::new();
        let away = TeamId::new();
        let mut cmd = sample_cmd(home.clone(), away.clone()); // 2-1 pour home
        cmd.home.casualties = CasualtiesInflicted(3); // > 2 → bonus pour home
        cmd.away.casualties = CasualtiesInflicted(0); // aucun bonus pour away

        execute(cmd, &repo, &port).await.unwrap();

        let lines = repo.lines.lock().unwrap();
        let home_line = lines.iter().find(|l| l.team_id == home).unwrap();
        let away_line = lines.iter().find(|l| l.team_id == away).unwrap();

        // Les TD se croisent : home gagne, away perd.
        assert_eq!(home_line.wins.0, 1);
        assert_eq!(away_line.losses.0, 1);
        // Les sorties ne se croisent pas : le bonus va à home seule. Inversées,
        // les 7 points atterriraient sur away.
        assert_eq!(home_line.bonus_points.0, 7);
        assert_eq!(away_line.bonus_points.0, 0);
    }

    /// Même verrou pour les cinq compteurs de départage : seuls les TD se croisent.
    /// Une inversion de `fouls` / `completions` / `casualties` compile sans broncher
    /// et produit des compteurs plausibles mais attribués à la mauvaise équipe.
    #[tokio::test]
    async fn tiebreak_counters_cross_over_only_for_touchdowns() {
        let repo = FakeRepo::default();
        let port = FakeCompetitionPort { rules: Some(rules_info(3, 1, 0)) };
        let home = TeamId::new();
        let away = TeamId::new();
        let mut cmd = sample_cmd(home.clone(), away.clone());
        // Match 2-1 pour home, avec des valeurs toutes distinctes des deux côtés.
        cmd.home = team_stats(2, 3, 1, 5);
        cmd.away = team_stats(1, 0, 4, 2);

        execute(cmd, &repo, &port).await.unwrap();

        let lines = repo.lines.lock().unwrap();
        let home_line = lines.iter().find(|l| l.team_id == home).unwrap();
        let away_line = lines.iter().find(|l| l.team_id == away).unwrap();

        // Les TD se croisent : les 2 de home sont les `td_against` de away.
        assert_eq!((home_line.td_for.0, home_line.td_against.0), (2, 1));
        assert_eq!((away_line.td_for.0, away_line.td_against.0), (1, 2));
        // Les trois autres restent ceux de l'équipe qui les a produits.
        assert_eq!((home_line.casualties.0, home_line.fouls.0, home_line.completions.0), (3, 1, 5));
        assert_eq!((away_line.casualties.0, away_line.fouls.0, away_line.completions.0), (0, 4, 2));
    }
}
