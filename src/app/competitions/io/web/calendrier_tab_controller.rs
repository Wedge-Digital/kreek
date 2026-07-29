use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::competitions::domain::match_day_repository_port::PairingDisplayDto;
use crate::app::competitions::io::web::competition_detail::{full_page, load_page_base};
use crate::app::competitions::io::web::resultats_view::{
    compute_authorization, ResultAuthorization,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::identity::ids::SpaceId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct TabCursorQuery {
    pub cursor: Option<i32>,
}

pub struct MatchCalendrierVm {
    pub home_name: String,
    pub home_logo: Option<String>,
    pub home_initials: String,
    pub away_name: String,
    pub away_logo: Option<String>,
    pub away_initials: String,
    /// Lien vers la saisie du rapport de match du pairing, quand l'utilisateur
    /// est autorisé à le démarrer (mêmes règles que l'onglet résultats).
    pub report_url: Option<String>,
}

pub struct JourneeCalendrierVm {
    pub label: String,
    pub date_range: String,
    pub match_count: usize,
    pub matches: Vec<MatchCalendrierVm>,
}

#[derive(Template)]
#[template(path = "competition-tab-calendrier.html")]
pub struct CalendrierTabTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub journees: Vec<JourneeCalendrierVm>,
    pub next_cursor: Option<i32>,
    pub is_initial: bool,
}

impl IntoResponse for CalendrierTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(h) => Html(h).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_calendrier_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    Query(query): Query<TabCursorQuery>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let rows = match load_calendrier(&state, &season_id, query.cursor).await {
        Ok(r) => r,
        Err(r) => return r,
    };

    let authz =
        match build_authorization(&state, &user, &space_id, &competition_id, &season_id).await {
            Ok(a) => a,
            Err(r) => return r,
        };

    let (journees, next_cursor) = build_journees(rows, 3, &space_id, &authz);

    if headers.contains_key("hx-request") {
        let is_initial = query.cursor.is_none();
        return fragment(
            space_id,
            competition_id,
            season_id,
            journees,
            next_cursor,
            is_initial,
        );
    }

    render_full_page(
        space_id,
        competition_id,
        season_id,
        journees,
        next_cursor,
        &state,
    )
    .await
}

async fn build_authorization(
    state: &AppState,
    user: &User,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
) -> Result<ResultAuthorization, Response> {
    let space = SpaceId::try_new(space_id).map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    let competition = CompetitionId::try_new(competition_id)
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;

    Ok(compute_authorization(state, user, &space, &competition, season_id).await)
}

fn fragment(
    space_id: String,
    competition_id: String,
    season_id: String,
    journees: Vec<JourneeCalendrierVm>,
    next_cursor: Option<i32>,
    is_initial: bool,
) -> Response {
    CalendrierTabTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        journees,
        next_cursor,
        is_initial,
    }
    .into_response()
}

