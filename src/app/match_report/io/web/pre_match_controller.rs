use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::D3Roll;
use crate::app::match_report::use_cases::record_fan_factor_use_case;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::MatchReportId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;

// ── Template ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "pre-match.html")]
pub struct PreMatchTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_initials: String,
    pub away_initials: String,
    pub home_coach_name: String,
    pub away_coach_name: String,
    pub home_roster_name: String,
    pub away_roster_name: String,
    pub home_team_context_url: String,
    pub away_team_context_url: String,
    pub form_action: String,
    pub fan_factor_already_recorded: bool,
}

impl IntoResponse for PreMatchTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

// ── GET ───────────────────────────────────────────────────────────────────────

pub async fn get_pre_match(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let Some(_user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mr_state = match state
        .match_report
        .match_report_repo
        .find_by_id(&match_report_id)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get_pre_match find_by_id {match_report_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let pre_match = match mr_state {
        MatchReportState::PreMatch(pm) => pm,
        MatchReportState::Draft(_) => {
            let url = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &match_report_id);
            return Redirect::to(&url).into_response();
        }
        MatchReportState::Cancelled(_) => return StatusCode::GONE.into_response(),
    };

    let home_id = pre_match.home_team_id.to_string();
    let away_id = pre_match.away_team_id.to_string();
    let fan_factor_already_recorded =
        pre_match.home_fan_roll.is_some() && pre_match.away_fan_roll.is_some();

    let (home_info, away_info) = tokio::join!(
        state.match_report.team_data.find_team_info(&home_id),
        state.match_report.team_data.find_team_info(&away_id),
    );
    let home_info = home_info.unwrap_or_default();
    let away_info = away_info.unwrap_or_default();

    let base_url = AppRoutes::default()
        .teams
        .team_match_context_json(&space_id);
    let home_team_context_url = format!("{}?team_id={}", base_url, home_id);
    let away_team_context_url = format!("{}?team_id={}", base_url, away_id);
    let form_action = AppRoutes::default()
        .match_report
        .step2(&space_id, &match_report_id);

    PreMatchTemplate {
        app_routes: Default::default(),
        space_id,
        match_report_id,
        home_initials: initials_from(&home_info.team_name),
        away_initials: initials_from(&away_info.team_name),
        home_team_name: home_info.team_name,
        away_team_name: away_info.team_name,
        home_coach_name: home_info.coach_name,
        away_coach_name: away_info.coach_name,
        home_roster_name: home_info.roster_name,
        away_roster_name: away_info.roster_name,
        home_team_id: home_id,
        away_team_id: away_id,
        home_team_context_url,
        away_team_context_url,
        form_action,
        fan_factor_already_recorded,
    }
    .into_response()
}

// ── POST ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RecordFanFactorForm {
    pub home_fan_roll: u8,
    pub away_fan_roll: u8,
}

pub async fn post_pre_match(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordFanFactorForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let home_fan_roll = match D3Roll::try_new(form.home_fan_roll) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let away_fan_roll = match D3Roll::try_new(form.away_fan_roll) {
        Ok(r) => r,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let mr_id = match MatchReportId::try_new(&match_report_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = record_fan_factor_use_case::RecordFanFactorCommand {
        match_report_id: mr_id,
        home_fan_roll,
        away_fan_roll,
        recorded_by: user.id,
    };

    match record_fan_factor_use_case::execute(cmd, state.match_report.match_report_repo.as_ref())
        .await
    {
        Ok(_) => {
            let url = AppRoutes::default()
                .match_report
                .step2(&space_id, &match_report_id);
            Redirect::to(&url).into_response()
        }
        Err(record_fan_factor_use_case::RecordFanFactorError::NotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(record_fan_factor_use_case::RecordFanFactorError::NotInPreMatchPhase) => {
            StatusCode::CONFLICT.into_response()
        }
        Err(e) => {
            tracing::error!("post_pre_match: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
