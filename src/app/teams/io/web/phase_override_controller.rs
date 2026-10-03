//! `POST …/phases/override` — un commissaire ouvre à la main une phase
//! d'après-match (carte 578).
//!
//! # Deux réponses, deux formes
//!
//! Le **succès** répond `HX-Refresh` : ouvrir une phase change le badge de
//! l'en-tête — hors de la zone d'onglets —, le bandeau et l'onglet Effectif.
//! Recharger met tout à jour d'un coup, en gardant l'onglet affiché.
//!
//! L'**erreur** ne remplace que le message du panneau, qui reste ouvert avec sa
//! saisie. Pas le pied entier : la carte 586 a appris qu'un pied porteur
//! d'Alpine, remonté, effaçait aussitôt ce que le serveur venait de dire.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::auth::domain::user::User;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::UserId;
use crate::app::teams::domain::team::OverridablePhase;
use crate::app::teams::domain::value_objects::OverrideReason;
use crate::app::teams::io::web::team_admin_guard::require_team_admin;
use crate::app::teams::io::web::team_detail::{PhaseChoiceVm, PhaseOverrideErrorVm};
use crate::app::teams::ports::RepositoryError;
use crate::app::teams::use_cases::commands::OpenPhaseOverrideCommand;
use crate::app::teams::use_cases::open_phase_override_use_case::{
    self as uc, OpenPhaseOverrideError,
};
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

/// Les primitives sont assumées : c'est le format HTTP, pas la commande.
#[derive(Debug, Deserialize)]
pub struct PhaseOverrideForm {
    pub phase: String,
    /// Vide quand le commissaire n'en donne pas : le motif est facultatif.
    #[serde(default)]
    pub reason: String,
}

#[derive(Template)]
#[template(path = "phase-override-error.html")]
struct PhaseOverrideErrorTemplate {
    vm: PhaseOverrideErrorVm,
}

pub async fn post_phase_override(
    Path((_space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    auth_session: AuthSession,
    // **En dernier, et axum l'exige** : il consomme le corps de la requête.
    Form(form): Form<PhaseOverrideForm>,
) -> Response {
    let user = match require_team_admin(&state, &auth_session, &team_id).await {
        Ok(user) => user,
        Err(refus) => return refus,
    };
    let cmd = match build_command(&form, &team_id, &user) {
        Ok(cmd) => cmd,
        Err(refus) => return refus,
    };
    let phase = cmd.phase;
    match uc::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => refresh(),
        Err(e) => translate(e, phase, &team_id),
    }
}

/// Une phase inconnue rend **400 et non un message** : le formulaire n'envoie
/// que les trois valeurs de `OverridablePhase::ALL`, une autre ne vient pas de
/// l'écran.
fn build_command(
    form: &PhaseOverrideForm,
    team_id: &str,
    user: &User,
) -> Result<OpenPhaseOverrideCommand, Response> {
    let bad_request = || StatusCode::BAD_REQUEST.into_response();
    let phase = OverridablePhase::parse(&form.phase).ok_or_else(bad_request)?;
    let team_id = TeamId::try_new(team_id).map_err(|_| bad_request())?;
    let reason = parse_reason(&form.reason)?;
    Ok(OpenPhaseOverrideCommand {
        team_id,
        phase,
        reason,
        admin_id: UserId::try_new(&user.id.to_string()).map_err(|_| bad_request())?,
        admin_name: user.coach_name.clone(),
    })
}

/// Vide — ou fait de seuls espaces — donne `None` : pas de motif.
fn parse_reason(raw: &str) -> Result<Option<OverrideReason>, Response> {
    if raw.trim().is_empty() {
        return Ok(None);
    }
    OverrideReason::try_new(raw.to_string())
        .map(Some)
        .map_err(|_| message("Le motif ne dépasse pas 200 caractères, sans caractère spécial."))
}

fn translate(e: OpenPhaseOverrideError, phase: OverridablePhase, team_id: &str) -> Response {
    match e {
        OpenPhaseOverrideError::TeamNotFound => StatusCode::NOT_FOUND.into_response(),
        OpenPhaseOverrideError::Domain(_) => message(&format!(
            "Impossible d'{} : l'équipe n'est plus prête à jouer. \
             Rechargez la fiche pour voir son état.",
            lowercase_action(phase)
        )),
        OpenPhaseOverrideError::Repository(RepositoryError::ConcurrentWrite) => {
            message("La fiche a changé entre-temps — recommence.")
        }
        OpenPhaseOverrideError::Repository(other) => {
            tracing::error!("ouverture d'une phase manuelle {team_id} : {other}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// « ouvrir le recrutement », tiré du libellé du bouton : un seul texte par
/// phase, partagé par le bouton et le refus.
fn lowercase_action(phase: OverridablePhase) -> String {
    let label = PhaseChoiceVm::all_from_domain()
        .into_iter()
        .find(|c| c.value == phase.as_str())
        .map(|c| c.open_label)
        .unwrap_or("Ouvrir la phase");
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn refresh() -> Response {
    Response::builder()
        .header("HX-Refresh", "true")
        .body(Body::empty())
        .unwrap()
        .into_response()
}

/// **200 et non 4xx.** htmx n'échange pas une réponse non-2xx par défaut : un
/// 422 n'afficherait rien, et le bouton paraîtrait sans effet.
fn message(text: &str) -> Response {
    let body = PhaseOverrideErrorTemplate {
        vm: PhaseOverrideErrorVm {
            message: text.to_string(),
        },
    }
    .render();
    match body {
        Ok(html) => Response::builder()
            .header("HX-Retarget", "#phase-override-error")
            .header("HX-Reselect", "#phase-override-error")
            .header("HX-Reswap", "outerHTML")
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap()
            .into_response(),
        Err(e) => {
            tracing::error!("rendu du message d'ouverture de phase : {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusal_names_the_phase_in_lowercase() {
        assert_eq!(
            lowercase_action(OverridablePhase::Recruitment),
            "ouvrir le recrutement"
        );
    }

    #[test]
    fn a_blank_reason_is_no_reason() {
        assert!(matches!(parse_reason("   "), Ok(None)));
        assert!(matches!(parse_reason(" Oubli "), Ok(Some(_))));
        assert!(parse_reason(&"a".repeat(201)).is_err());
    }
}
