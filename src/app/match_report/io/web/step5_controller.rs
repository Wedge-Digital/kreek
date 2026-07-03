use crate::app::auth::auth_backend::AuthSession;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{FanFactorMod, MatchGain};
use crate::app::match_report::use_cases::record_post_match_use_case;
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
#[template(path = "step5.html")]
pub struct Step5Template {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub form_action: String,
    pub back_url: String,
    pub home_team_name: String,
    pub away_team_name: String,
    pub home_initials: String,
    pub away_initials: String,
    pub home_logo_url: Option<String>,
    pub away_logo_url: Option<String>,
    pub home_score: u8,
    pub away_score: u8,
    pub home_cas: u8,
    pub away_cas: u8,
    pub home_gain: u32,
    pub away_gain: u32,
    pub home_fan_mod: i8,
    pub away_fan_mod: i8,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub already_recorded: bool,
}

impl IntoResponse for Step5Template {
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

pub async fn get_step5(
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
            tracing::error!("get_step5 find_by_id {match_report_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let (pm, already_recorded) = match mr_state {
        MatchReportState::PreMatch(pm) => (pm, false),
        MatchReportState::ReadyToPublish(rtp) => {
            return build_step5_from_rtp(rtp, space_id, match_report_id, &state).await;
        }
        MatchReportState::Draft(_) => {
            let url = AppRoutes::default()
                .match_report
                .edit_match_report(&space_id, &match_report_id);
            return Redirect::to(&url).into_response();
        }
        MatchReportState::Cancelled(_) => return StatusCode::GONE.into_response(),
        MatchReportState::Published(_) => return StatusCode::CONFLICT.into_response(),
    };

    let _ = already_recorded;
    let home_id = pm.home_team_id.to_string();
    let away_id = pm.away_team_id.to_string();
    let (home_score, away_score) = pm.compute_score();
    let (home_cas, away_cas) = pm.compute_cas();
    let (home_gain_sug, away_gain_sug) = pm.suggest_gains();

    let (home_info, away_info) = tokio::join!(
        state.match_report.team_data.find_team_info(&home_id),
        state.match_report.team_data.find_team_info(&away_id),
    );
    let home_info = home_info.unwrap_or_default();
    let away_info = away_info.unwrap_or_default();

    build_template(
        &space_id, &match_report_id,
        &state, &home_info.team_name, &away_info.team_name,
        home_info.logo_url, away_info.logo_url,
        home_score, away_score, home_cas, away_cas,
        home_gain_sug, away_gain_sug, 0, 0,
        None, None, false,
    )
    .into_response()
}

async fn build_step5_from_rtp(
    rtp: crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish,
    space_id: String,
    match_report_id: String,
    state: &AppState,
) -> Response {
    let home_id = rtp.home_team_id.to_string();
    let away_id = rtp.away_team_id.to_string();
    let (home_score, away_score) = count_tds(&rtp.home_actions, &rtp.away_actions);
    let (home_cas, away_cas) = count_cas(&rtp.home_actions, &rtp.away_actions);

    let (home_info, away_info) = tokio::join!(
        state.match_report.team_data.find_team_info(&home_id),
        state.match_report.team_data.find_team_info(&away_id),
    );
    let home_info = home_info.unwrap_or_default();
    let away_info = away_info.unwrap_or_default();

    build_template(
        &space_id, &match_report_id,
        state, &home_info.team_name, &away_info.team_name,
        home_info.logo_url, away_info.logo_url,
        home_score, away_score, home_cas, away_cas,
        rtp.home_gain.into_inner(), rtp.away_gain.into_inner(),
        rtp.home_fan_mod.into_inner(), rtp.away_fan_mod.into_inner(),
        rtp.summary_title, rtp.summary_body, true,
    )
    .into_response()
}

fn count_tds(
    home_actions: &[crate::app::match_report::domain::value_objects::MatchAction],
    away_actions: &[crate::app::match_report::domain::value_objects::MatchAction],
) -> (u8, u8) {
    use crate::app::match_report::domain::value_objects::MatchActionType;
    let home = home_actions.iter().filter(|a| matches!(a.action, MatchActionType::Touchdown)).count() as u8;
    let away = away_actions.iter().filter(|a| matches!(a.action, MatchActionType::Touchdown)).count() as u8;
    (home, away)
}

fn count_cas(
    home_actions: &[crate::app::match_report::domain::value_objects::MatchAction],
    away_actions: &[crate::app::match_report::domain::value_objects::MatchAction],
) -> (u8, u8) {
    use crate::app::match_report::domain::value_objects::MatchActionType;
    let home = home_actions.iter().filter(|a| matches!(a.action, MatchActionType::Sortie)).count() as u8;
    let away = away_actions.iter().filter(|a| matches!(a.action, MatchActionType::Sortie)).count() as u8;
    (home, away)
}

#[allow(clippy::too_many_arguments)]
fn build_template(
    space_id: &str,
    match_report_id: &str,
    state: &AppState,
    home_team_name: &str,
    away_team_name: &str,
    home_logo_url: Option<String>,
    away_logo_url: Option<String>,
    home_score: u8,
    away_score: u8,
    home_cas: u8,
    away_cas: u8,
    home_gain: u32,
    away_gain: u32,
    home_fan_mod: i8,
    away_fan_mod: i8,
    summary_title: Option<String>,
    summary_body: Option<String>,
    already_recorded: bool,
) -> Step5Template {
    let _ = state;
    let routes = AppRoutes::default();
    Step5Template {
        form_action: routes.match_report.step5(space_id, match_report_id),
        back_url: routes.match_report.step4(space_id, match_report_id),
        app_routes: Default::default(),
        space_id: space_id.to_string(),
        match_report_id: match_report_id.to_string(),
        home_initials: initials_from(home_team_name),
        away_initials: initials_from(away_team_name),
        home_team_name: home_team_name.to_string(),
        away_team_name: away_team_name.to_string(),
        home_logo_url,
        away_logo_url,
        home_score,
        away_score,
        home_cas,
        away_cas,
        home_gain,
        away_gain,
        home_fan_mod,
        away_fan_mod,
        summary_title,
        summary_body,
        already_recorded,
    }
}

// ── POST ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RecordPostMatchForm {
    pub home_gain: u32,
    pub away_gain: u32,
    pub home_fan_mod: i8,
    pub away_fan_mod: i8,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
}

