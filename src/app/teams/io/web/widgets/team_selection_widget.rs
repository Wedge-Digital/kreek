use crate::app::teams::routes::Routes as TeamsRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};

// ── JSON endpoint ───────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct TeamJson {
    pub id: String,
    pub name: String,
    pub coach_name: String,
    pub coach_id: String,
    pub roster_name: String,
    pub tv: String,
}

#[derive(Deserialize, Default)]
pub struct TeamSelectionJsonQuery {
    pub season_id: Option<String>,
}

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

pub async fn get_team_selection_json(
    Path(_space_id): Path<String>,
    Query(query): Query<TeamSelectionJsonQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let season_id = query.season_id.unwrap_or_default();

    let teams = state
        .teams
        .team_repository
        .find_enrolled_for_season(&season_id)
        .await
        .unwrap_or_default();

    let result: Vec<TeamJson> = teams
        .into_iter()
        .filter(|t| t.game_phase.as_deref() == Some("ReadyToPlay"))
        .map(|t| TeamJson {
            id: t.team_id,
            name: t.team_name,
            coach_name: t.coach_name,
            coach_id: t.coach_id,
            roster_name: t.roster_name,
            tv: format_tv(t.team_value),
        })
        .collect();

    Json(result)
}

// ── Widget HTML ─────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/team-selection.html")]
pub struct TeamSelectionWidgetTemplate {
    pub routes: TeamsRoutes,
    pub space_id: String,
    pub season_id: String,
    pub selected_home: String,
    pub selected_away: String,
}

impl IntoResponse for TeamSelectionWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Deserialize, Default)]
pub struct TeamSelectionWidgetQuery {
    pub season_id: Option<String>,
    pub selected_home: Option<String>,
    pub selected_away: Option<String>,
}

pub async fn get_team_selection_widget(
    Path(space_id): Path<String>,
    Query(query): Query<TeamSelectionWidgetQuery>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    TeamSelectionWidgetTemplate {
        routes: Default::default(),
        space_id,
        season_id: query.season_id.unwrap_or_default(),
        selected_home: query.selected_home.unwrap_or_default(),
        selected_away: query.selected_away.unwrap_or_default(),
    }
    .into_response()
}
