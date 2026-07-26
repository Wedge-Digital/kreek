use crate::app::ranking::domain::standings::{
    tiebreak_outcomes, Rank, RowTiebreak, TeamStanding, TiebreakOrder,
};
use crate::app::ranking::domain::tiebreak::TiebreakCriterion;
use crate::app::ranking::io::web::widgets::classement_widget::{ClassementGroupVm, ClassementRowVm};
use crate::app::ranking::io::web::widgets::detailed_standings_widget::{
    CellState, DetailedGroupVm, DetailedRowVm, TiebreakCellVm,
};
use crate::app::ranking::ports::{EnrolledTeamInfo, RankingGroupInfo, RankingLineRow};
use crate::app::ranking::use_cases::standings_service::build_ordered_standings;
use crate::app::routes::AppRoutes;
use std::collections::HashSet;

/// Une poule et les données qui la concernent — le **découpage seul**, sans rendu.
/// Partagé par les deux onglets de classement : sans lui, la règle « chaque poule
/// est un classement autonome » serait implémentée deux fois et pourrait diverger.
struct GroupSlice {
    title: Option<String>,
    lines: Vec<RankingLineRow>,
    teams: Vec<EnrolledTeamInfo>,
}

/// Un classement par poule si la saison en compte 2 ou plus (BR : chaque
/// poule a son propre classement, un tri global n'aurait pas de sens dès
/// qu'il y a plusieurs groupes) ; sinon un classement unique sur toute la
/// saison, comportement inchangé. Les équipes enrôlées non assignées à une
/// poule sont regroupées à part plutôt que de disparaître silencieusement.
fn split_into_groups(
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
) -> Vec<GroupSlice> {
    if groups.len() <= 1 {
        return vec![slice_for(None, None, lines, teams)];
    }

    let mut result: Vec<GroupSlice> = groups
        .iter()
        .map(|g| slice_for(Some(g.group_name.clone()), Some(&g.team_ids), lines, teams))
        .collect();

    if let Some(unassigned) = unassigned_slice(lines, teams, groups) {
        result.push(unassigned);
    }
    result
}

fn unassigned_slice(
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
) -> Option<GroupSlice> {
    let assigned: HashSet<&str> =
        groups.iter().flat_map(|g| g.team_ids.iter().map(String::as_str)).collect();
    let unassigned_ids: Vec<String> = teams
        .iter()
        .map(|t| t.team_id.clone())
        .filter(|id| !assigned.contains(id.as_str()))
        .collect();
    if unassigned_ids.is_empty() {
        return None;
    }
    let title = Some("Non assignées".to_string());
    Some(slice_for(title, Some(&unassigned_ids), lines, teams))
}

/// `team_ids: None` = pas de filtrage (classement à plat, saison sans poule).
fn slice_for(
    title: Option<String>,
    team_ids: Option<&[String]>,
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
) -> GroupSlice {
    GroupSlice {
        title,
        teams: teams
            .iter()
            .filter(|t| team_ids.is_none_or(|ids| ids.contains(&t.team_id)))
            .cloned()
            .collect(),
        lines: lines
            .iter()
            .filter(|l| team_ids.is_none_or(|ids| ids.contains(&l.team_id.to_string())))
            .cloned()
            .collect(),
    }
}

pub fn build_classement_groups(
    space_id: &str,
    lines: Vec<RankingLineRow>,
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
    order: &TiebreakOrder,
) -> Vec<ClassementGroupVm> {
    split_into_groups(&lines, teams, groups)
        .into_iter()
        .map(|slice| build_group_vm(space_id, slice, order))
        .collect()
}

/// Le découpage précède l'ordonnancement : chaque poule est un classement
/// autonome dont les rangs repartent à 1. Ordonner avant de découper donnerait
/// des rangs globaux — le leader de la poule 2 pourrait afficher un rang 3.
fn build_group_vm(space_id: &str, slice: GroupSlice, order: &TiebreakOrder) -> ClassementGroupVm {
    let ordered = build_ordered_standings(slice.lines, order);
    ClassementGroupVm {
        title: slice.title,
        has_enrolled_teams: !slice.teams.is_empty(),
        rows: build_classement_rows(space_id, ordered, &slice.teams),
    }
}

