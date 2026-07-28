use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
use crate::app::shared_kernel::bloodbowl::team::TeamId;
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

/// Nombre de sorties (`Sortie` seule) infligées par une équipe sur un match —
/// compté côté IO (listener) à partir des actions, sans invariant à valider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CasualtiesInflicted(pub u32);

/// Seuils de déclenchement des bonus — newtypes sans invariant (les bornes de
/// validité vivent côté `competitions`, à la saisie ; ici ce sont des données
/// de configuration déjà validées, recopiées via le port).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinTd(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxTdConceded(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinCasualties(pub u32);

/// Drapeau d'activation d'un bonus pour la compétition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BonusActivated(pub bool);

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

/// Compteurs cumulés servant aux critères de départage. Types distincts de leurs
/// équivalents par match (`CasualtiesTotal` vs `CasualtiesInflicted`) : deux `u32`
/// nus se confondraient sans que le compilateur bronche, et prendre les sorties
/// d'un match pour celles de la saison donnerait un compteur plausible mais faux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TdFor(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TdAgainst(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CasualtiesTotal(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoulsCommitted(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionsMade(pub u32);

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
    /// Points bonus cumulés. **Déjà comptés dans `ranking_points`** — conservés à
    /// part pour que l'onglet « Classement détaillé » puisse détailler le total.
    pub bonus_points: RankingPoints,
    /// Compteurs de départage, accumulés pour tous les critères indépendamment de
    /// leur activation (règle 12). `diff_td` n'y figure pas : dérivé (règle 13).
    pub td_for: TdFor,
    pub td_against: TdAgainst,
    pub casualties: CasualtiesTotal,
    pub fouls: FoulsCommitted,
    pub completions: CompletionsMade,
}

impl CumulativeTotals {
    pub const ZERO: CumulativeTotals = CumulativeTotals {
        matches_played: MatchesPlayed(0),
        wins: WinCount(0),
        draws: DrawCount(0),
        losses: LossCount(0),
        ranking_points: RankingPoints(0),
        bonus_points: RankingPoints(0),
        td_for: TdFor(0),
        td_against: TdAgainst(0),
        casualties: CasualtiesTotal(0),
        fouls: FoulsCommitted(0),
        completions: CompletionsMade(0),
    };
}

/// Bonus offensif — `points` si l'équipe a marqué au moins `min_td` TD (≥).
#[derive(Debug, Clone, Copy)]
pub struct OffensiveBonusRule {
    pub activated: BonusActivated,
    pub min_td: MinTd,
    pub points: RankingPoints,
}

/// Bonus défensif — `points` si l'équipe a encaissé au plus `max_td_conceded` TD (≤).
#[derive(Debug, Clone, Copy)]
pub struct DefensiveBonusRule {
    pub activated: BonusActivated,
    pub max_td_conceded: MaxTdConceded,
    pub points: RankingPoints,
}

/// Bonus agressif — `points` si l'équipe a infligé strictement plus de
/// `min_casualties` sorties (>).
#[derive(Debug, Clone, Copy)]
pub struct AggressiveBonusRule {
    pub activated: BonusActivated,
    pub min_casualties: MinCasualties,
    pub points: RankingPoints,
}

impl OffensiveBonusRule {
    fn points_for(&self, stats: &MatchStats) -> RankingPoints {
        if self.activated.0 && u32::from(stats.own_td.0) >= self.min_td.0 {
            self.points
        } else {
            RankingPoints(0)
        }
    }
}

impl DefensiveBonusRule {
    fn points_for(&self, stats: &MatchStats) -> RankingPoints {
        if self.activated.0 && u32::from(stats.opponent_td.0) <= self.max_td_conceded.0 {
            self.points
        } else {
            RankingPoints(0)
        }
    }
}

impl AggressiveBonusRule {
    fn points_for(&self, stats: &MatchStats) -> RankingPoints {
        if self.activated.0 && stats.casualties_inflicted.0 > self.min_casualties.0 {
            self.points
        } else {
            RankingPoints(0)
        }
    }
}

/// Stats d'une équipe sur un match — entrée du calcul, remplace `outcome`
/// (dérivé en interne dans `record_match`, carte 206).
#[derive(Debug, Clone, Copy)]
pub struct MatchStats {
    pub own_td: MatchScore,
    pub opponent_td: MatchScore,
    pub casualties_inflicted: CasualtiesInflicted,
    /// Alimentent les compteurs cumulés, pas les bonus — aucun bonus ne les
    /// utilise aujourd'hui (règle 12 : on accumule tout).
    pub fouls: FoulsCommitted,
    pub completions: CompletionsMade,
}

