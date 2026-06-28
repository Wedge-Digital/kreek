use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{TeamSide, TempPlayerKind};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct TempPlayerRowVm {
    pub temp_player_id: String,
    pub label:          String,
}

#[derive(Template)]
#[template(path = "temp-player-selector-widget.html")]
pub struct TempPlayerSelectorTemplate {
    pub players: Vec<TempPlayerRowVm>,
}

impl IntoResponse for TempPlayerSelectorTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_temp_players_step3(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    render_temp_players(&mr_id, TeamSide::Home, &state).await
}

pub async fn get_temp_players_step4(
    Path((space_id, mr_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    let _ = space_id;
    render_temp_players(&mr_id, TeamSide::Away, &state).await
}

async fn render_temp_players(mr_id: &str, side: TeamSide, state: &AppState) -> Response {
    let state_opt = match state.match_report.match_report_repo.find_by_id(mr_id).await {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let pm = match state_opt {
        Some(MatchReportState::PreMatch(pm)) => pm,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let players: Vec<TempPlayerRowVm> = pm
        .temp_players_for(side)
        .iter()
        .map(|tp| TempPlayerRowVm {
            temp_player_id: tp.id.0.clone(),
            label: tp.display_name.clone().unwrap_or_else(|| kind_label(&tp.kind)),
        })
        .collect();
    TempPlayerSelectorTemplate { players }.into_response()
}

fn kind_label(kind: &TempPlayerKind) -> String {
    match kind {
        TempPlayerKind::StarPlayer { ref_uid, .. } => ref_uid.clone(),
        TempPlayerKind::Mercenary { .. } => "Mercenaire".to_string(),
        TempPlayerKind::Journalier { .. } => "Journalier".to_string(),
    }
}