/// Habille des équipes **déjà ordonnées et rangées** par le domaine : résout le
/// nom depuis les équipes inscrites (port `competitions`), construit le lien, et
/// remplit le VM. Ni tri ni calcul de rang ici — cf. `domain/standings.rs`.
pub fn build_classement_rows(
    space_id: &str,
    ordered: Vec<(TeamStanding, Rank)>,
    teams: &[EnrolledTeamInfo],
) -> Vec<ClassementRowVm> {
    ordered
        .into_iter()
        .map(|(standing, rank)| {
            let team_id = standing.team_id.to_string();
            ClassementRowVm {
                rank: rank.0,
                team_name: resolve_team_name(&team_id, teams),
                team_link: AppRoutes::default().teams.team_detail(space_id, &team_id),
                played: standing.totals.matches_played.0,
                wins: standing.totals.wins.0,
                draws: standing.totals.draws.0,
                losses: standing.totals.losses.0,
                points: standing.totals.ranking_points.0,
            }
        })
        .collect()
}

/// À défaut d'inscription retrouvée, l'id tient lieu de nom — mieux qu'une
/// ligne vide, et le problème se voit.
fn resolve_team_name(team_id: &str, teams: &[EnrolledTeamInfo]) -> String {
    teams
        .iter()
        .find(|t| t.team_id == team_id)
        .map(|t| t.team_name.clone())
        .unwrap_or_else(|| team_id.to_string())
}

// ── Classement détaillé ───────────────────────────────────────────────────────

/// Même découpage par poule que le classement simple — les deux onglets ne
/// peuvent pas diverger sur le périmètre d'un classement.
pub fn build_detailed_groups(
    space_id: &str,
    lines: Vec<RankingLineRow>,
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
    order: &TiebreakOrder,
) -> Vec<DetailedGroupVm> {
    split_into_groups(&lines, teams, groups)
        .into_iter()
        .map(|slice| build_detailed_group_vm(space_id, slice, order))
        .collect()
}

fn build_detailed_group_vm(
    space_id: &str,
    slice: GroupSlice,
    order: &TiebreakOrder,
) -> DetailedGroupVm {
    let ordered = build_ordered_standings(slice.lines, order);
    DetailedGroupVm {
        title: slice.title,
        has_enrolled_teams: !slice.teams.is_empty(),
        rows: build_detailed_rows(space_id, ordered, &slice.teams, order),
    }
}

pub fn build_detailed_rows(
    space_id: &str,
    ordered: Vec<(TeamStanding, Rank)>,
    teams: &[EnrolledTeamInfo],
    order: &TiebreakOrder,
) -> Vec<DetailedRowVm> {
    // La résolution porte sur le classement **de cette poule** : chaque poule est
    // autonome, deux poules aux mêmes totaux ne s'influencent pas.
    let standings: Vec<TeamStanding> = ordered.iter().map(|(s, _)| s.clone()).collect();
    let outcomes = tiebreak_outcomes(&standings, order);
    ordered
        .into_iter()
        .zip(outcomes)
        .map(|((standing, rank), outcome)| {
            to_detailed_row(space_id, standing, rank, teams, order, outcome)
        })
        .collect()
}

fn to_detailed_row(
    space_id: &str,
    standing: TeamStanding,
    rank: Rank,
    teams: &[EnrolledTeamInfo],
    order: &TiebreakOrder,
    outcome: RowTiebreak,
) -> DetailedRowVm {
    let team_id = standing.team_id.to_string();
    DetailedRowVm {
        rank: rank.0,
        team_name: resolve_team_name(&team_id, teams),
        team_link: AppRoutes::default().teams.team_detail(space_id, &team_id),
        played: standing.totals.matches_played.0,
        wins: standing.totals.wins.0,
        draws: standing.totals.draws.0,
        losses: standing.totals.losses.0,
        bonus: signed(i64::from(standing.totals.bonus_points.0)),
        total: standing.totals.ranking_points.0,
        tiebreaks: build_tiebreak_cells(&standing, order, outcome),
    }
}

