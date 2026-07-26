use crate::app::ranking::domain::standings::{Rank, TeamStanding, TiebreakOrder};
use crate::app::ranking::io::web::widgets::classement_widget::{ClassementGroupVm, ClassementRowVm};
use crate::app::ranking::ports::{EnrolledTeamInfo, RankingGroupInfo, RankingLineRow};
use crate::app::ranking::use_cases::standings_service::build_ordered_standings;
use crate::app::routes::AppRoutes;
use std::collections::HashSet;

/// Un classement par poule si la saison en compte 2 ou plus (BR : chaque
/// poule a son propre classement, un tri global n'aurait pas de sens dès
/// qu'il y a plusieurs groupes) ; sinon un classement unique sur toute la
/// saison, comportement inchangé. Les équipes enrôlées non assignées à une
/// poule sont regroupées à part plutôt que de disparaître silencieusement.
pub fn build_classement_groups(
    space_id: &str,
    lines: Vec<RankingLineRow>,
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
    order: &TiebreakOrder,
) -> Vec<ClassementGroupVm> {
    if groups.len() <= 1 {
        return vec![build_group_vm(space_id, None, None, &lines, teams, order)];
    }

    let mut result: Vec<ClassementGroupVm> = groups
        .iter()
        .map(|g| {
            let title = Some(g.group_name.clone());
            build_group_vm(space_id, title, Some(&g.team_ids), &lines, teams, order)
        })
        .collect();

    if let Some(unassigned) = build_unassigned_group(space_id, &lines, teams, groups, order) {
        result.push(unassigned);
    }
    result
}

fn build_unassigned_group(
    space_id: &str,
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
    order: &TiebreakOrder,
) -> Option<ClassementGroupVm> {
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
    Some(build_group_vm(space_id, title, Some(&unassigned_ids), lines, teams, order))
}

/// `team_ids: None` = pas de filtrage (classement à plat, saison sans poule).
///
/// Le filtrage précède l'ordonnancement : chaque poule est un classement
/// autonome dont les rangs repartent à 1. Ordonner avant de découper donnerait
/// des rangs globaux — le leader de la poule 2 pourrait afficher un rang 3.
fn build_group_vm(
    space_id: &str,
    title: Option<String>,
    team_ids: Option<&[String]>,
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
    order: &TiebreakOrder,
) -> ClassementGroupVm {
    let group_teams: Vec<EnrolledTeamInfo> = teams
        .iter()
        .filter(|t| team_ids.is_none_or(|ids| ids.contains(&t.team_id)))
        .cloned()
        .collect();
    let group_lines: Vec<RankingLineRow> = lines
        .iter()
        .filter(|l| team_ids.is_none_or(|ids| ids.contains(&l.team_id.to_string())))
        .cloned()
        .collect();
    ClassementGroupVm {
        title,
        has_enrolled_teams: !group_teams.is_empty(),
        rows: build_classement_rows(space_id, build_ordered_standings(group_lines, order), &group_teams),
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

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn no_unassigned_section_when_every_enrolled_team_is_assigned() {
        let t1 = TeamId::new();
        let teams = vec![team(&t1, "A")];
        let groups = vec![group("g1", "Poule 1", &[&t1]), group("g2", "Poule 2", &[])];

        let result = build_classement_groups("sp1", vec![], &teams, &groups, &empty_order());

        assert_eq!(result.len(), 2);
    }
}