async fn load_calendrier(
    state: &AppState,
    season_id: &str,
    cursor: Option<i32>,
) -> Result<Vec<PairingDisplayDto>, Response> {
    state
        .competitions
        .match_day_repository
        .list_calendrier(season_id, cursor, 3)
        .await
        .map_err(|e| {
            tracing::error!("calendrier_tab: list_calendrier: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}

fn build_journees(
    rows: Vec<PairingDisplayDto>,
    max_rounds: usize,
    space_id: &str,
    authz: &ResultAuthorization,
) -> (Vec<JourneeCalendrierVm>, Option<i32>) {
    let mut by_round: BTreeMap<i32, (String, String, String, Vec<MatchCalendrierVm>)> =
        BTreeMap::new();
    for row in rows {
        let date_range = format_date_range(
            &row.round_day_type,
            row.round_date_start.as_deref(),
            row.round_date_end.as_deref(),
        );
        let entry = by_round.entry(row.round_position).or_insert_with(|| {
            (
                row.round_name.clone(),
                date_range,
                row.round_day_type.clone(),
                Vec::new(),
            )
        });
        entry.3.push(to_calendrier_vm(row, space_id, authz));
    }

    let mut journees: Vec<(i32, JourneeCalendrierVm)> = by_round
        .into_iter()
        .map(|(pos, (label, date_range, _, matches))| {
            let match_count = matches.len();
            (
                pos,
                JourneeCalendrierVm {
                    label,
                    date_range,
                    match_count,
                    matches,
                },
            )
        })
        .collect();

    journees.sort_by_key(|(pos, _)| *pos);
    journees.truncate(max_rounds);

    let next_cursor = if journees.len() == max_rounds {
        journees.last().map(|(pos, _)| *pos)
    } else {
        None
    };

    (journees.into_iter().map(|(_, j)| j).collect(), next_cursor)
}

fn format_date_range(day_type: &str, start: Option<&str>, end: Option<&str>) -> String {
    match day_type {
        "fixed_date" => start.unwrap_or("").to_string(),
        "time_frame" => match (start, end) {
            (Some(s), Some(e)) => format!("{} – {}", s, e),
            (Some(s), None) => s.to_string(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// Le rapport de match d'un pairing existe dès la création de celui-ci
/// (`pairing_created_listener` du BC match_report) — la ligne du calendrier
/// pointe donc directement vers sa saisie, qui reprend là où elle en est.
fn to_calendrier_vm(
    row: PairingDisplayDto,
    space_id: &str,
    authz: &ResultAuthorization,
) -> MatchCalendrierVm {
    let report_url = if authz.allows(&row.home_team_id, &row.away_team_id) {
        Some(
            AppRoutes::default()
                .match_report
                .from_pairing(space_id, &row.pairing_id),
        )
    } else {
        None
    };

    MatchCalendrierVm {
        report_url,
        home_name: row.home_team_name,
        home_logo: row.home_logo_url,
        home_initials: row.home_initials,
        away_name: row.away_team_name,
        away_logo: row.away_logo_url,
        away_initials: row.away_initials,
    }
}

async fn render_full_page(
    space_id: String,
    competition_id: String,
    season_id: String,
    journees: Vec<JourneeCalendrierVm>,
    next_cursor: Option<i32>,
    state: &AppState,
) -> Response {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let pb = match load_page_base(&cid, &sid, state, &competition_id).await {
        Ok(v) => v,
        Err(r) => return r,
    };
    let _ = (journees, next_cursor);
    full_page(
        pb,
        space_id,
        competition_id,
        season_id,
        "calendrier",
        false,
        vec![],
        vec![],
        vec![],
        vec![],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn sample_row() -> PairingDisplayDto {
        PairingDisplayDto {
            pairing_id: "pairing-1".to_string(),
            round_id: "round-1".to_string(),
            round_name: "Journée 1".to_string(),
            round_position: 1,
            round_date_start: None,
            round_date_end: None,
            round_day_type: "rest".to_string(),
            home_team_id: "team-a".to_string(),
            home_team_name: "Les A".to_string(),
            home_roster_name: "Humains".to_string(),
            home_coach_name: "Alice".to_string(),
            home_logo_url: None,
            home_initials: "LA".to_string(),
            away_team_id: "team-b".to_string(),
            away_team_name: "Les B".to_string(),
            away_roster_name: "Orques".to_string(),
            away_coach_name: "Bob".to_string(),
            away_logo_url: None,
            away_initials: "LB".to_string(),
            match_status: "upcoming".to_string(),
            home_score: None,
            away_score: None,
            home_casualties: None,
            away_casualties: None,
            match_report_url: None,
        }
    }

    fn coach_of(team_ids: &[&str]) -> ResultAuthorization {
        ResultAuthorization {
            is_admin: false,
            my_team_ids: team_ids
                .iter()
                .map(|t| t.to_string())
                .collect::<HashSet<_>>(),
        }
    }

    #[test]
    fn admin_gets_a_link_to_the_match_report() {
        let vm = to_calendrier_vm(
            sample_row(),
            "space-1",
            &ResultAuthorization::unrestricted(),
        );

        assert_eq!(
            vm.report_url.as_deref(),
            Some("/app/space-1/match-report/pairing/pairing-1")
        );
    }

    #[test]
    fn coach_of_one_of_the_two_teams_gets_a_link() {
        let vm = to_calendrier_vm(sample_row(), "space-1", &coach_of(&["team-b"]));

        assert_eq!(
            vm.report_url.as_deref(),
            Some("/app/space-1/match-report/pairing/pairing-1")
        );
    }

    #[test]
    fn coach_of_neither_team_gets_no_link() {
        let vm = to_calendrier_vm(sample_row(), "space-1", &coach_of(&["team-c"]));

        assert_eq!(vm.report_url, None);
    }

    #[test]
    fn format_fixed_date() {
        assert_eq!(
            format_date_range("fixed_date", Some("12 juin"), None),
            "12 juin"
        );
    }

    #[test]
    fn format_time_frame_with_end() {
        assert_eq!(
            format_date_range("time_frame", Some("10 juin"), Some("12 juin")),
            "10 juin – 12 juin"
        );
    }

    #[test]
    fn format_time_frame_without_end() {
        assert_eq!(
            format_date_range("time_frame", Some("10 juin"), None),
            "10 juin"
        );
    }

    #[test]
    fn format_rest_returns_empty() {
        assert_eq!(format_date_range("rest", None, None), "");
    }
}
