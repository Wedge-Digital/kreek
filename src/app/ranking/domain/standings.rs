//! Ordonnancement des équipes au classement : comparaison, tri et rangs.
//!
//! Rien ici ne peut échouer — il n'y a aucun invariant à protéger, seulement des
//! valeurs à comparer. Aucune variante n'est donc ajoutée à `DomainError`.

use crate::app::ranking::domain::ranking_line::CumulativeTotals;
use crate::app::ranking::domain::tiebreak::{Direction, TiebreakCriterion};
use crate::app::shared_kernel::team::TeamId;
use std::cmp::Ordering;

/// Rang au classement — 1 pour la tête.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rank(pub u32);

/// Une équipe et les totaux cumulés sur lesquels le classement la compare.
#[derive(Debug, Clone)]
pub struct TeamStanding {
    pub team_id: TeamId,
    pub totals: CumulativeTotals,
}

/// Critères de départage actifs, dans l'ordre de priorité choisi par le
/// gestionnaire — c'est l'index qui porte la priorité.
///
/// Vide est un état valide : l'ordre se réduit alors aux points de classement et
/// toute égalité de points devient un ex æquo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TiebreakOrder {
    criteria: Vec<TiebreakCriterion>,
}

impl TiebreakOrder {
    pub fn new(criteria: Vec<TiebreakCriterion>) -> Self {
        Self { criteria }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    /// Lecture seule des critères actifs, dans l'ordre de priorité — l'affichage
    /// en a besoin pour construire une colonne par critère.
    pub fn criteria(&self) -> &[TiebreakCriterion] {
        &self.criteria
    }
}

/// Règle 18 : les points de classement d'abord, puis chaque critère actif dans
/// l'ordre jusqu'au premier qui départage. Règle 19 : `Equal` si tous sont
/// égaux — l'ex æquo est assumé, il n'existe pas de départage ultime.
pub fn compare(a: &TeamStanding, b: &TeamStanding, order: &TiebreakOrder) -> Ordering {
    b.totals.ranking_points.0.cmp(&a.totals.ranking_points.0).then_with(|| {
        order
            .criteria
            .iter()
            .map(|criterion| compare_on(*criterion, a, b))
            .find(|ordering| ordering.is_ne())
            .unwrap_or(Ordering::Equal)
    })
}

/// Compare deux équipes sur un seul critère, dans le sens qui lui est propre
/// (règle 17).
fn compare_on(criterion: TiebreakCriterion, a: &TeamStanding, b: &TeamStanding) -> Ordering {
    let (value_a, value_b) = (criterion.value_of(&a.totals), criterion.value_of(&b.totals));
    match criterion.direction() {
        Direction::Desc => value_b.cmp(&value_a),
        Direction::Asc => value_a.cmp(&value_b),
    }
}

/// Tri **stable** : deux équipes strictement ex æquo conservent leur ordre
/// d'entrée. Sans cette garantie, le classement pourrait permuter d'un
/// affichage à l'autre sans qu'aucun match ait été joué.
pub fn order_standings(standings: &mut [TeamStanding], order: &TiebreakOrder) {
    standings.sort_by(|a, b| compare(a, b, order));
}

/// Règle 20 : numérotation standard après ex æquo (1, 2, 2, 4). « Même rang que
/// le précédent si `compare` renvoie `Equal`, sinon `idx + 1` » — aucun compteur
/// d'ex æquo à tenir, donc aucun cas particulier à oublier.
///
/// Attend un tableau **déjà ordonné** par `order_standings` : les rangs sont lus
/// de la position, la comparaison ne sert qu'à détecter les égalités voisines.
pub fn assign_ranks(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<Rank> {
    let mut ranks: Vec<Rank> = Vec::with_capacity(ordered.len());
    for (idx, standing) in ordered.iter().enumerate() {
        let tied = idx > 0 && compare(&ordered[idx - 1], standing, order).is_eq();
        let rank = match (tied, ranks.last()) {
            (true, Some(previous)) => *previous,
            _ => Rank((idx + 1) as u32),
        };
        ranks.push(rank);
    }
    ranks
}

/// Ce qui a décidé de la position d'une équipe parmi celles qui partagent son
/// total de points de classement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowTiebreak {
    /// Seule à son total : aucune égalité à résoudre.
    Alone,
    /// Départagée par le critère d'index donné ; tous ceux qui le précèdent
    /// étaient égaux au sein de son sous-groupe (règle 21).
    DecidedBy(usize),
    /// Tous les critères actifs sont égaux — ex æquo (règles 19 et 22).
    FullyTied,
}

/// Règle 21 : pour chaque équipe, le critère qui l'a séparée de celles **encore**
/// à égalité avec elle. Règle 22 : `FullyTied` quand aucun n'y parvient.
///
/// Attend un tableau **déjà ordonné** par `order_standings` : les équipes à
/// égalité de points y sont consécutives, les points étant la clé de tri primaire.
pub fn tiebreak_outcomes(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<RowTiebreak> {
    let mut outcomes = vec![RowTiebreak::Alone; ordered.len()];
    for (from, len) in point_runs(ordered) {
        resolve_run(ordered, order, from, len, 0, &mut outcomes);
    }
    outcomes
}

/// Descente par sous-groupes : le critère `k` règle les équipes qu'il isole, et
/// celles qu'il laisse à égalité repassent au critère suivant.
///
/// Marquer d'un coup tout le groupe sur le premier critère non constant
/// désignerait ce critère comme décisif sur des lignes qu'il n'a pas départagées
/// — deux valeurs identiques mises en évidence.
fn resolve_run(
    ordered: &[TeamStanding],
    order: &TiebreakOrder,
    from: usize,
    len: usize,
    k: usize,
    outcomes: &mut [RowTiebreak],
) {
    let Some(criterion) = order.criteria.get(k) else {
        outcomes[from..from + len].fill(RowTiebreak::FullyTied);
        return;
    };
    for (sub_from, sub_len) in runs_by(ordered, from, len, |s| criterion.value_of(&s.totals)) {
        match sub_len {
            1 => outcomes[sub_from] = RowTiebreak::DecidedBy(k),
            _ => resolve_run(ordered, order, sub_from, sub_len, k + 1, outcomes),
        }
    }
}

/// Suites d'équipes consécutives à égalité de points, de deux équipes ou plus —
/// les autres n'ont rien à départager.
fn point_runs(ordered: &[TeamStanding]) -> Vec<(usize, usize)> {
    runs_by(ordered, 0, ordered.len(), |s| i64::from(s.totals.ranking_points.0))
        .into_iter()
        .filter(|(_, len)| *len >= 2)
        .collect()
}

/// Découpe `[from, from + len)` en suites consécutives de même valeur.
fn runs_by(
    ordered: &[TeamStanding],
    from: usize,
    len: usize,
    value: impl Fn(&TeamStanding) -> i64,
) -> Vec<(usize, usize)> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for idx in from..from + len {
        match runs.last_mut() {
            Some((start, run_len)) if value(&ordered[*start]) == value(&ordered[idx]) => {
                *run_len += 1
            }
            _ => runs.push((idx, 1)),
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::domain::ranking_line::{
        CasualtiesTotal, CompletionsMade, FoulsCommitted, RankingPoints, TdAgainst, TdFor, WinCount,
    };

    fn standing(points: u32) -> TeamStanding {
        TeamStanding {
            team_id: TeamId::new(),
            totals: CumulativeTotals {
                ranking_points: RankingPoints(points),
                ..CumulativeTotals::ZERO
            },
        }
    }

    /// `standing`, plus les compteurs que le test fait varier.
    fn standing_with(points: u32, tweak: impl FnOnce(&mut CumulativeTotals)) -> TeamStanding {
        let mut s = standing(points);
        tweak(&mut s.totals);
        s
    }

    fn all_criteria() -> TiebreakOrder {
        TiebreakOrder::new(TiebreakCriterion::all())
    }

    fn ranks_of(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<u32> {
        assign_ranks(ordered, order).iter().map(|r| r.0).collect()
    }

    /// Règle 18 : `Ordering::Less` signifie « passe devant » au tri.
    #[test]
    fn ranking_points_prevail_over_every_criterion() {
        // Le leader est derrière sur les sept critères, mais devant aux points.
        let leader = standing(9);
        let chaser = standing_with(6, |t| {
            t.td_for = TdFor(50);
            t.wins = WinCount(50);
            t.casualties = CasualtiesTotal(50);
            t.fouls = FoulsCommitted(50);
            t.completions = CompletionsMade(50);
        });

        assert_eq!(compare(&leader, &chaser, &all_criteria()), Ordering::Less);
        assert_eq!(compare(&chaser, &leader, &all_criteria()), Ordering::Greater);
    }

    #[test]
    fn at_equal_points_the_first_criterion_decides() {
        // Les sorties priment sur les TD : `b` marque plus mais sort moins.
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbCas, TiebreakCriterion::NbTd]);
        let a = standing_with(6, |t| {
            t.casualties = CasualtiesTotal(4);
            t.td_for = TdFor(1);
        });
        let b = standing_with(6, |t| {
            t.casualties = CasualtiesTotal(2);
            t.td_for = TdFor(9);
        });

        assert_eq!(compare(&a, &b, &order), Ordering::Less);
    }

    #[test]
    fn the_second_criterion_decides_when_the_first_is_equal() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbCas, TiebreakCriterion::NbTd]);
        let a = standing_with(6, |t| {
            t.casualties = CasualtiesTotal(3);
            t.td_for = TdFor(2);
        });
        let b = standing_with(6, |t| {
            t.casualties = CasualtiesTotal(3);
            t.td_for = TdFor(5);
        });

