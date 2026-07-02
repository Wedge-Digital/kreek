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
    pub home_team_name:           String,
    pub away_team_name:           String,
    pub home_initials:            String,
    pub away_initials:            String,
    pub home_logo_url:            Option<String>,
    pub away_logo_url:            Option<String>,
    pub turn_selector_url:        String,
    pub player_selector_url:      String,
    pub temp_player_selector_url: String,
    pub action_panel_url:         String,
    pub action_log_url:           String,
    pub back_url:                 String,
    pub next_url:                 String,
    pub next_label:               String,
    pub next_disabled:            bool,
}

fn initials_from(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
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
    let home_id = pm.home_team_id.to_string();
    let away_id = pm.away_team_id.to_string();
    let team_id = match side { TeamSide::Home => home_id.clone(), TeamSide::Away => away_id.clone() };
    let (home_info, away_info) = tokio::join!(
        state.match_report.team_data.find_team_info(&home_id),
        state.match_report.team_data.find_team_info(&away_id),
    );
    let home_info = home_info.unwrap_or_default();
    let away_info = away_info.unwrap_or_default();
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
    let back_url = match side {
        TeamSide::Home => routes.match_report.step2(&space_id, &mr_id),
        TeamSide::Away => routes.match_report.step3(&space_id, &mr_id),
    };
    let (next_url, next_label, next_disabled) = match side {
        TeamSide::Home => (routes.match_report.step4(&space_id, &mr_id), "Équipe extérieure", false),
        TeamSide::Away => (routes.match_report.step5(&space_id, &mr_id), "Après-match", false),
    };

    ActionsStepTemplate {
        app_routes: routes,
        space_id,
        match_report_id: mr_id,
        team_side: team_side_label.to_string(),
        home_initials:  initials_from(&home_info.team_name),
        away_initials:  initials_from(&away_info.team_name),
        home_logo_url:  home_info.logo_url,
        away_logo_url:  away_info.logo_url,
        home_team_name: home_info.team_name,
        away_team_name: away_info.team_name,
        turn_selector_url,
        player_selector_url,
        temp_player_selector_url,
        action_panel_url,
        action_log_url,
        back_url,
        next_url,
        next_label: next_label.to_string(),
        next_disabled,
    }.into_response()
}