pub async fn post_step5(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RecordPostMatchForm>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let mr_id = match MatchReportId::try_new(&match_report_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let home_gain = match MatchGain::try_new(form.home_gain) {
        Ok(g) => g,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let away_gain = match MatchGain::try_new(form.away_gain) {
        Ok(g) => g,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let home_fan_mod = match FanFactorMod::try_new(form.home_fan_mod) {
        Ok(m) => m,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    let away_fan_mod = match FanFactorMod::try_new(form.away_fan_mod) {
        Ok(m) => m,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let summary_title = form.summary_title.filter(|s| !s.trim().is_empty());
    let summary_body = form.summary_body.filter(|s| !s.trim().is_empty());

    let cmd = record_post_match_use_case::RecordPostMatchCommand {
        match_report_id: mr_id,
        home_gain,
        away_gain,
        home_fan_mod,
        away_fan_mod,
        summary_title,
        summary_body,
        recorded_by: user.id,
    };

    match record_post_match_use_case::execute(cmd, state.match_report.match_report_repo.as_ref()).await {
        Ok(record_post_match_use_case::RecordPostMatchOutcome::Success) => {
            let url = AppRoutes::default().match_report.recap(&space_id, &match_report_id);
            Redirect::to(&url).into_response()
        }
        Err(record_post_match_use_case::RecordPostMatchError::NotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(record_post_match_use_case::RecordPostMatchError::NotInCompatibleState) => {
            StatusCode::CONFLICT.into_response()
        }
        Err(e) => {
            tracing::error!("post_step5: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
