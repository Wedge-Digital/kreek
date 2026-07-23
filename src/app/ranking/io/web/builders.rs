use crate::app::ranking::io::web::widgets::classement_widget::ClassementRowVm;
use crate::app::ranking::ports::{EnrolledTeamInfo, RankingLineRow};

/// Combine les lignes de classement (repository) et les équipes inscrites
/// (port `competitions`, pour les noms) — trie par points décroissants et
/// assigne le rang à la construction (jamais stocké sur la ligne elle-même).
pub fn build_classement_rows(
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
            ClassementRowVm {
                rank: 0,
                team_name,
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
        RankingLineRow { team_id: team_id.into(), matches_played: 1, wins: 0, draws: 0, losses: 0, ranking_points: points }
    }

    fn team(team_id: &str, name: &str) -> EnrolledTeamInfo {
        EnrolledTeamInfo { team_id: team_id.into(), team_name: name.into() }
    }

    #[test]
    fn sorts_by_points_descending_and_assigns_rank() {
        let lines = vec![line("t1", 3), line("t2", 9), line("t3", 6)];
        let teams = vec![team("t1", "A"), team("t2", "B"), team("t3", "C")];

        let rows = build_classement_rows(lines, &teams);

        assert_eq!(rows[0].team_name, "B");
        assert_eq!(rows[0].rank, 1);
        assert_eq!(rows[1].team_name, "C");
        assert_eq!(rows[1].rank, 2);
        assert_eq!(rows[2].team_name, "A");
        assert_eq!(rows[2].rank, 3);
    }

    #[test]
    fn resolves_team_names_from_enrolled_teams() {
        let rows = build_classement_rows(vec![line("t1", 3)], &[team("t1", "Les Guerriers")]);
        assert_eq!(rows[0].team_name, "Les Guerriers");
    }

    #[test]
    fn falls_back_to_team_id_when_name_unresolved() {
        let rows = build_classement_rows(vec![line("t1", 3)], &[]);
        assert_eq!(rows[0].team_name, "t1");
    }
}
