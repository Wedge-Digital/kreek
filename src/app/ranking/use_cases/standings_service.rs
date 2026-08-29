//! Conversion des DTOs du port `ranking` vers le domaine, et ordonnancement du
//! classement.
//!
//! Ce service est le **seul** point de passage : ni les handlers ni les widgets ne
//! manipulent `RankingLineRow` ou `TiebreakSettingInfo` pour en tirer des objets
//! du domaine. Il ne contient aucune logique de comparaison — « qui est devant ? »
//! vit dans `domain/standings.rs`.

use crate::app::ranking::domain::ranking_line::{
    CasualtiesTotal, CompletionsMade, CumulativeTotals, DrawCount, FoulsCommitted, LossCount,
    MatchesPlayed, RankingPoints, TdAgainst, TdFor, WinCount,
};
use crate::app::ranking::domain::standings::{
    assign_ranks, order_standings, Rank, TeamStanding, TiebreakOrder,
};
use crate::app::ranking::domain::tiebreak::TiebreakCriterion;
use crate::app::ranking::ports::{RankingLineRow, RankingRulesInfo, TiebreakSettingInfo};
use std::collections::HashMap;

/// Configuration du port → ordre de départage du domaine. Ne peut vivre ni dans
/// le domaine (il ignore les types du port) ni dans le seul use case d'écriture
/// (la lecture en a besoin aussi).
///
/// L'ordre du vecteur d'entrée est préservé : c'est lui qui porte la priorité.
pub fn to_tiebreak_order(settings: &[TiebreakSettingInfo]) -> TiebreakOrder {
    let criteria = settings
        .iter()
        .filter(|setting| setting.activated)
        .filter_map(|setting| resolve_criterion(&setting.code))
        .collect();
    TiebreakOrder::new(criteria)
}

/// Un code inconnu est **sauté**, pas remonté en erreur (décision D2) : la
/// configuration persistée peut référencer un critère retiré du catalogue, et le
/// classement doit rester affichable. Le `warn!` porte le code fautif, sans quoi
/// un départage disparu resterait introuvable.
fn resolve_criterion(code: &str) -> Option<TiebreakCriterion> {
    let criterion = TiebreakCriterion::from_code(code);
    if criterion.is_none() {
        tracing::warn!("critère de départage inconnu, ignoré : « {code} »");
    }
    criterion
}

/// Lignes de classement → équipes ordonnées, chacune avec son rang.
///
/// Appelé **par groupe** : chaque poule est un classement autonome dont les rangs
/// repartent à 1.
pub fn build_ordered_standings(
    lines: Vec<RankingLineRow>,
    manual: &HashMap<String, i32>,
    order: &TiebreakOrder,
) -> Vec<(TeamStanding, Rank)> {
    let mut standings: Vec<TeamStanding> = lines
        .into_iter()
        .map(|row| to_standing(row, manual))
        .collect();
    order_standings(&mut standings, order);
    let ranks = assign_ranks(&standings, order);
    standings.into_iter().zip(ranks).collect()
}

/// **Une équipe absente de la carte a zéro point manuel**, ce qui est le cas
/// commun : la carte ne porte que les équipes qui en ont reçu. Elle n'est donc
/// jamais complète, et `unwrap_or(0)` n'est pas un repli mais la règle.
fn to_standing(row: RankingLineRow, manual: &HashMap<String, i32>) -> TeamStanding {
    let manual_points = manual.get(&row.team_id.to_string()).copied().unwrap_or(0);
    TeamStanding {
        team_id: row.team_id,
        totals: to_totals(row),
        manual_points,
    }
}

/// Sans règles configurées, l'ordre est vide — le classement n'est de toute
/// façon pas affiché, mais l'ordre reste un état valide plutôt qu'une absence.
pub fn tiebreak_order_of(rules: &Option<RankingRulesInfo>) -> TiebreakOrder {
    rules
        .as_ref()
        .map(|r| to_tiebreak_order(&r.tiebreakers))
        .unwrap_or_else(TiebreakOrder::empty)
}

