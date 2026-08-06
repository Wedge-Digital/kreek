use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub season_id: String,
    pub space_id: String,
}

pub struct CompetitionTeamCardVm {
    pub team_id: String,
    pub initials: String,
    pub name: String,
    pub logo: Option<String>,
    pub roster: String,
    pub coach: String,
    pub tv: u32,
    pub link: String,
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[derive(Template)]
#[template(path = "widgets/competition-teams.html")]
pub struct CompetitionTeamsWidgetTemplate {
    pub teams: Vec<CompetitionTeamCardVm>,
}

impl IntoResponse for CompetitionTeamsWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("competition teams widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn competition_teams_widget(
    Query(params): Query<Params>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rows = match state
        .teams
        .team_repository
        .find_enrolled_for_season(&params.season_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("competition_teams_widget: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let app_routes = AppRoutes::default();
    let teams = rows
        .into_iter()
        .map(|r| CompetitionTeamCardVm {
            link: app_routes.teams.team_detail(&params.space_id, &r.team_id),
            initials: initials(&r.team_name),
            team_id: r.team_id,
            name: r.team_name,
            logo: r.logo_url,
            roster: r.roster_name,
            coach: r.coach_name,
            tv: r.team_value,
        })
        .collect();

    CompetitionTeamsWidgetTemplate { teams }.into_response()
}
