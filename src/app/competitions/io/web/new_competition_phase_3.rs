use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use crate::app::competitions::domain::competition_structure::CompetitionStructure;
use crate::app::competitions::routes::Routes;
use crate::app::competitions::use_cases::save_competition_structure::{SaveCompetitionStructureCommand, SaveCompetitionStructureError, execute};
use crate::app::shared_kernel::common_types::CompetitionId;
use crate::state::AppState;
use crate::web::app_layout::AppLayout;

#[derive(Template)]
#[template(path = "new-competition-phase-3.html")]
pub struct NewCompetitionPhase3Template {
    pub competition_routes:      Routes,
    pub space_id:                String,
    pub competition_id:          String,
    pub existing_structure_json: String,
}

impl IntoResponse for NewCompetitionPhase3Template {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_)   => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn get_new_competition_phase_3(
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let existing_structure_json = state.competitions.competition_repository
        .find_structure(&cid)
        .await
        .ok()
        .flatten()
        .and_then(|s| serde_json::to_string(&s).ok())
        .unwrap_or_else(|| "null".to_string());

    let tmpl = NewCompetitionPhase3Template {
        competition_routes: Routes,
        space_id,
        competition_id,
        existing_structure_json,
    };
    if headers.contains_key("hx-request") {
        tmpl.into_response()
    } else {
        let content = tmpl.render().unwrap_or_default();
        AppLayout { content, routes: Default::default() }.into_response()
    }
}

pub async fn post_competition_structure(
    Path((space_id, competition_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Json(structure): Json<CompetitionStructure>,
) -> impl IntoResponse {
    let cid = match CompetitionId::try_new(&competition_id) {
        Ok(id) => id,
        Err(_) => return (StatusCode::BAD_REQUEST, "Identifiant de compétition invalide.").into_response(),
    };

    let cmd = SaveCompetitionStructureCommand { competition_id: cid, structure };

    match execute(cmd, state.competitions.competition_repository.as_ref()).await {
        Ok(()) => Response::builder()
            .header("HX-Redirect", Routes.new_competition_invitations(&space_id, &competition_id))
            .body(Body::empty())
            .unwrap(),

        Err(SaveCompetitionStructureError::CompetitionNotFound) =>
            (StatusCode::NOT_FOUND, "Compétition introuvable.").into_response(),

        Err(SaveCompetitionStructureError::Database(_)) =>
            (StatusCode::INTERNAL_SERVER_ERROR, "Erreur interne, veuillez réessayer.").into_response(),
    }
}
