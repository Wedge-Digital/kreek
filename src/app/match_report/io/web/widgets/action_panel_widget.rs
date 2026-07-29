use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ActionPanelParams {
    pub turn: u8,
    pub player_id: String,
    pub player_type: String,
}

#[derive(Template)]
#[template(path = "action-panel-widget.html")]
pub struct ActionPanelTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub team_side: String,
    pub turn: u8,
    pub player_id: String,
    pub player_type: String,
    pub post_url: String,
}

impl IntoResponse for ActionPanelTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_action_panel_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    Query(params): Query<ActionPanelParams>,
    State(state): State<AppState>,
) -> Response {
    let _ = state;
    let routes = AppRoutes::default();
    let post_url = routes.match_report.step3_post_action(&space_id, &mr_id);
    ActionPanelTemplate {
        app_routes: routes,
        space_id,
        match_report_id: mr_id,
        team_side: "home".into(),
        turn: params.turn,
        player_id: params.player_id,
        player_type: params.player_type,
        post_url,
    }
    .into_response()
}

pub async fn get_action_panel_step4(
    Path((space_id, mr_id)): Path<(String, String)>,
    Query(params): Query<ActionPanelParams>,
    State(state): State<AppState>,
) -> Response {
    let _ = state;
    let routes = AppRoutes::default();
    let post_url = routes.match_report.step4_post_action(&space_id, &mr_id);
    ActionPanelTemplate {
        app_routes: routes,
        space_id,
        match_report_id: mr_id,
        team_side: "away".into(),
        turn: params.turn,
        player_id: params.player_id,
        player_type: params.player_type,
        post_url,
    }
    .into_response()
}