/// Barème de points de classement d'une compétition — copie en lecture des
/// règles consultées via `IRankingCompetitionPort` (carte 193), jamais le
/// type domaine `competitions::RankingRules`.
#[derive(Debug, Clone, Copy)]
pub struct RankingRules {
    pub win_points: RankingPoints,
    pub draw_points: RankingPoints,
    pub lose_points: RankingPoints,
    pub offensive_bonus: OffensiveBonusRule,
    pub defensive_bonus: DefensiveBonusRule,
    pub aggressive_bonus: AggressiveBonusRule,
}

impl RankingRules {
    /// Points bonus cumulés d'une équipe sur un match — somme des 3 bonus
    /// (chacun 0 si désactivé ou condition non remplie).
    pub fn bonus_points(&self, stats: &MatchStats) -> RankingPoints {
        self.offensive_bonus.points_for(stats)
            + self.defensive_bonus.points_for(stats)
            + self.aggressive_bonus.points_for(stats)
    }
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
    /// Part bonus du total ci-dessus, cumulée depuis le début de la saison.
    pub bonus_points: RankingPoints,
    pub td_for: TdFor,
    pub td_against: TdAgainst,
    pub casualties: CasualtiesTotal,
    pub fouls: FoulsCommitted,
    pub completions: CompletionsMade,
}

/// Identité d'une ligne de classement à enregistrer — regroupe les champs
/// contextuels de `record_match` (l'équipe, la compétition, la saison, la
/// journée, le rapport, l'horodatage), séparés des données de calcul (`MatchStats`).
#[derive(Debug, Clone)]
pub struct MatchContext {
    pub team_id: TeamId,
    pub competition_id: CompetitionId,
    pub season_id: SeasonId,
    pub round_id: RoundId,
    pub match_report_id: MatchReportId,
    pub recorded_at: DateTime<Utc>,
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
    /// L'outcome est dérivé des scores de `stats` ; les points bonus s'ajoutent
    /// aux points de résultat.
    pub fn record_match(
        previous: Option<CumulativeTotals>,
        ctx: MatchContext,
        stats: MatchStats,
        rules: &RankingRules,
    ) -> RankingLine {
        let outcome = Self::derive_outcome(stats.own_td, stats.opponent_td);
        let CumulativeTotals {
            matches_played,
            wins,
            draws,
            losses,
            ranking_points: points,
            bonus_points: bonus_total,
            td_for,
            td_against,
            casualties,
            fouls,
            completions,
        } = previous.unwrap_or(CumulativeTotals::ZERO);

        let match_points = match outcome {
            MatchOutcome::Win => rules.win_points,
            MatchOutcome::Draw => rules.draw_points,
            MatchOutcome::Loss => rules.lose_points,
        };
        // Calculé une seule fois, utilisé deux fois : le total et sa part bonus ne
        // peuvent pas divergier.
        let bonus = rules.bonus_points(&stats);

        RankingLine {
            team_id: ctx.team_id,
            competition_id: ctx.competition_id,
            season_id: ctx.season_id,
            round_id: ctx.round_id,
            match_report_id: ctx.match_report_id,
            recorded_at: ctx.recorded_at,
            matches_played: MatchesPlayed(matches_played.0 + 1),
            wins: WinCount(wins.0 + u32::from(outcome == MatchOutcome::Win)),
            draws: DrawCount(draws.0 + u32::from(outcome == MatchOutcome::Draw)),
            losses: LossCount(losses.0 + u32::from(outcome == MatchOutcome::Loss)),
            ranking_points: points + match_points + bonus,
            bonus_points: bonus_total + bonus,
            // Compteurs de départage : accumulés sans condition d'activation
            // (règle 12). `diff_td` est dérivé, il n'est pas stocké (règle 13).
            td_for: TdFor(td_for.0 + u32::from(stats.own_td.0)),
            td_against: TdAgainst(td_against.0 + u32::from(stats.opponent_td.0)),
            casualties: CasualtiesTotal(casualties.0 + stats.casualties_inflicted.0),
            fouls: FoulsCommitted(fouls.0 + stats.fouls.0),
            completions: CompletionsMade(completions.0 + stats.completions.0),
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
            bonus_points: line.bonus_points,
            td_for: line.td_for,
            td_against: line.td_against,
            casualties: line.casualties,
            fouls: line.fouls,
            completions: line.completions,
        }
    }

