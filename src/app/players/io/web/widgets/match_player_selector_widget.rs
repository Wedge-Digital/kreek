use crate::app::players::domain::player::TeamId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct PlayerRowVm {
    pub player_id: String,
    pub name:      String,
    pub position:  String,
    pub jersey:    Option<i16>,
}

#[derive(Template)]
#[template(path = "match-player-selector-widget.html")]
pub struct MatchPlayerSelectorTemplate {
    pub players: Vec<PlayerRowVm>,
}

pub async fn match_player_selector_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    let team_id = TeamId(team_id);
    let players = match state.players.projection_repository.find_by_team_id(&team_id).await {
        Ok(p) => p,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let vms = players
        .into_iter()
        .map(|p| PlayerRowVm {
            player_id: p.player_id,
            name:      p.personal_name,
            position:  p.position_name,
            jersey:    p.jersey,
        })
        .collect();
    let template = MatchPlayerSelectorTemplate { players: vms };
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
