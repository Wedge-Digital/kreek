use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::io::web::view_models::*;
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "match-selection.html")]
pub struct MatchSelectionTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub seasons_url: String,
    pub rounds_url: String,
    pub teams_url: String,
    pub competitions: Vec<CompetitionOptionVm>,
    pub seasons: Vec<SeasonOptionVm>,
    pub rounds: Vec<RoundOptionVm>,
    pub teams: Vec<TeamOptionVm>,
    pub selected: Option<SelectedMatchVm>,
    pub is_admin: bool,
    pub is_prefilled: bool,
    pub error_message: Option<String>,
    pub form_action: String,
}

impl IntoResponse for MatchSelectionTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "fragments/season-options.html")]
pub struct SeasonOptionsFragment {
    pub rounds_url: String,
    pub seasons: Vec<SeasonOptionVm>,
}

impl IntoResponse for SeasonOptionsFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "fragments/round-options.html")]
pub struct RoundOptionsFragment {
    pub teams_url: String,
    pub rounds: Vec<RoundOptionVm>,
}

impl IntoResponse for RoundOptionsFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "fragments/team-options.html")]
pub struct TeamOptionsFragment {
    pub teams: Vec<TeamOptionVm>,
    pub is_admin: bool,
}

impl IntoResponse for TeamOptionsFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Query params ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CompetitionQuery {
    pub competition_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SeasonQuery {
    pub season_id: Option<String>,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn format_tv(kpo: u32) -> String {
    if kpo >= 1000 {
        let major = kpo / 1000;
        let minor = (kpo % 1000) / 100;
        if minor > 0 {
            format!("{} {}00 kPo", major, minor)
        } else {
            format!("{} 000 kPo", major)
        }
    } else {
        format!("{} kPo", kpo)
    }
}

async fn is_admin(
    state: &AppState,
    competition_id: Option<&str>,
    coach_id: &str,
) -> bool {
    if let Some(comp_id) = competition_id {
        if let Ok(true) = state
            .match_report
            .competition_data
            .is_competition_admin(comp_id, coach_id)
            .await
        {
            return true;
        }
    }
    false
}

// ── Handlers GET ─────────────────────────────────────────────────────────────

pub async fn new_match_report(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let coach_id = user.id.to_string();

    let competitions = state
        .match_report
        .competition_data
        .list_competitions_with_active_season(&space_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|c| CompetitionOptionVm {
            id: c.competition_id,
            name: c.name,
            selected: false,
        })
        .collect();

    let form_action = AppRoutes::default()
        .match_report
        .new_match_report(&space_id);

    let mr_routes = AppRoutes::default().match_report;
    MatchSelectionTemplate {
        app_routes: Default::default(),
        seasons_url: mr_routes.seasons_fragment(&space_id),
        rounds_url: mr_routes.rounds_fragment(&space_id),
        teams_url: mr_routes.teams_fragment(&space_id),
        space_id,
        competitions,
        seasons: vec![],
        rounds: vec![],
        teams: vec![],
        selected: None,
        is_admin: false,
        is_prefilled: false,
        error_message: None,
        form_action,
    }
    .into_response()
}

pub async fn edit_match_report(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let coach_id = user.id.to_string();

    let mr_state = match state
        .match_report
        .match_report_repo
        .find_by_id(&match_report_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("edit_match_report find_by_id {match_report_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match mr_state {
        MatchReportState::Draft(draft) => {
            let comp_id = draft.competition_id.to_string();
            let season_id = draft.season_id.to_string();
            let round_id = draft.round_id.to_string();

            let admin = is_admin(&state, Some(&comp_id), &coach_id).await;

            let competitions = state
                .match_report
                .competition_data
                .list_competitions_with_active_season(&space_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|c| CompetitionOptionVm {
                    selected: c.competition_id == comp_id,
                    id: c.competition_id,
                    name: c.name,
                })
                .collect();

            let seasons = state
                .match_report
                .competition_data
                .list_seasons(&comp_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|s| SeasonOptionVm {
                    selected: s.season_id == season_id,
                    id: s.season_id,
                    name: s.name,
                })
                .collect();

            let rounds = state
                .match_report
                .competition_data
                .list_rounds(&season_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|r| RoundOptionVm {
                    selected: r.round_id == round_id,
                    id: r.round_id,
                    name: r.name,
                    dates: match (r.date_start, r.date_end) {
                        (Some(s), Some(e)) => format!("{} → {}", s, e),
                        (Some(s), None) => s,
                        _ => String::new(),
                    },
                })
                .collect();

            let home_id = draft.home_team_id.to_string();
            let away_id = draft.away_team_id.to_string();

            let teams = state
                .match_report
                .team_data
                .list_enrolled_teams(&season_id)
                .await
                .unwrap_or_default()
                .into_iter()
                .filter(|t| t.game_phase.as_deref() == Some("ReadyToPlay"))
                .map(|t| TeamOptionVm {
                    is_own_team: t.coach_id == coach_id,
                    id: t.team_id,
                    name: t.team_name,
                    coach_name: t.coach_name,
                    roster_name: t.roster_name,
                    tv: format_tv(t.team_value),
                })
                .collect();

            let form_action = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &match_report_id);

            let mr_routes2 = AppRoutes::default().match_report;
            MatchSelectionTemplate {
                app_routes: Default::default(),
                seasons_url: mr_routes2.seasons_fragment(&space_id),
                rounds_url: mr_routes2.rounds_fragment(&space_id),
                teams_url: mr_routes2.teams_fragment(&space_id),
                space_id,
                competitions,
                seasons,
                rounds,
                teams,
                selected: Some(SelectedMatchVm {
                    match_report_id: match_report_id.clone(),
                    home_team_id: home_id,
                    away_team_id: away_id,
                }),
                is_admin: admin,
                is_prefilled: true,
                error_message: None,
                form_action,
            }
            .into_response()
        }
        MatchReportState::PreMatch(_pm) => {
            let url = format!(
                "/app/{}/match-report/{}/step2",
                space_id, match_report_id
            );
            Redirect::to(&url).into_response()
        }
    }
}

pub async fn seasons_fragment(
    Path(space_id): Path<String>,
    Query(q): Query<CompetitionQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let competition_id = q.competition_id.unwrap_or_default();
    let seasons = state
        .match_report
        .competition_data
        .list_seasons(&competition_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .map(|(i, s)| SeasonOptionVm {
            selected: i == 0,
            id: s.season_id,
            name: s.name,
        })
        .collect();

    let rounds_url = AppRoutes::default()
        .match_report
        .rounds_fragment(&space_id);

    SeasonOptionsFragment {
        rounds_url,
        seasons,
    }
    .into_response()
}

pub async fn rounds_fragment(
    Path(space_id): Path<String>,
    Query(q): Query<SeasonQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let season_id = q.season_id.unwrap_or_default();
    let rounds = state
        .match_report
        .competition_data
        .list_rounds(&season_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| RoundOptionVm {
            selected: false,
            id: r.round_id,
            name: r.name,
            dates: match (r.date_start, r.date_end) {
                (Some(s), Some(e)) => format!("{} → {}", s, e),
                (Some(s), None) => s,
                _ => String::new(),
            },
        })
        .collect();

    let teams_url = AppRoutes::default()
        .match_report
        .teams_fragment(&space_id);

    RoundOptionsFragment {
        teams_url,
        rounds,
    }
    .into_response()
}

pub async fn teams_fragment(
    auth_session: AuthSession,
    Query(q): Query<SeasonQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let coach_id = auth_session
        .user
        .map(|u| u.id.to_string())
        .unwrap_or_default();

    let season_id = q.season_id.unwrap_or_default();
    let teams = state
        .match_report
        .team_data
        .list_enrolled_teams(&season_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|t| t.game_phase.as_deref() == Some("ReadyToPlay"))
        .map(|t| TeamOptionVm {
            is_own_team: t.coach_id == coach_id,
            id: t.team_id,
            name: t.team_name,
            coach_name: t.coach_name,
            roster_name: t.roster_name,
            tv: format_tv(t.team_value),
        })
        .collect();

    TeamOptionsFragment {
        teams,
        is_admin: false,
    }
    .into_response()
}
