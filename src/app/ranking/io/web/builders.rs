use crate::app::ranking::io::web::widgets::classement_widget::{ClassementGroupVm, ClassementRowVm};
use crate::app::ranking::ports::{EnrolledTeamInfo, RankingGroupInfo, RankingLineRow};
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
) -> Vec<ClassementGroupVm> {
    if groups.len() <= 1 {
        return vec![build_group_vm(space_id, None, None, &lines, teams)];
    }

    let mut result: Vec<ClassementGroupVm> = groups
        .iter()
        .map(|g| build_group_vm(space_id, Some(g.group_name.clone()), Some(&g.team_ids), &lines, teams))
        .collect();

    if let Some(unassigned) = build_unassigned_group(space_id, &lines, teams, groups) {
        result.push(unassigned);
    }
    result
}

fn build_unassigned_group(
    space_id: &str,
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
    groups: &[RankingGroupInfo],
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
    Some(build_group_vm(space_id, Some("Non assignées".to_string()), Some(&unassigned_ids), lines, teams))
}

/// `team_ids: None` = pas de filtrage (classement à plat, saison sans poule).
fn build_group_vm(
    space_id: &str,
    title: Option<String>,
    team_ids: Option<&[String]>,
    lines: &[RankingLineRow],
    teams: &[EnrolledTeamInfo],
) -> ClassementGroupVm {
    let group_teams: Vec<EnrolledTeamInfo> = teams
        .iter()
        .filter(|t| team_ids.is_none_or(|ids| ids.contains(&t.team_id)))
        .cloned()
        .collect();
    let group_lines: Vec<RankingLineRow> = lines
        .iter()
        .filter(|l| team_ids.is_none_or(|ids| ids.contains(&l.team_id)))
        .cloned()
        .collect();
    ClassementGroupVm {
        title,
        has_enrolled_teams: !group_teams.is_empty(),
        rows: build_classement_rows(space_id, group_lines, &group_teams),
    }
}

