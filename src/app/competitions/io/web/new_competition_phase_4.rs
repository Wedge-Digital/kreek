use crate::app::competitions::domain::competition_invitations::CompetitionInvitations;
use crate::app::competitions::domain::competition_notifications::CompetitionNotifications;
use crate::app::competitions::use_cases::save_competition_invitations::{
    execute, SaveCompetitionInvitationsCommand, SaveCompetitionInvitationsError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;

#[derive(Template)]
#[template(path = "new-competition-phase-4.html")]
pub struct NewCompetitionPhase4Template {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub existing_invitations_json: String,
    /// Rendu à côté du précédent, et pour la même raison : sans lui, `state`
    /// repart sur son défaut au retour arrière, et une re-validation sans
    /// toucher aux cases écraserait les réglages sauvegardés — pendant que le
    /// widget, lui, affiche les bonnes valeurs.
    pub existing_notifications_json: String,
}

/// Le corps de l'étape 4, qui n'est plus la struct de domaine seule.
///
/// `flatten` conserve la forme historique — les champs d'invitation à plat — et
/// y ajoute le sous-objet des notifications. `notify_by_email` en a disparu :
/// les quatre réglages l'absorbent.
#[derive(serde::Deserialize)]
pub struct InvitationsPayload {
    #[serde(flatten)]
    pub invitations: CompetitionInvitations,
    /// Absent d'un corps ancien : le défaut du domaine s'applique alors, tout
    /// allumé, ce qui est le comportement d'une saison neuve.
    #[serde(default)]
    pub notifications: CompetitionNotifications,
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

    let existing_notifications_json = match state
        .competitions
        .season_repository
        .find_notifications(&sid)
        .await
    {
        Ok(Some(n)) => serde_json::to_string(&n).unwrap_or_else(|e| {
            tracing::error!("phase 4 serialize notifications error for {season_id}: {e}");
            "null".to_string()
        }),
        // `null` et non le défaut sérialisé : la page distingue « rien
        // d'enregistré » de « enregistré tout éteint », et applique elle-même
        // son défaut dans le premier cas.
        Ok(None) => "null".to_string(),
        Err(e) => {
            tracing::error!("phase 4 find_notifications error for {season_id}: {e}");
            "null".to_string()
        }
    };

    NewCompetitionPhase4Template {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        existing_invitations_json,
        existing_notifications_json,
    }
    .into_response()
}

pub async fn post_competition_invitations(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<InvitationsPayload>,
) -> impl IntoResponse {
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Identifiant de saison invalide.").into_response()
        }
    };

    let cmd = SaveCompetitionInvitationsCommand {
        season_id: sid,
        invitations: payload.invitations,
        notifications: payload.notifications,
    };

    match execute(cmd, state.competitions.season_repository.as_ref()).await {
        Ok(()) => Response::builder()
            .header(
                "HX-Redirect",
                AppRoutes::default()
                    .competitions
                    .new_competition_validation(&space_id, &competition_id, &season_id),
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
