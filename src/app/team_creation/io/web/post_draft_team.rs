use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::bloodbowl::team::TeamName;
use crate::app::shared_kernel::identity::id_service::EntityIdService;
use crate::app::shared_kernel::identity::ids::{CoachId, Entity, UserId};
use crate::app::team_creation::routes::Routes as TeamRoutes;
use crate::app::team_creation::use_cases::create_draft_team::create_draft_team;
use crate::app::team_creation::use_cases::season_access_service::{
    acces_creation, AccesCreation, SAISON_PAS_OUVERTE,
};
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
    pub coach_name: Option<String>,
    pub logo_url: Option<String>,
    pub competition_id: String,
    pub season_id: String,
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
        Err(_) => {
            return error_response(
                "Veuillez remplir tous les champs obligatoires (compétition, saison).",
            )
        }
    };

    let team_name = match TeamName::try_new(form.team_name) {
        Ok(n) => n,
        Err(_) => {
            // « alphanumériques » était faux avant même la bascule : les
            // espaces et les tirets passaient. Le message dit la règle réelle.
            return error_response(
                "Le nom d'équipe doit tenir sur une ligne, sans caractère invisible, et faire 100 caractères au plus.",
            );
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
        .and_then(|u| crate::app::shared_kernel::identity::ids::CloudinaryImage::try_new(u).ok());

    if form.competition_id.is_empty() || form.season_id.is_empty() {
        return error_response("Sélectionnez une compétition et une saison.");
    }

    let acces = acces_creation(
        state
            .team_creation
            .competition_rules
            .find_season_creation_data(&form.season_id)
            .await,
    );
    let creation_rules = match acces {
        AccesCreation::Ouverte(r) => r,
        AccesCreation::PasEncorePrete { statut } => {
            tracing::info!(
                season_id = %form.season_id,
                statut = %statut,
                "création d'équipe refusée : la saison n'est pas ouverte aux inscriptions"
            );
            return error_response(SAISON_PAS_OUVERTE);
        }
        AccesCreation::SansRegles => {
            return error_response("La compétition sélectionnée n'a pas de règles de création.")
        }
        AccesCreation::Introuvable => {
            return error_response("Saison introuvable — rechargez la page.")
        }
    };

    let user_id_str = user.id.to_string();
    let created_by = UserId::try_new(&user_id_str).unwrap_or_else(|_| user.id.clone());

    let id_service = EntityIdService {};
    let coach_name_val = form.coach_name.unwrap_or_default();
    let team = create_draft_team(
        &id_service,
        created_by,
        team_name,
        coach_id,
        coach_name_val,
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
        .header(
            "HX-Redirect",
            routes.team_build(&space_id_raw, &team.get_id().to_string()),
        )
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