/// Combine les lignes de classement (repository) et les équipes inscrites
/// (port `competitions`, pour les noms) — trie par points décroissants et
/// assigne le rang à la construction (jamais stocké sur la ligne elle-même).
pub fn build_classement_rows(
    space_id: &str,
    lines: Vec<RankingLineRow>,
    teams: &[EnrolledTeamInfo],
) -> Vec<ClassementRowVm> {
    let mut rows: Vec<ClassementRowVm> = lines
        .into_iter()
        .map(|line| {
            let team_name = teams
                .iter()
                .find(|t| t.team_id == line.team_id)
                .map(|t| t.team_name.clone())
                .unwrap_or_else(|| line.team_id.clone());
            let team_link = AppRoutes::default().teams.team_detail(space_id, &line.team_id);
            ClassementRowVm {
                rank: 0,
                team_name,
                team_link,
                played: line.matches_played,
                wins: line.wins,
                draws: line.draws,
                losses: line.losses,
                points: line.ranking_points,
            }
        })
        .collect();

    rows.sort_by(|a, b| b.points.cmp(&a.points));
    for (idx, row) in rows.iter_mut().enumerate() {
        row.rank = (idx + 1) as u32;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(team_id: &str, points: u32) -> RankingLineRow {
        RankingLineRow {
            team_id: team_id.into(),
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

    fn team(team_id: &str, name: &str) -> EnrolledTeamInfo {
        EnrolledTeamInfo { team_id: team_id.into(), team_name: name.into() }
    }

    #[test]
    fn sorts_by_points_descending_and_assigns_rank() {
        let lines = vec![line("t1", 3), line("t2", 9), line("t3", 6)];
        let teams = vec![team("t1", "A"), team("t2", "B"), team("t3", "C")];

        let rows = build_classement_rows("sp1", lines, &teams);

        assert_eq!(rows[0].team_name, "B");
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[1].team_name, "C");
        assert_eq!(rows[1].rank, 2);
        assert_eq!(rows[2].team_name, "A");
        assert_eq!(rows[2].rank, 3);
    }

    #[test]
    fn resolves_team_names_from_enrolled_teams() {
        let rows = build_classement_rows("sp1", vec![line("t1", 3)], &[team("t1", "Les Guerriers")]);
        assert_eq!(rows[0].team_name, "Les Guerriers");
    }

    #[test]
    fn falls_back_to_team_id_when_name_unresolved() {
        let rows = build_classement_rows("sp1", vec![line("t1", 3)], &[]);
        assert_eq!(rows[0].team_name, "t1");
    }

    #[test]
    fn builds_team_detail_link_from_space_and_team_id() {
        let rows = build_classement_rows("sp1", vec![line("t1", 3)], &[team("t1", "A")]);
        assert_eq!(rows[0].team_link, AppRoutes::default().teams.team_detail("sp1", "t1"));
    }

    fn group(id: &str, name: &str, team_ids: &[&str]) -> RankingGroupInfo {
        RankingGroupInfo {
            group_id: id.into(),
            group_name: name.into(),
            team_ids: team_ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn no_group_or_single_group_yields_one_flat_untitled_classement() {
        let teams = vec![team("t1", "A"), team("t2", "B")];
        let lines = vec![line("t1", 3), line("t2", 9)];

        let none = build_classement_groups("sp1", lines.clone(), &teams, &[]);
        let single = build_classement_groups("sp1", lines, &teams, &[group("g1", "Poule unique", &["t1", "t2"])]);

        for groups in [none, single] {
            assert_eq!(groups.len(), 1);
            assert_eq!(groups[0].title, None);
            assert_eq!(groups[0].rows.len(), 2);
        }
    }

    #[test]
    fn multiple_groups_split_classement_and_rank_independently_per_group() {
        let teams = vec![team("t1", "A"), team("t2", "B"), team("t3", "C"), team("t4", "D")];
        let lines = vec![line("t1", 3), line("t2", 9), line("t3", 6), line("t4", 1)];
        let groups = vec![group("g1", "Poule 1", &["t1", "t2"]), group("g2", "Poule 2", &["t3", "t4"])];

        let result = build_classement_groups("sp1", lines, &teams, &groups);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].title, Some("Poule 1".to_string()));
        assert_eq!(result[0].rows[0].team_name, "B");
        assert_eq!(result[0].rows[0].rank, 1);
        assert_eq!(result[1].title, Some("Poule 2".to_string()));
        assert_eq!(result[1].rows[0].team_name, "C");
        assert_eq!(result[1].rows[0].rank, 1);
    }

    #[test]
    fn empty_group_keeps_its_slot_with_no_enrolled_teams_state() {
        let teams = vec![team("t1", "A"), team("t2", "B")];
        let groups = vec![group("g1", "Poule 1", &["t1", "t2"]), group("g2", "Poule 2", &[])];

        let result = build_classement_groups("sp1", vec![], &teams, &groups);

        assert_eq!(result[1].title, Some("Poule 2".to_string()));
        assert!(!result[1].has_enrolled_teams);
        assert!(result[1].rows.is_empty());
    }

    #[test]
    fn team_not_assigned_to_any_group_appears_in_dedicated_section() {
        let teams = vec![team("t1", "A"), team("t2", "B")];
        let groups = vec![group("g1", "Poule 1", &["t1"]), group("g2", "Poule 2", &[])];

        let result = build_classement_groups("sp1", vec![], &teams, &groups);

        assert_eq!(result.len(), 3);
        assert_eq!(result[2].title, Some("Non assignées".to_string()));
        assert!(result[2].has_enrolled_teams);
        assert_eq!(result[2].rows.len(), 0);
    }

    #[test]
    fn no_unassigned_section_when_every_enrolled_team_is_assigned() {
        let teams = vec![team("t1", "A")];
        let groups = vec![group("g1", "Poule 1", &["t1"]), group("g2", "Poule 2", &[])];

        let result = build_classement_groups("sp1", vec![], &teams, &groups);

        assert_eq!(result.len(), 2);
    }
}
