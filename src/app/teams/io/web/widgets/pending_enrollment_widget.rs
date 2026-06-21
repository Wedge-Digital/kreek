use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub competition_id: String,
    pub season_id: String,
}

pub struct PendingTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub coach_name: String,
    pub roster_name: String,
}

#[derive(Template)]
#[template(path = "widgets/pending-enrollments.html")]
pub struct PendingEnrollmentWidgetTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub teams: Vec<PendingTeamVm>,
}

impl IntoResponse for PendingEnrollmentWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("pending enrollment widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

pub async fn pending_enrollment_widget(
    Query(params): Query<Params>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let rows = match state
        .teams
        .team_repository
        .find_by_season_and_status(&params.season_id, "PendingEnrollment")
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("pending_enrollment_widget: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let teams = rows
        .into_iter()
        .map(|r| PendingTeamVm {
            team_initials: initials(&r.team_name),
            team_id: r.team_id,
            team_name: r.team_name,
            coach_name: r.coach_name,
            roster_name: r.roster_name,
        })
        .collect();

    PendingEnrollmentWidgetTemplate {
        app_routes: AppRoutes::default(),
        space_id: String::new(),
        competition_id: params.competition_id,
        season_id: params.season_id,
        teams,
    }
    .into_response()
}
