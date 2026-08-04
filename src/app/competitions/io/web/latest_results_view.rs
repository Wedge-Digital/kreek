use crate::app::auth::domain::user::User;
use crate::app::competitions::domain::match_day_repository_port::LatestResultDto;
use crate::app::shared_kernel::bloodbowl::ids::CompetitionId;
use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use std::collections::HashSet;

pub struct LatestResultVm {
    pub competition_name: String,
    pub round_name: String,
    pub home_name: String,
    pub home_score: u32,
    pub home_is_winner: bool,
    pub away_name: String,
    pub away_score: u32,
    pub away_is_winner: bool,
    pub date: String,
    pub report_url: Option<String>,
}

/// Autorisation à cliquer sur un résultat pour naviguer vers le rapport de
/// match — même règle que `resultats_view::ResultAuthorization`, étendue à
/// plusieurs compétitions puisque ce widget couvre tout l'espace.
pub struct LatestResultsAuthorization {
    is_space_admin: bool,
    admin_competition_ids: HashSet<String>,
    my_team_ids: HashSet<String>,
}

impl LatestResultsAuthorization {
    pub fn allows(&self, competition_id: &str, home_team_id: &str, away_team_id: &str) -> bool {
        self.is_space_admin
            || self.admin_competition_ids.contains(competition_id)
            || self.my_team_ids.contains(home_team_id)
            || self.my_team_ids.contains(away_team_id)
    }
}

pub async fn compute_authorization(
    state: &AppState,
    user: &User,
    space_id: &SpaceId,
    rows: &[LatestResultDto],
) -> LatestResultsAuthorization {
    let is_space_admin = matches!(
        state
            .competitions
            .space_member_port
            .find_member_profile(&user.id, space_id)
            .await,
        Some(SpaceProfile::SpaceAdmin)
    );
    if is_space_admin {
        return LatestResultsAuthorization {
            is_space_admin: true,
            admin_competition_ids: HashSet::new(),
            my_team_ids: HashSet::new(),
        };
    }

    let admin_competition_ids = admin_competition_ids(state, user, rows).await;
    let my_team_ids = my_team_ids(state, user, rows).await;
    LatestResultsAuthorization {
        is_space_admin: false,
        admin_competition_ids,
        my_team_ids,
    }
}

async fn admin_competition_ids(
    state: &AppState,
    user: &User,
    rows: &[LatestResultDto],
) -> HashSet<String> {
    let user_id_str = user.id.to_string();
    let coach_name_str = user.coach_name.clone().into_inner();
    let distinct_ids: HashSet<&str> = rows.iter().map(|r| r.competition_id.as_str()).collect();

    let mut admin_ids = HashSet::new();
    for cid in distinct_ids {
        let Ok(competition_id) = CompetitionId::try_new(cid) else {
            continue;
        };
        if let Ok(Some(info)) = state
            .competitions
            .competition_repository
            .find_base_info(&competition_id)
            .await
        {
            if info.admin_ids.contains(&user_id_str) || info.admin_names.contains(&coach_name_str) {
                admin_ids.insert(cid.to_string());
            }
        }
    }
    admin_ids
}

async fn my_team_ids(state: &AppState, user: &User, rows: &[LatestResultDto]) -> HashSet<String> {
    let user_id_str = user.id.to_string();
    let distinct_seasons: HashSet<&str> = rows.iter().map(|r| r.season_id.as_str()).collect();

    let mut team_ids = HashSet::new();
    for season_id in distinct_seasons {
        let enrolled = state
            .competitions
            .team_info_port
            .find_enrolled_teams(season_id)
            .await
            .unwrap_or_default();
        team_ids.extend(
            enrolled
                .into_iter()
                .filter(|t| t.coach_id == user_id_str)
                .map(|t| t.team_id),
        );
    }
    team_ids
}