    fn off_disabled() -> OffensiveBonusRule {
        OffensiveBonusRule { activated: BonusActivated(false), min_td: MinTd(0), points: RankingPoints(0) }
    }
    fn def_disabled() -> DefensiveBonusRule {
        DefensiveBonusRule { activated: BonusActivated(false), max_td_conceded: MaxTdConceded(0), points: RankingPoints(0) }
    }
    fn agg_disabled() -> AggressiveBonusRule {
        AggressiveBonusRule { activated: BonusActivated(false), min_casualties: MinCasualties(0), points: RankingPoints(0) }
    }

    fn rules() -> RankingRules {
        RankingRules {
            win_points: RankingPoints(3),
            draw_points: RankingPoints(1),
            lose_points: RankingPoints(0),
            offensive_bonus: off_disabled(),
            defensive_bonus: def_disabled(),
            aggressive_bonus: agg_disabled(),
        }
    }

    fn stats(own_td: u8, opponent_td: u8, casualties: u32) -> MatchStats {
        stats_with(own_td, opponent_td, casualties, 0, 0)
    }

    /// Variante complète, pour les tests des compteurs de départage.
    fn stats_with(
        own_td: u8,
        opponent_td: u8,
        casualties: u32,
        fouls: u32,
        completions: u32,
    ) -> MatchStats {
        MatchStats {
            own_td: MatchScore(own_td),
            opponent_td: MatchScore(opponent_td),
            casualties_inflicted: CasualtiesInflicted(casualties),
            fouls: FoulsCommitted(fouls),
            completions: CompletionsMade(completions),
        }
    }

    fn ctx() -> MatchContext {
        let (team_id, competition_id, season_id, round_id, match_report_id) = ids();
        MatchContext { team_id, competition_id, season_id, round_id, match_report_id, recorded_at: Utc::now() }
    }