        assert_eq!(compare(&a, &b, &order), Ordering::Greater);
    }

    /// Règle 19 : l'ex æquo résiduel est assumé, aucun départage de dernier
    /// recours (tirage, ordre alphabétique) ne doit apparaître ici.
    #[test]
    fn teams_equal_on_every_criterion_stay_tied() {
        let tied = |points| {
            standing_with(points, |t| {
                t.td_for = TdFor(4);
                t.td_against = TdAgainst(2);
                t.wins = WinCount(2);
                t.casualties = CasualtiesTotal(3);
                t.fouls = FoulsCommitted(1);
                t.completions = CompletionsMade(5);
            })
        };

        assert_eq!(compare(&tied(6), &tied(6), &all_criteria()), Ordering::Equal);
    }

    #[test]
    fn an_empty_order_compares_on_points_only() {
        let order = TiebreakOrder::empty();
        let prolific = standing_with(6, |t| t.td_for = TdFor(20));
        let barren = standing_with(6, |t| t.td_for = TdFor(0));

        // Les compteurs n'entrent pas en jeu : égalité de points ⇒ ex æquo.
        assert_eq!(compare(&prolific, &barren, &order), Ordering::Equal);
        // Les points, eux, départagent toujours.
        assert_eq!(compare(&standing(9), &barren, &order), Ordering::Less);
    }

    /// Règle 17 : le seul critère où le plus petit compteur gagne.
    #[test]
    fn conceded_touchdowns_favour_the_tightest_defence() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTdConceded]);
        let tight = standing_with(6, |t| t.td_against = TdAgainst(2));
        let leaky = standing_with(6, |t| t.td_against = TdAgainst(7));

        assert_eq!(compare(&tight, &leaky, &order), Ordering::Less);
        assert_eq!(compare(&leaky, &tight, &order), Ordering::Greater);
    }

    /// Vingt-quatre équipes strictement identiques — au-delà du seuil sous lequel
    /// `sort_unstable_by` se comporte comme un tri par insertion, pour que le test
    /// ait une chance de repérer un passage au tri instable. Il verrouille surtout
    /// l'intention : l'ordre d'entrée survit quand `compare` renvoie `Equal`.
    #[test]
    fn order_standings_keeps_the_entry_order_of_tied_teams() {
        let mut standings: Vec<TeamStanding> = (0..24).map(|_| standing(6)).collect();
        let entry_order: Vec<String> = standings.iter().map(|s| s.team_id.to_string()).collect();

        order_standings(&mut standings, &all_criteria());

        let after: Vec<String> = standings.iter().map(|s| s.team_id.to_string()).collect();
        assert_eq!(after, entry_order);
    }

    #[test]
    fn order_standings_sorts_by_points_then_by_criterion() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd]);
        let low = standing(3);
        let tied_few_td = standing_with(6, |t| t.td_for = TdFor(2));
        let tied_many_td = standing_with(6, |t| t.td_for = TdFor(8));
        let mut standings = vec![low.clone(), tied_few_td.clone(), tied_many_td.clone()];

        order_standings(&mut standings, &order);

        let ordered: Vec<String> = standings.iter().map(|s| s.team_id.to_string()).collect();
        assert_eq!(
            ordered,
            vec![tied_many_td.team_id.to_string(), tied_few_td.team_id.to_string(), low.team_id.to_string()]
        );
    }

    /// Règle 20 : deux équipes au rang 2, la suivante au rang 4 — pas 3.
    #[test]
    fn assign_ranks_numbers_ties_the_standard_way() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd]);
        let ordered = vec![
            standing(9),
            standing_with(6, |t| t.td_for = TdFor(2)),
            standing_with(6, |t| t.td_for = TdFor(2)),
            standing(3),
        ];

        assert_eq!(ranks_of(&ordered, &order), vec![1, 2, 2, 4]);
    }

    /// Égalité à **trois**, et non à deux : sur une égalité à deux, « rang du
    /// précédent » et « index » donnent le même nombre par coïncidence, et un
    /// `assign_ranks` écrit avec l'index passerait le test ci-dessus. Il faut un
    /// troisième ex æquo pour que les deux formules divergent (2, 2, 2 contre
    /// 2, 2, 3).
    #[test]
    fn assign_ranks_holds_the_same_rank_across_a_triple_tie() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd]);
        let tied = || standing_with(6, |t| t.td_for = TdFor(2));
        let ordered = vec![standing(9), tied(), tied(), tied(), standing(3)];

        assert_eq!(ranks_of(&ordered, &order), vec![1, 2, 2, 2, 5]);
    }

    #[test]
    fn assign_ranks_puts_every_team_first_when_all_are_tied() {
        let ordered: Vec<TeamStanding> = (0..4).map(|_| standing(6)).collect();

        assert_eq!(ranks_of(&ordered, &all_criteria()), vec![1, 1, 1, 1]);
    }

    #[test]
    fn assign_ranks_gives_rank_one_to_a_lone_team() {
        assert_eq!(ranks_of(&[standing(0)], &all_criteria()), vec![1]);
    }

    // ── Critère décisif par équipe (carte 220) ───────────────────────────────

    use RowTiebreak::{Alone, DecidedBy, FullyTied};

    fn outcomes(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<RowTiebreak> {
        tiebreak_outcomes(ordered, order)
    }

    #[test]
    fn a_team_alone_on_its_points_total_has_nothing_to_resolve() {
        let ordered = vec![standing(9), standing(6)];
        assert_eq!(outcomes(&ordered, &all_criteria()), vec![Alone, Alone]);
    }

    #[test]
    fn the_first_criterion_decides_when_it_separates() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd]);
        let ordered = vec![
            standing_with(6, |t| t.td_for = TdFor(5)),
            standing_with(6, |t| t.td_for = TdFor(2)),
        ];

        assert_eq!(outcomes(&ordered, &order), vec![DecidedBy(0), DecidedBy(0)]);
    }

    #[test]
    fn the_second_criterion_decides_the_outcome_when_the_first_is_equal() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbCas, TiebreakCriterion::NbTd]);
        let tied_on_casualties = |td: u32| {
            standing_with(6, move |t| {
                t.casualties = CasualtiesTotal(3);
                t.td_for = TdFor(td);
            })
        };
        let ordered = vec![tied_on_casualties(5), tied_on_casualties(2)];

        assert_eq!(outcomes(&ordered, &order), vec![DecidedBy(1), DecidedBy(1)]);
    }

    /// Règle 22 : aucun critère ne tranche, l'ex æquo est assumé.
    #[test]
    fn teams_equal_on_every_criterion_are_fully_tied() {
        let identical = || standing_with(6, |t| t.td_for = TdFor(4));
        let ordered = vec![identical(), identical()];

        assert_eq!(outcomes(&ordered, &all_criteria()), vec![FullyTied, FullyTied]);
    }

    /// **Le test qui distingue la résolution par sous-groupes de la résolution à
    /// plat.** Trois équipes à égalité de points, touchdowns 5 / 2 / 2 : le premier
    /// critère isole la première, mais laisse les deux autres à égalité — c'est le
    /// second qui les départage. À plat, on désignerait le premier critère comme
    /// décisif sur les trois lignes, dont deux affichant la même valeur.
    #[test]
    fn a_criterion_that_leaves_a_sub_group_tied_does_not_decide_for_it() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd, TiebreakCriterion::NbCas]);
        let team = |td: u32, cas: u32| {
            standing_with(6, move |t| {
                t.td_for = TdFor(td);
                t.casualties = CasualtiesTotal(cas);
            })
        };
        let ordered = vec![team(5, 0), team(2, 4), team(2, 1)];

        assert_eq!(outcomes(&ordered, &order), vec![DecidedBy(0), DecidedBy(1), DecidedBy(1)]);
    }

    #[test]
    fn an_empty_order_leaves_teams_tied_on_points_fully_tied() {
        let ordered = vec![
            standing_with(6, |t| t.td_for = TdFor(9)),
            standing_with(6, |t| t.td_for = TdFor(0)),
        ];

        assert_eq!(outcomes(&ordered, &TiebreakOrder::empty()), vec![FullyTied, FullyTied]);
    }

    /// Deux totaux de points distincts sont deux problèmes distincts : le critère
    /// qui tranche l'un n'a pas à trancher l'autre.
    #[test]
    fn groups_on_different_points_totals_are_resolved_independently() {
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd, TiebreakCriterion::NbCas]);
        let team = |points: u32, td: u32, cas: u32| {
            standing_with(points, move |t| {
                t.td_for = TdFor(td);
                t.casualties = CasualtiesTotal(cas);
            })
        };
        // 9 pts : départagées par les TD. 6 pts : TD égaux, départagées par les sorties.
        let ordered = vec![team(9, 4, 0), team(9, 1, 0), team(6, 3, 5), team(6, 3, 2)];

        assert_eq!(
            outcomes(&ordered, &order),
            vec![DecidedBy(0), DecidedBy(0), DecidedBy(1), DecidedBy(1)]
        );
    }
}
