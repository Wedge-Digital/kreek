use crate::app::shared_kernel::common_types::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::team::TeamId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Score brut d'une équipe pour un match — aucun invariant métier connu
/// (toute valeur est un score valide), simple newtype pour sortir du régime
/// primitif nu (règle CQRS du CLAUDE.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchScore(pub u8);

/// Points de classement — cumul non borné, aucun invariant à valider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankingPoints(pub u32);

impl std::ops::Add for RankingPoints {
    type Output = RankingPoints;
    fn add(self, rhs: RankingPoints) -> RankingPoints {
        RankingPoints(self.0 + rhs.0)
    }
}

/// Compteurs cumulés d'une ligne de classement — newtypes sans invariant
/// (sortent du régime primitif nu, règle CQRS), un type par compteur pour
/// éviter toute confusion entre eux (même style que `players::TouchdownCount`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchesPlayed(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WinCount(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DrawCount(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LossCount(pub u32);

/// Compteurs cumulés d'une équipe avant un nouveau match — c'est tout ce dont
/// `record_match` a besoin de la ligne précédente, pas la `RankingLine`
/// complète (id, dates, etc. non pertinents pour le calcul).
#[derive(Debug, Clone, Copy)]
pub struct CumulativeTotals {
    pub matches_played: MatchesPlayed,
    pub wins: WinCount,
    pub draws: DrawCount,
    pub losses: LossCount,
    pub ranking_points: RankingPoints,
}

impl CumulativeTotals {
    pub const ZERO: CumulativeTotals = CumulativeTotals {
        matches_played: MatchesPlayed(0),
        wins: WinCount(0),
        draws: DrawCount(0),
        losses: LossCount(0),
        ranking_points: RankingPoints(0),
    };
}

/// Barème de points de classement d'une compétition — copie en lecture des
/// règles consultées via `IRankingCompetitionPort` (carte 193), jamais le
/// type domaine `competitions::RankingRules`.
#[derive(Debug, Clone, Copy)]
pub struct RankingRules {
    pub win_points: RankingPoints,
    pub draw_points: RankingPoints,
    pub lose_points: RankingPoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOutcome {
    Win,
    Draw,
    Loss,
}

/// Ligne de classement — fait immuable : une fois enregistrée, jamais modifiée.
/// Contient les compteurs **cumulés** depuis le début de la saison, pas
/// seulement les points — c'est toujours la ligne la plus récente d'une
/// équipe (par ordre d'enregistrement, cf. `sequence` en base) qui fait foi
/// pour l'affichage, jamais une agrégation de plusieurs lignes à la lecture.
#[derive(Debug, Clone)]
pub struct RankingLine {
    pub team_id: TeamId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub match_report_id: MatchReportId,
    pub recorded_at: DateTime<Utc>,
    pub matches_played: MatchesPlayed,
    pub wins: WinCount,
    pub draws: DrawCount,
    pub losses: LossCount,
    pub ranking_points: RankingPoints,
}

impl RankingLine {
    /// Dérive le résultat d'une équipe à partir des deux scores — total,
    /// jamais d'erreur (toute paire de scores produit un résultat valide).
    pub fn derive_outcome(own_score: MatchScore, opponent_score: MatchScore) -> MatchOutcome {
        match own_score.0.cmp(&opponent_score.0) {
            Ordering::Greater => MatchOutcome::Win,
            Ordering::Equal => MatchOutcome::Draw,
            Ordering::Less => MatchOutcome::Loss,
        }
    }