    /// Stats produisant l'outcome voulu, sans sortie (pour les tests V/N/D).
    fn stats_for(outcome: MatchOutcome) -> MatchStats {
        match outcome {
            MatchOutcome::Win => stats(2, 1, 0),
            MatchOutcome::Draw => stats(1, 1, 0),
            MatchOutcome::Loss => stats(0, 2, 0),
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
        let c = ctx();
        // home marque 2 - away 1 : home gagne, away perd (scores croisés).
        let home_line = RankingLine::record_match(None, c.clone(), stats(2, 1, 0), &rules());
        let away_line = RankingLine::record_match(None, c, stats(1, 2, 0), &rules());

        assert_eq!(home_line.wins.0, 1);
        assert_eq!(home_line.losses.0, 0);
        assert_eq!(away_line.wins.0, 0);
        assert_eq!(away_line.losses.0, 1);
    }

    #[test]
    fn record_match_without_previous_line_starts_from_zero() {
        let line = RankingLine::record_match(None, ctx(), stats_for(MatchOutcome::Win), &rules());

        assert_eq!(line.matches_played.0, 1);
        assert_eq!(line.wins.0, 1);
        assert_eq!(line.draws.0, 0);
        assert_eq!(line.losses.0, 0);
        assert_eq!(line.ranking_points.0, 3);
    }

    #[test]
    fn record_match_with_previous_line_accumulates() {
        let c = ctx();
        let previous = RankingLine::record_match(None, c.clone(), stats_for(MatchOutcome::Win), &rules());
        let next = RankingLine::record_match(Some(totals_of(&previous)), c, stats_for(MatchOutcome::Draw), &rules());

        assert_eq!(next.matches_played.0, 2);
        assert_eq!(next.wins.0, 1);
        assert_eq!(next.draws.0, 1);
        assert_eq!(next.losses.0, 0);
        assert_eq!(next.ranking_points.0, 4); // 3 (victoire) + 1 (nul)
    }

    #[test]
    fn record_match_accumulates_over_three_successive_matches() {
        let c = ctx();
        let mut line: Option<RankingLine> = None;

        for outcome in [MatchOutcome::Win, MatchOutcome::Draw, MatchOutcome::Loss] {
            line = Some(RankingLine::record_match(
                line.as_ref().map(totals_of), c.clone(), stats_for(outcome), &rules(),
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
        let custom_rules = RankingRules {
            win_points: RankingPoints(5),
            draw_points: RankingPoints(2),
            lose_points: RankingPoints(1),
            offensive_bonus: off_disabled(),
            defensive_bonus: def_disabled(),
            aggressive_bonus: agg_disabled(),
        };

        let win = RankingLine::record_match(None, ctx(), stats_for(MatchOutcome::Win), &custom_rules);
        let draw = RankingLine::record_match(None, ctx(), stats_for(MatchOutcome::Draw), &custom_rules);
        let loss = RankingLine::record_match(None, ctx(), stats_for(MatchOutcome::Loss), &custom_rules);

        assert_eq!(win.ranking_points.0, 5);
        assert_eq!(draw.ranking_points.0, 2);
        assert_eq!(loss.ranking_points.0, 1);
    }

    #[test]
    fn losing_team_still_receives_bonus_via_record_match() {
        let mut r = rules();
        r.aggressive_bonus = AggressiveBonusRule {
            activated: BonusActivated(true), min_casualties: MinCasualties(1), points: RankingPoints(2),
        };
        // Défaite 0-2 mais 3 sorties infligées (>1) → lose_points(0) + bonus(2).
        let line = RankingLine::record_match(None, ctx(), stats(0, 2, 3), &r);
        assert_eq!(line.losses.0, 1);
        assert_eq!(line.ranking_points.0, 2);
    }

    // ── Bonus de classement — `RankingRules::bonus_points` ────────────────────

    fn with_offensive(activated: bool, min_td: u32, points: u32) -> RankingRules {
        let mut r = rules();
        r.offensive_bonus = OffensiveBonusRule {
            activated: BonusActivated(activated), min_td: MinTd(min_td), points: RankingPoints(points),
        };
        r
    }
    fn with_defensive(activated: bool, max_td_conceded: u32, points: u32) -> RankingRules {
        let mut r = rules();
        r.defensive_bonus = DefensiveBonusRule {
            activated: BonusActivated(activated), max_td_conceded: MaxTdConceded(max_td_conceded), points: RankingPoints(points),
        };
        r
    }
    fn with_aggressive(activated: bool, min_casualties: u32, points: u32) -> RankingRules {
        let mut r = rules();
        r.aggressive_bonus = AggressiveBonusRule {
            activated: BonusActivated(activated), min_casualties: MinCasualties(min_casualties), points: RankingPoints(points),
        };
        r
    }

    // ── Compteurs de départage (carte 216) ───────────────────────────────────

    fn counters_of(line: &RankingLine) -> [u32; 5] {
        [line.td_for.0, line.td_against.0, line.casualties.0, line.fouls.0, line.completions.0]
    }

    #[test]
    fn tiebreak_counters_accumulate_over_successive_matches() {
        let c = ctx();
        // 3-1 (2 sorties, 1 agression, 4 passes) puis 1-2 (0 sortie, 3 agressions, 2 passes).
        let first = RankingLine::record_match(None, c.clone(), stats_with(3, 1, 2, 1, 4), &rules());
        let second =
            RankingLine::record_match(Some(totals_of(&first)), c, stats_with(1, 2, 0, 3, 2), &rules());

        assert_eq!(counters_of(&first), [3, 1, 2, 1, 4]);
        assert_eq!(counters_of(&second), [4, 3, 2, 4, 6]);
    }

    /// Règle 12 : les compteurs sont alimentés quelle que soit la configuration —
    /// ni les bonus ni les critères de départage ne conditionnent l'accumulation.
    #[test]
    fn tiebreak_counters_accumulate_even_when_no_bonus_is_activated() {
        // `rules()` a les trois bonus désactivés.
        let line = RankingLine::record_match(None, ctx(), stats_with(2, 1, 5, 3, 7), &rules());

        assert_eq!(line.bonus_points.0, 0);
        assert_eq!(counters_of(&line), [2, 1, 5, 3, 7]);
    }

    /// Les compteurs partent de zéro sans ligne précédente — sinon le premier
    /// match d'une saison hériterait des totaux de la précédente.
    #[test]
    fn tiebreak_counters_start_from_zero_without_previous_line() {
        let line = RankingLine::record_match(None, ctx(), stats_with(0, 0, 0, 0, 0), &rules());
        assert_eq!(counters_of(&line), [0, 0, 0, 0, 0]);
    }

    // ── Part bonus du total (carte 213) ──────────────────────────────────────

    #[test]
    fn bonus_points_are_accumulated_across_matches_and_stay_a_subset_of_the_total() {
        let r = with_aggressive(true, 1, 2);

        // Match 1 : victoire 2-1 avec 3 sorties → 3 pts de victoire + 2 de bonus.
        let first = RankingLine::record_match(None, ctx(), stats(2, 1, 3), &r);
        assert_eq!(first.ranking_points.0, 5);
        assert_eq!(first.bonus_points.0, 2);

        // Match 2 : nul 1-1 avec 3 sorties → +1 pt de nul, +2 de bonus.
        let second = RankingLine::record_match(Some(totals_of(&first)), ctx(), stats(1, 1, 3), &r);
        assert_eq!(second.ranking_points.0, 8);
        assert_eq!(second.bonus_points.0, 4);
        assert!(second.bonus_points.0 <= second.ranking_points.0);
    }

    #[test]
    fn bonus_points_stay_at_zero_when_no_bonus_is_activated() {
        // `rules()` a les trois bonus désactivés : le total est purement V/N/D.
        let line = RankingLine::record_match(None, ctx(), stats(4, 0, 5), &rules());
        assert_eq!(line.ranking_points.0, 3);
        assert_eq!(line.bonus_points.0, 0);
    }

    #[test]
    fn bonus_points_are_carried_over_when_a_match_earns_none() {
        let r = with_aggressive(true, 1, 2);
        let first = RankingLine::record_match(None, ctx(), stats(2, 1, 3), &r);

        // Match sans sortie : aucun bonus gagné, mais le cumul acquis est conservé.
        let second = RankingLine::record_match(Some(totals_of(&first)), ctx(), stats(2, 1, 0), &r);
        assert_eq!(second.bonus_points.0, 2);
        assert_eq!(second.ranking_points.0, 8);
    }

    #[test]
    fn offensive_bonus_granted_when_activated_and_threshold_met() {
        let r = with_offensive(true, 3, 2);
        assert_eq!(r.bonus_points(&stats(4, 0, 0)).0, 2); // 4 >= 3
    }

    #[test]
    fn offensive_bonus_boundary_is_inclusive() {
        let r = with_offensive(true, 3, 2);
        assert_eq!(r.bonus_points(&stats(3, 0, 0)).0, 2); // 3 >= 3 (≥ large)
    }

    #[test]
    fn offensive_bonus_zero_when_below_threshold() {
        let r = with_offensive(true, 3, 2);
        assert_eq!(r.bonus_points(&stats(2, 0, 0)).0, 0);
    }

    #[test]
    fn offensive_bonus_zero_when_deactivated_even_if_met() {
        let r = with_offensive(false, 3, 2);
        assert_eq!(r.bonus_points(&stats(5, 0, 0)).0, 0);
    }

    #[test]
    fn defensive_bonus_granted_when_conceded_at_or_below_threshold() {
        let r = with_defensive(true, 1, 3);
        assert_eq!(r.bonus_points(&stats(0, 1, 0)).0, 3); // 1 <= 1 (≤ large)
        assert_eq!(r.bonus_points(&stats(0, 0, 0)).0, 3);
    }

    #[test]
    fn defensive_bonus_zero_when_above_threshold() {
        let r = with_defensive(true, 1, 3);
        assert_eq!(r.bonus_points(&stats(0, 2, 0)).0, 0);
    }

    #[test]
    fn defensive_bonus_zero_when_deactivated_even_if_met() {
        let r = with_defensive(false, 1, 3);
        assert_eq!(r.bonus_points(&stats(0, 0, 0)).0, 0);
    }

    #[test]
    fn aggressive_bonus_is_strict_greater_than() {
        let r = with_aggressive(true, 2, 1);
        assert_eq!(r.bonus_points(&stats(0, 0, 2)).0, 0); // == seuil → non (strict)
        assert_eq!(r.bonus_points(&stats(0, 0, 3)).0, 1); // > seuil → oui
    }

    #[test]
    fn aggressive_bonus_zero_when_deactivated_even_if_met() {
        let r = with_aggressive(false, 2, 1);
        assert_eq!(r.bonus_points(&stats(0, 0, 5)).0, 0);
    }

    #[test]
    fn bonuses_are_cumulative() {
        let mut r = rules();
        r.offensive_bonus = OffensiveBonusRule { activated: BonusActivated(true), min_td: MinTd(2), points: RankingPoints(1) };
        r.defensive_bonus = DefensiveBonusRule { activated: BonusActivated(true), max_td_conceded: MaxTdConceded(0), points: RankingPoints(2) };
        r.aggressive_bonus = AggressiveBonusRule { activated: BonusActivated(true), min_casualties: MinCasualties(1), points: RankingPoints(3) };
        // 3 TD marqués (≥2), 0 encaissé (≤0), 2 sorties (>1) → 1+2+3
        assert_eq!(r.bonus_points(&stats(3, 0, 2)).0, 6);
    }

    #[test]
    fn no_bonus_points_when_all_deactivated() {
        assert_eq!(rules().bonus_points(&stats(9, 0, 9)).0, 0);
    }
}
