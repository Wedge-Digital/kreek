use crate::app::references::domain::port::IReferenceRepository;
use crate::app::references::io::web::pickers::{build_star_player_items, StarPlayerPickerItem};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct StarPlayerPickerParams {
    #[serde(default)]
    pub instance_id: String,
    #[serde(default)]
    pub selected: String,
    #[serde(default)]
    pub select_all: bool,
}

pub async fn star_player_picker_controller(
    State(state): State<AppState>,
    Query(params): Query<StarPlayerPickerParams>,
) -> impl IntoResponse {
    let star_players = build_star_player_items(state.references.repository.as_ref());

    let selected: Vec<String> = if params.select_all {
        star_players.iter().map(|s| s.uid.clone()).collect()
    } else {
        params
            .selected
            .split(',')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect()
    };

    StarPlayerPickerTemplate {
        routes: AppRoutes::default(),
        star_players,
        instance_id: params.instance_id,
        selected_json: serde_json::to_string(&selected).unwrap_or_else(|_| "[]".to_string()),
    }
    .into_response()
}

#[derive(Template)]
#[template(path = "widgets/star-player-picker.html")]
pub struct StarPlayerPickerTemplate {
    pub routes: AppRoutes,
    pub star_players: Vec<StarPlayerPickerItem>,
    pub instance_id: String,
    pub selected_json: String,
}

impl IntoResponse for StarPlayerPickerTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
