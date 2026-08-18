use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub competition_id: String,
    pub season_id: String,
}

pub struct EnrolledTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub coach_name: String,
    pub roster_name: String,
}

#[derive(Template)]
#[template(path = "widgets/enrolled-teams.html")]
pub struct EnrolledTeamsWidgetTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub teams: Vec<EnrolledTeamVm>,
}

impl IntoResponse for EnrolledTeamsWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("enrolled teams widget render: {e}");
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

/// `space_id` vient du chemin, et **doit** en venir : c'est lui que le template
/// place dans l'URL des actions. Rendu vide, il produisait `/app//team/…`, que
/// `space_scope_middleware` refuse par un `400` sans trace.
pub async fn enrolled_teams_widget(
    Path(space_id): Path<String>,
    Query(params): Query<Params>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let teams = match load_enrolled_teams(&state, &params.season_id).await {
        Ok(t) => t,
        Err(reponse) => return reponse,
    };

    EnrolledTeamsWidgetTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id: params.competition_id,
        season_id: params.season_id,
        teams,
    }
    .into_response()
}

async fn load_enrolled_teams(
    state: &AppState,
    season_id: &str,
) -> Result<Vec<EnrolledTeamVm>, Response> {
    let rows = state
        .teams
        .team_repository
        .find_by_season_and_status(season_id, "Enrolled")
        .await
        .map_err(|e| {
            tracing::error!("enrolled_teams_widget: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    Ok(rows
        .into_iter()
        .map(|r| EnrolledTeamVm {
            team_initials: initials(&r.team_name),
            team_id: r.team_id,
            team_name: r.team_name,
            coach_name: r.coach_name,
            roster_name: r.roster_name,
        })
        .collect())
}
