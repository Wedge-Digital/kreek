use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::TeamSide;
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "actions-step.html")]
pub struct ActionsStepTemplate {
    pub app_routes:               AppRoutes,
    pub space_id:                 String,
    pub match_report_id:          String,
    pub team_side:                String,
    pub turn_selector_url:        String,
    pub player_selector_url:      String,
    pub temp_player_selector_url: String,
    pub action_panel_url:         String,
    pub action_log_url:           String,
}

impl IntoResponse for ActionsStepTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    get_step(space_id, mr_id, TeamSide::Home, state).await
}

pub async fn get_step4(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    get_step(space_id, mr_id, TeamSide::Away, state).await
}

async fn get_step(
    space_id: String,
    mr_id: String,
    side: TeamSide,
    state: AppState,
) -> Response {
    let state_opt = match state.match_report.match_report_repo.find_by_id(&mr_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let pm = match state_opt {
        Some(MatchReportState::PreMatch(pm)) => pm,
        Some(_) | None => return StatusCode::NOT_FOUND.into_response(),
    };

    let routes = AppRoutes::default();
    let team_id = match side {
        TeamSide::Home => pm.home_team_id.to_string(),
        TeamSide::Away => pm.away_team_id.to_string(),
    };
    let (turn_selector_url, temp_player_selector_url, action_panel_url, action_log_url) = match side {
        TeamSide::Home => (
            routes.match_report.step3_turn_selector(&space_id, &mr_id),
            routes.match_report.step3_temp_players(&space_id, &mr_id),
            routes.match_report.step3_action_panel(&space_id, &mr_id),
            routes.match_report.step3_action_log(&space_id, &mr_id),
        ),
        TeamSide::Away => (
            routes.match_report.step4_turn_selector(&space_id, &mr_id),
            routes.match_report.step4_temp_players(&space_id, &mr_id),
            routes.match_report.step4_action_panel(&space_id, &mr_id),
            routes.match_report.step4_action_log(&space_id, &mr_id),
        ),
    };
    let player_selector_url = routes.players.match_player_selector(&space_id, &team_id);
    let team_side_label = match side { TeamSide::Home => "home", TeamSide::Away => "away" };

    ActionsStepTemplate {
        app_routes: routes,
        space_id,
        match_report_id: mr_id,
        team_side: team_side_label.to_string(),
        turn_selector_url,
        player_selector_url,
        temp_player_selector_url,
        action_panel_url,
        action_log_url,
    }.into_response()
}