pub fn to_latest_result_vm(
    row: LatestResultDto,
    authz: &LatestResultsAuthorization,
) -> LatestResultVm {
    let home_score = row.home_score.unwrap_or(0) as u32;
    let away_score = row.away_score.unwrap_or(0) as u32;
    let report_url = if authz.allows(&row.competition_id, &row.home_team_id, &row.away_team_id) {
        row.match_report_url
    } else {
        None
    };
    LatestResultVm {
        competition_name: row.competition_name,
        round_name: row.round_name,
        home_name: row.home_team_name,
        home_score,
        home_is_winner: home_score > away_score,
        away_name: row.away_team_name,
        away_score,
        away_is_winner: away_score > home_score,
        date: format_date(row.published_at),
        report_url,
    }
}

fn format_date(date: Option<time::OffsetDateTime>) -> String {
    let Some(date) = date else {
        return String::new();
    };
    let months = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    format!(
        "{} {} {}",
        date.day(),
        months[date.month() as usize - 1],
        date.year(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authz(
        is_space_admin: bool,
        admin_competition_ids: &[&str],
        my_team_ids: &[&str],
    ) -> LatestResultsAuthorization {
        LatestResultsAuthorization {
            is_space_admin,
            admin_competition_ids: admin_competition_ids
                .iter()
                .map(|s| s.to_string())
                .collect(),
            my_team_ids: my_team_ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn space_admin_allows_any_match() {
        let a = authz(true, &[], &[]);
        assert!(a.allows("any-comp", "any-home", "any-away"));
    }

    #[test]
    fn competition_admin_allows_matches_of_that_competition() {
        let a = authz(false, &["comp-a"], &[]);
        assert!(a.allows("comp-a", "team-x", "team-y"));
    }

    #[test]
    fn coach_of_home_team_is_allowed() {
        let a = authz(false, &[], &["team-a"]);
        assert!(a.allows("comp-a", "team-a", "team-b"));
    }

    #[test]
    fn coach_of_away_team_is_allowed() {
        let a = authz(false, &[], &["team-b"]);
        assert!(a.allows("comp-a", "team-a", "team-b"));
    }

    #[test]
    fn neither_admin_nor_coach_is_not_allowed() {
        let a = authz(false, &["comp-x"], &["team-c"]);
        assert!(!a.allows("comp-a", "team-a", "team-b"));
    }

    fn sample_row(home_score: Option<i32>, away_score: Option<i32>) -> LatestResultDto {
        LatestResultDto {
            pairing_id: "p1".into(),
            season_id: "s1".into(),
            competition_id: "c1".into(),
            competition_name: "Ligue A".into(),
            round_name: "Journée 1".into(),
            home_team_id: "home".into(),
            home_team_name: "Home".into(),
            home_score,
            away_team_id: "away".into(),
            away_team_name: "Away".into(),
            away_score,
            match_report_url: Some("/report".into()),
            published_at: None,
        }
    }

    #[test]
    fn home_wins_when_score_is_higher() {
        let vm = to_latest_result_vm(sample_row(Some(3), Some(1)), &authz(true, &[], &[]));
        assert!(vm.home_is_winner);
        assert!(!vm.away_is_winner);
    }

    #[test]
    fn away_wins_when_score_is_higher() {
        let vm = to_latest_result_vm(sample_row(Some(1), Some(3)), &authz(true, &[], &[]));
        assert!(!vm.home_is_winner);
        assert!(vm.away_is_winner);
    }

    #[test]
    fn draw_highlights_neither_side() {
        let vm = to_latest_result_vm(sample_row(Some(2), Some(2)), &authz(true, &[], &[]));
        assert!(!vm.home_is_winner);
        assert!(!vm.away_is_winner);
    }

    #[test]
    fn report_url_hidden_when_not_authorized() {
        let vm = to_latest_result_vm(sample_row(Some(1), Some(0)), &authz(false, &[], &[]));
        assert_eq!(vm.report_url, None);
    }
}