/// Une cellule par critère actif, dans l'ordre.
fn build_tiebreak_cells(
    standing: &TeamStanding,
    order: &TiebreakOrder,
    outcome: RowTiebreak,
) -> Vec<TiebreakCellVm> {
    order
        .criteria()
        .iter()
        .enumerate()
        .map(|(idx, criterion)| TiebreakCellVm {
            value: format_criterion(*criterion, criterion.value_of(&standing.totals)),
            state: cell_state(outcome, idx),
        })
        .collect()
}

/// Règles 21 et 22 rendues visibles : les critères de priorité supérieure au
/// décisif étaient égaux, ceux qui le suivent n'ont pas eu à se prononcer.
fn cell_state(outcome: RowTiebreak, idx: usize) -> CellState {
    match outcome {
        RowTiebreak::Alone => CellState::Neutral,
        RowTiebreak::FullyTied => CellState::Tied,
        RowTiebreak::DecidedBy(k) if idx < k => CellState::Tied,
        RowTiebreak::DecidedBy(k) if idx == k => CellState::Decisive,
        RowTiebreak::DecidedBy(_) => CellState::Neutral,
    }
}

/// Seule la différence de touchdowns peut être négative : elle s'affiche signée.
/// Les autres critères sont des dénombrements, affichés bruts.
fn format_criterion(criterion: TiebreakCriterion, value: i64) -> String {
    match criterion {
        TiebreakCriterion::DiffTd => signed(value),
        _ => value.to_string(),
    }
}