    /// Construit la nouvelle ligne de classement d'une équipe après un match.
    /// `previous` : compteurs cumulés de la dernière ligne connue de cette
    /// équipe pour cette saison (`None` = première apparition dans le classement).
    #[allow(clippy::too_many_arguments)]
    pub fn record_match(
        previous: Option<CumulativeTotals>,
        team_id: TeamId,
        competition_id: CompetitionId,
        season_id: SeasonId,
        round_id: RoundId,
        match_report_id: MatchReportId,
        recorded_at: DateTime<Utc>,
        outcome: MatchOutcome,
        rules: &RankingRules,
    ) -> RankingLine {
        let CumulativeTotals { matches_played, wins, draws, losses, ranking_points: points } =
            previous.unwrap_or(CumulativeTotals::ZERO);

        let match_points = match outcome {
            MatchOutcome::Win => rules.win_points,
            MatchOutcome::Draw => rules.draw_points,
            MatchOutcome::Loss => rules.lose_points,
        };

        RankingLine {
            team_id,
            competition_id,
            season_id,
            round_id,
            match_report_id,
            recorded_at,
            matches_played: MatchesPlayed(matches_played.0 + 1),
            wins: WinCount(wins.0 + u32::from(outcome == MatchOutcome::Win)),
            draws: DrawCount(draws.0 + u32::from(outcome == MatchOutcome::Draw)),
            losses: LossCount(losses.0 + u32::from(outcome == MatchOutcome::Loss)),
            ranking_points: points + match_points,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> (TeamId, CompetitionId, SeasonId, RoundId, MatchReportId) {
        (TeamId::new(), CompetitionId::new(), SeasonId::new(), RoundId::new(), MatchReportId::new())
    }

    fn totals_of(line: &RankingLine) -> CumulativeTotals {
        CumulativeTotals {
            matches_played: line.matches_played,
            wins: line.wins,
            draws: line.draws,
            losses: line.losses,
            ranking_points: line.ranking_points,
        }
    }

    fn rules() -> RankingRules {
        RankingRules {
            win_points: RankingPoints(3),
            draw_points: RankingPoints(1),
            lose_points: RankingPoints(0),
        }
    }

    #[test]
    fn derive_outcome_higher_score_wins() {
        let outcome = RankingLine::derive_outcome(MatchScore(2), MatchScore(1));
        assert_eq!(outcome, MatchOutcome::Win);
    }

    #[test]
    fn derive_outcome_equal_score_is_draw() {
        let outcome = RankingLine::derive_outcome(MatchScore(1), MatchScore(1));
        assert_eq!(outcome, MatchOutcome::Draw);
    }

    #[test]
    fn derive_outcome_lower_score_loses() {
        let outcome = RankingLine::derive_outcome(MatchScore(0), MatchScore(2));
        assert_eq!(outcome, MatchOutcome::Loss);
    }

    #[test]
    fn record_match_home_and_away_outcomes_are_symmetric() {
        let (team_id, competition_id, season_id, round_id, match_report_id) = ids();
        let home_score = MatchScore(2);
        let away_score = MatchScore(1);

        let home_outcome = RankingLine::derive_outcome(home_score, away_score);
        let away_outcome = RankingLine::derive_outcome(away_score, home_score);

        assert_eq!(home_outcome, MatchOutcome::Win);
        assert_eq!(away_outcome, MatchOutcome::Loss);

        let home_line = RankingLine::record_match(
            None, team_id.clone(), competition_id.clone(), season_id.clone(), round_id.clone(), match_report_id.clone(),
            Utc::now(), home_outcome, &rules(),
        );
        let away_line = RankingLine::record_match(
            None, team_id, competition_id, season_id, round_id, match_report_id,
            Utc::now(), away_outcome, &rules(),
        );

        assert_eq!(home_line.wins.0, 1);
        assert_eq!(home_line.losses.0, 0);
        assert_eq!(away_line.wins.0, 0);
        assert_eq!(away_line.losses.0, 1);
    }

    #[test]
    fn record_match_without_previous_line_starts_from_zero() {
        let (team_id, competition_id, season_id, round_id, match_report_id) = ids();
        let line = RankingLine::record_match(
            None, team_id, competition_id, season_id, round_id, match_report_id,
            Utc::now(), MatchOutcome::Win, &rules(),
        );

        assert_eq!(line.matches_played.0, 1);
        assert_eq!(line.wins.0, 1);
        assert_eq!(line.draws.0, 0);
        assert_eq!(line.losses.0, 0);
        assert_eq!(line.ranking_points.0, 3);
    }

    #[test]
    fn record_match_with_previous_line_accumulates() {
        let (team_id, competition_id, season_id, round_id, match_report_id) = ids();
        let previous = RankingLine::record_match(
            None, team_id.clone(), competition_id.clone(), season_id.clone(), round_id.clone(), match_report_id.clone(),
            Utc::now(), MatchOutcome::Win, &rules(),
        );

        let next = RankingLine::record_match(
            Some(totals_of(&previous)), team_id, competition_id, season_id, round_id, match_report_id,
            Utc::now(), MatchOutcome::Draw, &rules(),
        );

        assert_eq!(next.matches_played.0, 2);
        assert_eq!(next.wins.0, 1);
        assert_eq!(next.draws.0, 1);
        assert_eq!(next.losses.0, 0);
        assert_eq!(next.ranking_points.0, 4); // 3 (victoire) + 1 (nul)
    }

    #[test]
    fn record_match_accumulates_over_three_successive_matches() {
        let (team_id, competition_id, season_id, round_id, match_report_id) = ids();
        let mut line: Option<RankingLine> = None;

        for outcome in [MatchOutcome::Win, MatchOutcome::Draw, MatchOutcome::Loss] {
            line = Some(RankingLine::record_match(
                line.as_ref().map(totals_of), team_id.clone(), competition_id.clone(), season_id.clone(), round_id.clone(),
                match_report_id.clone(), Utc::now(), outcome, &rules(),
            ));
        }

        let line = line.unwrap();
        assert_eq!(line.matches_played.0, 3);
        assert_eq!(line.wins.0, 1);
        assert_eq!(line.draws.0, 1);
        assert_eq!(line.losses.0, 1);
        assert_eq!(line.ranking_points.0, 4); // 3 + 1 + 0
    }

    #[test]
    fn record_match_applies_points_from_rules_per_outcome() {
        let (team_id, competition_id, season_id, round_id, match_report_id) = ids();
        let custom_rules = RankingRules {
            win_points: RankingPoints(5),
            draw_points: RankingPoints(2),
            lose_points: RankingPoints(1),
        };

        let win = RankingLine::record_match(
            None, team_id.clone(), competition_id.clone(), season_id.clone(), round_id.clone(), match_report_id.clone(),
            Utc::now(), MatchOutcome::Win, &custom_rules,
        );
        let draw = RankingLine::record_match(
            None, team_id.clone(), competition_id.clone(), season_id.clone(), round_id.clone(), match_report_id.clone(),
            Utc::now(), MatchOutcome::Draw, &custom_rules,
        );
        let loss = RankingLine::record_match(
            None, team_id, competition_id, season_id, round_id, match_report_id,
            Utc::now(), MatchOutcome::Loss, &custom_rules,
        );

        assert_eq!(win.ranking_points.0, 5);
        assert_eq!(draw.ranking_points.0, 2);
        assert_eq!(loss.ranking_points.0, 1);
    }
}
