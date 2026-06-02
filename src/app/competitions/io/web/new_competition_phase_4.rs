use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::routes::Routes;
use crate::app::competitions::use_cases::save_competition_invitations::{
    execute, SaveCompetitionInvitationsCommand, SaveCompetitionInvitationsError,
};
use crate::app::shared_kernel::common_types::SeasonId;
use crate::app::spaces::routes::Routes as SpaceRoutes;
use crate::state::AppState;
use crate::web::routes::Routes as WebRoutes;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;

#[derive(Template)]
#[template(path = "new-competition-phase-4.html")]
pub struct NewCompetitionPhase4Template {
    pub web_routes: WebRoutes,
    pub competition_routes: Routes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub existing_invitations_json: String,
    pub coach_search_widget_url: String,
}

impl IntoResponse for NewCompetitionPhase4Template {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_4(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let existing_invitations_json = match state
        .competitions
        .season_repository
        .find_invitations(&sid)
        .await
    {
        Ok(Some(inv)) => serde_json::to_string(&inv).unwrap_or_else(|e| {
            tracing::error!("phase 4 serialize error for {season_id}: {e}");
            "null".to_string()
        }),
        Ok(None) => "null".to_string(),
        Err(e) => {
            tracing::error!("phase 4 find_invitations error for {season_id}: {e}");
            "null".to_string()
        }
    };

    NewCompetitionPhase4Template {
        web_routes: WebRoutes,
        competition_routes: Routes,
        coach_search_widget_url: SpaceRoutes.coach_search_widget(&space_id),
        space_id,
        competition_id,
        season_id,
        existing_invitations_json,
    }
    .into_response()
}

pub async fn post_competition_invitations(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(invitations): Json<CompetitionInvitations>,
) -> impl IntoResponse {
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Identifiant de saison invalide.").into_response()
        }
    };

    let cmd = SaveCompetitionInvitationsCommand {
        season_id: sid,
        invitations,
    };

    match execute(cmd, state.competitions.season_repository.as_ref()).await {
        Ok(()) => Response::builder()
            .header(
                "HX-Redirect",
                Routes.new_competition_validation(&space_id, &competition_id, &season_id),
            )
            .body(Body::empty())
            .unwrap(),

        Err(SaveCompetitionInvitationsError::SeasonNotFound) => {
            (StatusCode::NOT_FOUND, "Saison introuvable.").into_response()
        }

        Err(SaveCompetitionInvitationsError::Database(_)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Erreur interne, veuillez réessayer.",
        )
            .into_response(),
    }
}