/// Signe explicite, `+0` compris : la valeur se lit comme une contribution et
/// non comme un total autonome. Le moins est le signe **typographique** `−`
/// (U+2212), pas le trait d'union ASCII — c'est ce qu'utilise la maquette.
fn signed(value: i64) -> String {
    match value < 0 {
        true => format!("−{}", value.abs()),
        false => format!("+{value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ranking::io::web::widgets::detailed_standings_widget::DetailedRowVm;
    use crate::app::ranking::domain::ranking_line::{CumulativeTotals, MatchesPlayed, RankingPoints};
    use crate::app::ranking::domain::tiebreak::TiebreakCriterion;
    use crate::app::shared_kernel::team::TeamId;

    /// Les ids d'équipe sont désormais des ULID côté ligne de classement : les
    /// tests les génèrent au lieu d'écrire « t1 », et les DTOs du port
    /// `competitions` (qui restent en `String`) reçoivent leur représentation
    /// textuelle.
    fn line(team_id: &TeamId, points: u32) -> RankingLineRow {
        RankingLineRow {
            team_id: *team_id,
            matches_played: 1,
            wins: 0,
            draws: 0,
            losses: 0,
            ranking_points: points,
            bonus_points: 0,
            td_for: 0,
            td_against: 0,
            casualties: 0,
            fouls: 0,
            completions: 0,
        }
    }

    fn team(team_id: &TeamId, name: &str) -> EnrolledTeamInfo {
        EnrolledTeamInfo { team_id: team_id.to_string(), team_name: name.into() }
    }

    fn group(id: &str, name: &str, team_ids: &[&TeamId]) -> RankingGroupInfo {
        RankingGroupInfo {
            group_id: id.into(),
            group_name: name.into(),
            team_ids: team_ids.iter().map(|t| t.to_string()).collect(),
        }
    }

    /// Une équipe déjà ordonnée et rangée, telle que le domaine la fournit.
    fn ranked(team_id: &TeamId, points: u32, rank: u32) -> (TeamStanding, Rank) {
        let standing = TeamStanding {
            team_id: *team_id,
            totals: CumulativeTotals {
                matches_played: MatchesPlayed(1),
                ranking_points: RankingPoints(points),
                ..CumulativeTotals::ZERO
            },
        };
        (standing, Rank(rank))
    }

    fn empty_order() -> TiebreakOrder {
        TiebreakOrder::empty()
    }

    /// `build_classement_rows` n'ordonne plus et ne numérote plus : il habille.
    /// On lui passe donc volontairement un ordre et des rangs qu'aucun tri par
    /// points ne produirait — s'il triait encore, l'assertion basculerait.
    #[test]
    fn keeps_the_order_and_the_ranks_produced_by_the_domain() {
        let (t1, t2, t3) = (TeamId::new(), TeamId::new(), TeamId::new());
        let teams = vec![team(&t1, "A"), team(&t2, "B"), team(&t3, "C")];
        let ordered = vec![ranked(&t1, 3, 1), ranked(&t2, 9, 2), ranked(&t3, 6, 2)];

        let rows = build_classement_rows("sp1", ordered, &teams);

        assert_eq!(rows.iter().map(|r| r.team_name.as_str()).collect::<Vec<_>>(), vec!["A", "B", "C"]);
        assert_eq!(rows.iter().map(|r| r.rank).collect::<Vec<_>>(), vec![1, 2, 2]);
    }

    #[test]
    fn resolves_team_names_from_enrolled_teams() {
        let t1 = TeamId::new();
        let rows = build_classement_rows("sp1", vec![ranked(&t1, 3, 1)], &[team(&t1, "Les Guerriers")]);
        assert_eq!(rows[0].team_name, "Les Guerriers");
    }

    #[test]
    fn falls_back_to_team_id_when_name_unresolved() {
        let t1 = TeamId::new();
        let rows = build_classement_rows("sp1", vec![ranked(&t1, 3, 1)], &[]);
        assert_eq!(rows[0].team_name, t1.to_string());
    }

    #[test]
    fn builds_team_detail_link_from_space_and_team_id() {
        let t1 = TeamId::new();
        let rows = build_classement_rows("sp1", vec![ranked(&t1, 3, 1)], &[team(&t1, "A")]);
        assert_eq!(rows[0].team_link, AppRoutes::default().teams.team_detail("sp1", &t1.to_string()));
    }

    #[test]
    fn no_group_or_single_group_yields_one_flat_untitled_classement() {
        let (t1, t2) = (TeamId::new(), TeamId::new());
        let teams = vec![team(&t1, "A"), team(&t2, "B")];
        let lines = vec![line(&t1, 3), line(&t2, 9)];

        let none = build_classement_groups("sp1", lines.clone(), &teams, &[], &empty_order());
        let single_group = [group("g1", "Poule unique", &[&t1, &t2])];
        let single = build_classement_groups("sp1", lines, &teams, &single_group, &empty_order());

        for groups in [none, single] {
            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].title, None);
            assert_eq!(groups[0].rows.len(), 2);
        }
    }

    /// Verrou du « par groupe » : les deux poules repartent au rang 1. Si
    /// l'ordonnancement était appliqué avant le découpage, le leader de la
    /// poule 2 hériterait d'un rang global (ici 2) au lieu de 1.
    #[test]
    fn multiple_groups_split_classement_and_rank_independently_per_group() {
        let (t1, t2, t3, t4) = (TeamId::new(), TeamId::new(), TeamId::new(), TeamId::new());
        let teams = vec![team(&t1, "A"), team(&t2, "B"), team(&t3, "C"), team(&t4, "D")];
        let lines = vec![line(&t1, 3), line(&t2, 9), line(&t3, 6), line(&t4, 1)];
        let groups = vec![group("g1", "Poule 1", &[&t1, &t2]), group("g2", "Poule 2", &[&t3, &t4])];

        let result = build_classement_groups("sp1", lines, &teams, &groups, &empty_order());

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, Some("Poule 1".to_string()));
        assert_eq!(result[0].rows[0].team_name, "B");
        assert_eq!(result[0].rows[0].rank, 1);
        assert_eq!(result[1].title, Some("Poule 2".to_string()));
        assert_eq!(result[1].rows[0].team_name, "C");
        assert_eq!(result[1].rows[0].rank, 1);
    }

    /// Le départage configuré traverse tout le chemin de lecture : deux équipes
    /// à égalité de points, seul `nb_td` les sépare. Sans la propagation de
    /// l'ordre jusqu'au domaine, elles resteraient dans leur ordre d'entrée.
    #[test]
    fn the_configured_criterion_orders_teams_tied_on_points() {
        let (poor, prolific) = (TeamId::new(), TeamId::new());
        let teams = vec![team(&poor, "Modeste"), team(&prolific, "Prolifique")];
        let mut lines = vec![line(&poor, 6), line(&prolific, 6)];
        lines[1].td_for = 7;
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd]);

        let result = build_classement_groups("sp1", lines, &teams, &[], &order);

        assert_eq!(result[0].rows[0].team_name, "Prolifique");
        assert_eq!(result[0].rows[0].rank, 1);
        assert_eq!(result[0].rows[1].team_name, "Modeste");
        assert_eq!(result[0].rows[1].rank, 2);
    }

    #[test]
    fn empty_group_keeps_its_slot_with_no_enrolled_teams_state() {
        let (t1, t2) = (TeamId::new(), TeamId::new());
        let teams = vec![team(&t1, "A"), team(&t2, "B")];
        let groups = vec![group("g1", "Poule 1", &[&t1, &t2]), group("g2", "Poule 2", &[])];

        let result = build_classement_groups("sp1", vec![], &teams, &groups, &empty_order());

        assert_eq!(result[1].title, Some("Poule 2".to_string()));
        assert!(!result[1].has_enrolled_teams);
        assert!(result[1].rows.is_empty());
    }

    #[test]
    fn team_not_assigned_to_any_group_appears_in_dedicated_section() {
        let (t1, t2) = (TeamId::new(), TeamId::new());
        let teams = vec![team(&t1, "A"), team(&t2, "B")];
        let groups = vec![group("g1", "Poule 1", &[&t1]), group("g2", "Poule 2", &[])];

        let result = build_classement_groups("sp1", vec![], &teams, &groups, &empty_order());

        assert_eq!(result.len(), 3);
        assert_eq!(result[2].title, Some("Non assignées".to_string()));
        assert!(result[2].has_enrolled_teams);
        assert_eq!(result[2].rows.len(), 0);
    }


    // ── Mise en évidence du critère décisif (carte 223) ──────────────────────

    fn states_of(row: &DetailedRowVm) -> Vec<&'static str> {
        row.tiebreaks.iter().map(|c| c.state.css_class()).collect()
    }

    /// `DecidedBy(1)` : le critère 0 était égal, le 1 a tranché, le 2 n'a pas eu
    /// à se prononcer.
    #[test]
    fn the_decisive_criterion_is_highlighted_and_the_previous_ones_are_greyed() {
        let (t1, t2) = (TeamId::new(), TeamId::new());
        let order = TiebreakOrder::new(vec![
            TiebreakCriterion::NbCas,
            TiebreakCriterion::NbTd,
            TiebreakCriterion::NbFouls,
        ]);
        // Égalité de points, sorties égales, touchdowns différents.
        let mut lines = vec![line(&t1, 6), line(&t2, 6)];
        for l in lines.iter_mut() {
            l.casualties = 3;
        }
        lines[0].td_for = 7;
        lines[1].td_for = 2;

        let groups = build_detailed_groups("sp1", lines, &[team(&t1, "A"), team(&t2, "B")], &[], &order);
        let rows = &groups[0].rows;

        assert_eq!(states_of(&rows[0]), vec!["sd-tied", "sd-decisive", ""]);
        assert_eq!(states_of(&rows[1]), vec!["sd-tied", "sd-decisive", ""]);
    }

    /// Règle 22 : aucune mise en évidence, tout est marqué égal.
    #[test]
    fn a_full_tie_highlights_nothing() {
        let (t1, t2) = (TeamId::new(), TeamId::new());
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd, TiebreakCriterion::NbCas]);
        let lines = vec![line(&t1, 6), line(&t2, 6)];

        let groups = build_detailed_groups("sp1", lines, &[team(&t1, "A"), team(&t2, "B")], &[], &order);

        for row in &groups[0].rows {
            assert_eq!(states_of(row), vec!["sd-tied", "sd-tied"]);
        }
    }

    /// Une équipe seule à son total n'a rien à départager : aucune cellule marquée.
    #[test]
    fn a_team_alone_on_its_total_has_no_marked_cell() {
        let (t1, t2) = (TeamId::new(), TeamId::new());
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd]);
        let lines = vec![line(&t1, 9), line(&t2, 6)];

        let groups = build_detailed_groups("sp1", lines, &[team(&t1, "A"), team(&t2, "B")], &[], &order);

        for row in &groups[0].rows {
            assert_eq!(states_of(row), vec![""]);
        }
    }

    /// La résolution porte sur le classement **de chaque poule** : deux poules
    /// aux mêmes totaux de points ne s'influencent pas. Résolue globalement, la
    /// poule 2 verrait ses deux équipes séparées par le critère alors qu'elles y
    /// sont à égalité.
    #[test]
    fn the_decisive_criterion_is_resolved_within_each_group() {
        let (a1, a2, b1, b2) = (TeamId::new(), TeamId::new(), TeamId::new(), TeamId::new());
        let order = TiebreakOrder::new(vec![TiebreakCriterion::NbTd]);
        let mut lines = vec![line(&a1, 6), line(&a2, 6), line(&b1, 6), line(&b2, 6)];
        // Poule 1 : départagée par les touchdowns. Poule 2 : strictement ex æquo.
        lines[0].td_for = 5;
        lines[1].td_for = 1;
        let teams = vec![team(&a1, "A1"), team(&a2, "A2"), team(&b1, "B1"), team(&b2, "B2")];
        let groups = vec![group("g1", "Poule 1", &[&a1, &a2]), group("g2", "Poule 2", &[&b1, &b2])];

        let result = build_detailed_groups("sp1", lines, &teams, &groups, &order);

        assert_eq!(states_of(&result[0].rows[0]), vec!["sd-decisive"]);
        assert_eq!(states_of(&result[1].rows[0]), vec!["sd-tied"]);
    }

    /// Le formatage est celui arrêté en phase 4 : bonus toujours signé, `+0`
    /// compris ; différence de touchdowns signée ; dénombrements bruts.
    #[test]
    fn values_are_formatted_with_an_explicit_sign_only_where_it_is_meaningful() {
        let t1 = TeamId::new();
        let order = TiebreakOrder::new(vec![TiebreakCriterion::DiffTd, TiebreakCriterion::NbTd]);
        let mut lines = vec![line(&t1, 6)];
        lines[0].td_for = 2;
        lines[0].td_against = 6;

        let groups = build_detailed_groups("sp1", lines, &[team(&t1, "A")], &[], &order);
        let row = &groups[0].rows[0];

        assert_eq!(row.bonus, "+0");
        assert_eq!(row.tiebreaks[0].value, "\u{2212}4", "différence de TD signée, moins typographique");
        assert_eq!(row.tiebreaks[1].value, "2", "dénombrement brut");
    }

    #[test]
    fn no_unassigned_section_when_every_enrolled_team_is_assigned() {
        let t1 = TeamId::new();
        let teams = vec![team(&t1, "A")];
        let groups = vec![group("g1", "Poule 1", &[&t1]), group("g2", "Poule 2", &[])];

        let result = build_classement_groups("sp1", vec![], &teams, &groups, &empty_order());

        assert_eq!(result.len(), 2);
    }
}
