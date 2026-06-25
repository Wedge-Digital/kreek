use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::MatchReportOrigin;
use crate::app::match_report::io::web::view_models::*;
use crate::app::match_report::use_cases::{
    create_match_report_use_case, update_match_selection_use_case,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::{
    CompetitionId, MatchReportId, RoundId, SeasonId, SpaceId,
};
use crate::app::shared_kernel::team::TeamId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Templates ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "match-selection.html")]
pub struct MatchSelectionTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub widget_url: String,
    pub teams_url: String,
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
pub struct TeamsQuery {
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

fn build_widget_url(space_id: &str) -> String {
    AppRoutes::default()
        .competitions
        .competition_widget(space_id)
        + "?show_rounds=true"
}

fn build_widget_url_prefilled(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    round_id: &str,
) -> String {
    format!(
        "{}?show_rounds=true&competition_id={}&season_id={}&round_id={}",
        AppRoutes::default().competitions.competition_widget(space_id),
        competition_id,
        season_id,
        round_id,
    )
}

// ── Handlers GET ─────────────────────────────────────────────────────────────

pub async fn from_pairing(
    Path((space_id, pairing_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mr_id = match state
        .match_report
        .match_report_repo
        .find_id_by_pairing(&pairing_id)
        .await
    {
        Ok(Some(id)) => id,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("from_pairing find_id_by_pairing {pairing_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let url = AppRoutes::default()
        .match_report
        .edit_match_report(&space_id, &mr_id);
    Redirect::to(&url).into_response()
}

pub async fn new_match_report(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    let Some(_user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let form_action = AppRoutes::default()
        .match_report
        .new_match_report(&space_id);
    let teams_url = AppRoutes::default()
        .match_report
        .teams_fragment(&space_id);

    MatchSelectionTemplate {
        app_routes: Default::default(),
        widget_url: build_widget_url(&space_id),
        teams_url,
        space_id,
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
            let teams_url = AppRoutes::default()
                .match_report
                .teams_fragment(&space_id);

            MatchSelectionTemplate {
                app_routes: Default::default(),
                widget_url: build_widget_url_prefilled(
                    &space_id, &comp_id, &season_id, &round_id,
                ),
                teams_url,
                space_id,
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
        MatchReportState::Cancelled(_) => {
            StatusCode::GONE.into_response()
        }
    }
}

pub async fn teams_fragment(
    auth_session: AuthSession,
    Query(q): Query<TeamsQuery>,
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

// ── POST form ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateMatchReportForm {
    pub competition_id: String,
    pub season_id: String,
    pub round_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

// ── Handlers POST ────────────────────────────────────────────────────────────

pub async fn create_match_report(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
    Form(form): Form<CreateMatchReportForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let cmd = create_match_report_use_case::CreateMatchReportCommand {
        space_id: match SpaceId::try_new(&space_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        competition_id: match CompetitionId::try_new(&form.competition_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        season_id: match SeasonId::try_new(&form.season_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        round_id: match RoundId::try_new(&form.round_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        home_team_id: match TeamId::try_new(&form.home_team_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        away_team_id: match TeamId::try_new(&form.away_team_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        created_by: user.id,
        origin: MatchReportOrigin::Manual,
        pairing_id: None,
    };

    match create_match_report_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
    )
    .await
    {
        Ok(mr_id) => {
            let url = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &mr_id.to_string());
            Redirect::to(&url).into_response()
        }
        Err(create_match_report_use_case::CreateMatchReportError::SameTeam) => {
            StatusCode::BAD_REQUEST.into_response()
        }
        Err(e) => {
            tracing::error!("create_match_report: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update_match_selection(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<CreateMatchReportForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let cmd = update_match_selection_use_case::UpdateMatchSelectionCommand {
        match_report_id: match MatchReportId::try_new(&match_report_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        home_team_id: match TeamId::try_new(&form.home_team_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        away_team_id: match TeamId::try_new(&form.away_team_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        confirmed_by: user.id,
    };

    match update_match_selection_use_case::execute(
        cmd,
        state.match_report.match_report_repo.as_ref(),
        state.match_report.team_data.as_ref(),
        &state.app_event_bus,
    )
    .await
    {
        Ok(_mr_id) => {
            let url = format!(
                "/app/{}/match-report/{}/step2",
                space_id, match_report_id
            );
            Redirect::to(&url).into_response()
        }
        Err(update_match_selection_use_case::UpdateMatchSelectionError::SameTeam) => {
            StatusCode::BAD_REQUEST.into_response()
        }
        Err(update_match_selection_use_case::UpdateMatchSelectionError::TeamNotAvailable(tid)) => {
            tracing::warn!("update_match_selection: team {tid} not available");
            StatusCode::CONFLICT.into_response()
        }
        Err(e) => {
            tracing::error!("update_match_selection: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