pub fn to_totals(row: RankingLineRow) -> CumulativeTotals {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::ports::BonusRuleInfo;
    use crate::app::shared_kernel::bloodbowl::team::TeamId;

    fn setting(code: &str, activated: bool) -> TiebreakSettingInfo {
        TiebreakSettingInfo {
            code: code.into(),
            activated,
        }
    }

    fn row(points: u32, td_for: u32) -> RankingLineRow {
        RankingLineRow {
            team_id: TeamId::new(),
            matches_played: 1,
            wins: 0,
            draws: 0,
            losses: 0,
            ranking_points: points,
            bonus_points: 0,
            td_for,
            td_against: 0,
            casualties: 0,
            fouls: 0,
            completions: 0,
        }
    }

    #[test]
    fn to_tiebreak_order_keeps_only_activated_criteria() {
        let settings = vec![
            setting("nb_cas", false),
            setting("nb_td", true),
            setting("diff_td", false),
        ];

        let order = to_tiebreak_order(&settings);

        assert_eq!(order, TiebreakOrder::new(vec![TiebreakCriterion::NbTd]));
    }

    /// L'ordre du vecteur porte la priorité : un tri ou un détour par une table
    /// de hachage la détruirait sans que rien ne le signale. L'ordre choisi ici
    /// est volontairement l'inverse de l'ordre canonique du catalogue.
    #[test]
    fn to_tiebreak_order_preserves_the_configured_priority() {
        let settings = vec![
            setting("nb_reu", true),
            setting("nb_cas", true),
            setting("diff_td", true),
        ];

        let order = to_tiebreak_order(&settings);

        assert_eq!(
            order,
            TiebreakOrder::new(vec![
                TiebreakCriterion::NbReu,
                TiebreakCriterion::NbCas,
                TiebreakCriterion::DiffTd
            ])
        );
    }

    /// Décision D2 : un critère retiré du catalogue (ici les cartons rouges) ne
    /// fait pas échouer la lecture, il disparaît de l'ordre — et les critères
    /// **suivants** conservent leur priorité relative.
    #[test]
    fn to_tiebreak_order_skips_an_unknown_code_without_dropping_the_rest() {
        let settings = vec![
            setting("nb_cas", true),
            setting("nb_red_cards", true),
            setting("nb_td", true),
        ];

        let order = to_tiebreak_order(&settings);

        assert_eq!(
            order,
            TiebreakOrder::new(vec![TiebreakCriterion::NbCas, TiebreakCriterion::NbTd])
        );
    }

    #[test]
    fn to_tiebreak_order_of_an_empty_configuration_is_empty() {
        assert_eq!(to_tiebreak_order(&[]), TiebreakOrder::empty());
    }

    #[test]
    fn build_ordered_standings_orders_by_points_and_numbers_from_one() {
        let (last, first, middle) = (row(3, 0), row(9, 0), row(6, 0));
        let expected_order = [first.team_id, middle.team_id, last.team_id];

        let ordered = build_ordered_standings(
            vec![last, first, middle],
            &HashMap::new(),
            &TiebreakOrder::empty(),
        );

        let team_ids: Vec<TeamId> = ordered.iter().map(|(s, _)| s.team_id).collect();
        assert_eq!(team_ids, expected_order);
        assert_eq!(
            ordered.iter().map(|(_, r)| r.0).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }

    /// Le critère actif départage deux équipes à égalité de points — c'est la
    /// jonction entre la configuration du port et la comparaison du domaine.
    #[test]
    fn build_ordered_standings_applies_the_configured_criterion() {
        let (poor, prolific) = (row(6, 1), row(6, 7));
        let expected_first = prolific.team_id;

        let order = to_tiebreak_order(&[setting("nb_td", true)]);
        let ordered = build_ordered_standings(vec![poor, prolific], &HashMap::new(), &order);

        assert_eq!(ordered[0].0.team_id, expected_first);
        assert_eq!(ordered[0].1 .0, 1);
    }

    /// Sans critère actif, deux équipes à égalité de points sont ex æquo : même
    /// rang, et le rang suivant saute (règle 20).
    #[test]
    fn build_ordered_standings_gives_the_same_rank_to_tied_teams() {
        let lines = vec![row(9, 0), row(6, 5), row(6, 2), row(1, 0)];

        let ordered = build_ordered_standings(lines, &HashMap::new(), &TiebreakOrder::empty());

        assert_eq!(
            ordered.iter().map(|(_, r)| r.0).collect::<Vec<_>>(),
            vec![1, 2, 2, 4]
        );
    }

    // ── `tiebreak_order_of`, déplacé depuis `classement_widget` (carte 221) ──

    fn rules_with(tiebreakers: Vec<TiebreakSettingInfo>) -> RankingRulesInfo {
        let no_bonus = || BonusRuleInfo {
            activated: false,
            threshold: 0,
            points: 0,
        };
        RankingRulesInfo {
            win_points: 3,
            draw_points: 1,
            lose_points: 0,
            offensive: no_bonus(),
            defensive: no_bonus(),
            aggressive: no_bonus(),
            tiebreakers,
        }
    }

    /// Jonction entre la configuration de la compétition et le chemin de lecture :
    /// si l'ordre n'était pas construit ici, le classement se réduirait
    /// silencieusement aux points et le départage n'aurait aucun effet visible.
    #[test]
    fn tiebreak_order_of_builds_the_configured_order() {
        let rules = Some(rules_with(vec![
            setting("nb_cas", true),
            setting("nb_td", true),
        ]));

        let order = tiebreak_order_of(&rules);

        assert_eq!(
            order,
            TiebreakOrder::new(vec![TiebreakCriterion::NbCas, TiebreakCriterion::NbTd])
        );
    }

    #[test]
    fn tiebreak_order_of_drops_deactivated_criteria() {
        let rules = Some(rules_with(vec![
            setting("nb_cas", false),
            setting("nb_td", true),
        ]));

        assert_eq!(
            tiebreak_order_of(&rules),
            TiebreakOrder::new(vec![TiebreakCriterion::NbTd])
        );
    }

    /// Sans règles configurées, le classement n'est pas affiché — l'ordre reste
    /// un état valide plutôt qu'une absence à traiter en aval.
    #[test]
    fn tiebreak_order_of_none_is_empty() {
        assert_eq!(tiebreak_order_of(&None), TiebreakOrder::empty());
    }

    // ── Points manuels (carte 449) ───────────────────────────────────────────

    /// **La carte est réellement lue.**
    ///
    /// Les deux appelants la passent vide jusqu'à la carte 451 : sans ce test,
    /// `to_standing` pourrait ignorer son argument et poser zéro sans que rien
    /// ne bronche. La 451 chercherait alors longtemps pourquoi ses points
    /// n'apparaissent pas.
    #[test]
    fn la_carte_des_points_manuels_atteint_le_classement() {
        let ligne = row(3, 0);
        let equipe = ligne.team_id.to_string();
        let manuels = HashMap::from([(equipe, 2)]);

        let ordered = build_ordered_standings(vec![ligne], &manuels, &TiebreakOrder::empty());

        assert_eq!(ordered[0].0.manual_points, 2);
        assert_eq!(ordered[0].0.total_points(), 5);
    }

    /// Une équipe absente de la carte vaut zéro — ce n'est pas un repli, c'est
    /// le cas commun : la carte ne porte que les équipes qui ont reçu des
    /// points.
    #[test]
    fn une_equipe_absente_de_la_carte_a_zero_point_manuel() {
        let ligne = row(4, 0);
        let autre = HashMap::from([(TeamId::new().to_string(), 50)]);

        let ordered = build_ordered_standings(vec![ligne], &autre, &TiebreakOrder::empty());

        assert_eq!(ordered[0].0.manual_points, 0);
        assert_eq!(ordered[0].0.total_points(), 4);
    }

    /// L'ordre tient compte des points manuels **avant** d'attribuer les rangs :
    /// une équipe deuxième aux points passe première si sa bonification l'y met.
    #[test]
    fn les_points_manuels_changent_les_rangs_attribues() {
        let devancee = row(5, 0);
        let bonifiee = row(3, 0);
        let bonifiee_id = bonifiee.team_id.to_string();
        let manuels = HashMap::from([(bonifiee_id.clone(), 4)]);

        let ordered =
            build_ordered_standings(vec![devancee, bonifiee], &manuels, &TiebreakOrder::empty());

        assert_eq!(
            ordered[0].0.team_id.to_string(),
            bonifiee_id,
            "3 + 4 = 7 > 5"
        );
        assert_eq!(ordered[0].1 .0, 1);
        assert_eq!(ordered[1].1 .0, 2);
    }
}
