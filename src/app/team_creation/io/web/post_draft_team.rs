use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::common_types::{CoachId, Entity, UserId};
use crate::app::shared_kernel::id_service::EntityIdService;
use crate::app::shared_kernel::team::TeamName;
use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::routes::Routes as TeamRoutes;
use crate::app::team_creation::use_cases::create_draft_team::create_draft_team;
use crate::state::AppState;
use axum::body::Body;
use axum::extract::{Json, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DraftTeamForm {
    pub team_name: String,
    pub coach_id: Option<String>,
    pub logo_url: Option<String>,
    pub competition_id: String,
    pub season_id: String,
    pub tc_creation_rules: String,
}

pub async fn post_draft_team(
    auth_session: AuthSession,
    Path(space_id_raw): Path<String>,
    State(state): State<AppState>,
    body: Result<Json<DraftTeamForm>, axum::extract::rejection::JsonRejection>,
) -> impl IntoResponse {
    let Some(user) = auth_session.user else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let Json(form) = match body {
        Ok(json) => json,
        Err(_) => return error_response("Veuillez remplir tous les champs obligatoires (compétition, saison)."),
    };

    let team_name = match TeamName::try_new(form.team_name) {
        Ok(n) => n,
        Err(_) => {
            return error_response(
                "Le nom d'équipe est invalide (1–50 caractères alphanumériques).",
            )
        }
    };

    let coach_id_str = match form.coach_id.filter(|s| !s.is_empty()) {
        Some(id) => id,
        None => return error_response("Veuillez sélectionner un coach."),
    };
    let coach_id = match CoachId::try_new(&coach_id_str) {
        Ok(id) => id,
        Err(_) => return error_response("Identifiant de coach invalide."),
    };

    let logo_url = form
        .logo_url
        .filter(|s| !s.is_empty())
        .and_then(|u| crate::app::shared_kernel::common_types::CloudinaryImage::try_new(u).ok());

    if form.competition_id.is_empty() || form.season_id.is_empty() {
        return error_response("Sélectionnez une compétition et une saison.");
    }

    let creation_rules: CreationRules = match serde_json::from_str(&form.tc_creation_rules) {
        Ok(r) => r,
        Err(_) => return error_response("Données de compétition invalides — rechargez la page."),
    };

    if creation_rules.tiers.is_empty() {
        return error_response("La compétition sélectionnée n'a pas de règles de création.");
    }

    let user_id_str = user.id.to_string();
    let created_by = UserId::try_new(&user_id_str).unwrap_or_else(|_| user.id.clone());

    let id_service = EntityIdService {};
    let team = create_draft_team(
        &id_service,
        created_by,
        team_name,
        coach_id,
        logo_url,
        form.competition_id,
        form.season_id,
        creation_rules,
    );

    if let Err(e) = state
        .team_creation
        .team_repository
        .save(&team, &space_id_raw)
        .await
    {
        tracing::error!("post_draft_team save failed space={space_id_raw}: {e}");
        return error_response("Erreur lors de la sauvegarde. Réessayez.");
    }

    let routes = TeamRoutes;
    Response::builder()
        .header("HX-Redirect", routes.team_build(&space_id_raw, &team.get_id().to_string()))
        .body(Body::empty())
        .unwrap()
        .into_response()
}

fn error_response(msg: &str) -> Response {
    Response::builder()
        .status(StatusCode::UNPROCESSABLE_ENTITY)
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(r#"<p class="table-error">{msg}</p>"#)))
        .unwrap()
}
