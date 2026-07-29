use crate::app::match_report::domain::value_objects::TeamSide;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::collections::HashSet;

pub struct TurnButtonVm {
    pub turn: u8,
    pub has_action: bool,
}

#[derive(Template)]
#[template(path = "turn-selector-widget.html")]
pub struct TurnSelectorTemplate {
    pub turns: Vec<TurnButtonVm>,
}

impl IntoResponse for TurnSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_turn_selector_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    render_turn_selector(&mr_id, TeamSide::Home, &state).await
}

pub async fn get_turn_selector_step4(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    render_turn_selector(&mr_id, TeamSide::Away, &state).await
}

async fn render_turn_selector(mr_id: &str, side: TeamSide, state: &AppState) -> Response {
    let actions = match state
        .match_report
        .match_report_repo
        .find_actions_by_match_and_side(mr_id, side)
        .await
    {
        Ok(a) => a,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let turns_with_action: HashSet<i16> = actions.iter().map(|a| a.turn_number).collect();
    let turns = (1u8..=16)
        .map(|t| TurnButtonVm {
            turn: t,
            has_action: turns_with_action.contains(&(t as i16)),
        })
        .collect();
    TurnSelectorTemplate { turns }.into_response()
}
