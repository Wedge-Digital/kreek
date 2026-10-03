//! La garde des étapes de l'assistant de création de compétition (carte 573).
//!
//! **Deux conditions**, et toutes deux côté serveur :
//! - le visiteur est admin de l'espace ou de la compétition — le créateur
//!   l'est dès la création (`Competition::new`) ;
//! - la saison est encore en brouillon (`is_draft_status`). Une compétition
//!   publiée se modifie par ses panneaux d'administration, pas par un assistant
//!   de création.
//!
//! La création elle-même (`COMPETITION_NEW`) n'est pas gardée : tout membre
//! peut créer une compétition.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::season_repository_port::is_draft_status;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::state::AppState;
use axum::body::Body;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};

/// Ce que la requête veut faire de l'étape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardIntent {
    /// Afficher l'étape (GET).
    Display,
    /// L'enregistrer (POST).
    Submit,
}

/// Ce que la garde décide, une fois le droit acquis.
#[derive(Debug, PartialEq, Eq)]
enum WizardVerdict {
    Open,
    /// Une compétition publiée affichée par l'assistant : on renvoie vers son
    /// administration — un lien ancien ou un marque-page ne doit pas finir en
    /// erreur (décision du 2026-10-03).
    RedirectToAdmin,
    /// Une compétition publiée modifiée par l'assistant : 409, rien n'est écrit.
    Conflict,
}

fn wizard_verdict(status: &str, intent: WizardIntent) -> WizardVerdict {
    match (is_draft_status(status), intent) {
        (true, _) => WizardVerdict::Open,
        (false, WizardIntent::Display) => WizardVerdict::RedirectToAdmin,
        (false, WizardIntent::Submit) => WizardVerdict::Conflict,
    }
}

/// La garde d'une étape **avec** sa saison dans le chemin (étapes 2 à 5).
///
/// `require_admin_access` répond 401, 400, 404 ou 403, et vérifie que la saison
/// appartient à la compétition (carte 416).
pub async fn require_wizard_access(
    auth_session: &AuthSession,
    headers: &HeaderMap,
    (space_id, competition_id, season_id): (&str, &str, &str),
    intent: WizardIntent,
    state: &AppState,
) -> Result<(), Response> {
    require_admin_access(auth_session, space_id, competition_id, season_id, state).await?;
    let status = season_status(season_id, state).await?;
    match wizard_verdict(&status, intent) {
        WizardVerdict::Open => Ok(()),
        WizardVerdict::RedirectToAdmin => Err(redirect_to_admin(
            headers,
            &AppRoutes::default()
                .competitions
                .admin(space_id, competition_id, season_id),
        )),
        WizardVerdict::Conflict => Err(StatusCode::CONFLICT.into_response()),
    }
}

/// La garde de l'étape 1, dont le chemin ne porte pas la saison : elle est
/// retrouvée comme le contrôleur le fait déjà, la dernière de la compétition.
pub async fn require_wizard_access_without_season(
    auth_session: &AuthSession,
    headers: &HeaderMap,
    (space_id, competition_id): (&str, &str),
    intent: WizardIntent,
    state: &AppState,
) -> Result<(), Response> {
    let season_id = latest_season_id(competition_id, state).await?;
    require_wizard_access(
        auth_session,
        headers,
        (space_id, competition_id, &season_id),
        intent,
        state,
    )
    .await
}

async fn latest_season_id(competition_id: &str, state: &AppState) -> Result<String, Response> {
    let cid = CompetitionId::try_new(competition_id)
        .map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    match state
        .competitions
        .season_repository
        .find_latest_season_id(&cid)
        .await
    {
        Ok(Some(id)) => Ok(id.to_string()),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("garde de l'assistant : saison de {competition_id} : {e:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

async fn season_status(season_id: &str, state: &AppState) -> Result<String, Response> {
    let sid = SeasonId::try_new(season_id).map_err(|_| StatusCode::BAD_REQUEST.into_response())?;
    match state.competitions.season_repository.find_full(&sid).await {
        Ok(Some(season)) => Ok(season.status),
        Ok(None) => Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => {
            tracing::error!("garde de l'assistant : statut de {season_id} : {e:?}");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
    }
}

/// `HX-Redirect` sous htmx, une vraie redirection sinon — l'étape peut être
/// ouverte par une navigation htmx comme par un lien direct.
fn redirect_to_admin(headers: &HeaderMap, url: &str) -> Response {
    if headers.contains_key("HX-Request") {
        return Response::builder()
            .header("HX-Redirect", url)
            .body(Body::empty())
            .map(IntoResponse::into_response)
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
    Redirect::to(url).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_season_opens_the_wizard_both_ways() {
        for status in [
            "draft",
            "rules_selected",
            "structure_selected",
            "invitations_configured",
        ] {
            assert_eq!(
                wizard_verdict(status, WizardIntent::Display),
                WizardVerdict::Open
            );
            assert_eq!(
                wizard_verdict(status, WizardIntent::Submit),
                WizardVerdict::Open
            );
        }
    }

    #[test]
    fn a_published_season_redirects_a_display_and_refuses_a_submit() {
        for status in ["ready", "upcoming", "completed"] {
            assert_eq!(
                wizard_verdict(status, WizardIntent::Display),
                WizardVerdict::RedirectToAdmin
            );
            assert_eq!(
                wizard_verdict(status, WizardIntent::Submit),
                WizardVerdict::Conflict
            );
        }
    }

    #[test]
    fn htmx_gets_an_hx_redirect_and_a_link_a_real_one() {
        let mut htmx = HeaderMap::new();
        htmx.insert("HX-Request", "true".parse().unwrap());
        let reponse = redirect_to_admin(&htmx, "/admin");
        assert_eq!(reponse.headers().get("HX-Redirect").unwrap(), "/admin");

        let reponse = redirect_to_admin(&HeaderMap::new(), "/admin");
        assert!(reponse.status().is_redirection());
    }
}
