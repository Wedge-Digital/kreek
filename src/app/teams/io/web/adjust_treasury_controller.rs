//! `POST …/tresorerie/ajuster` — un commissaire crédite ou débite une caisse.
//!
//! # Deux réponses, deux formes
//!
//! Le **succès** rend la page entière, que le formulaire échange sur
//! `#app-content`. Pas sur `#team-tab-zone` : la trésorerie s'affiche **deux
//! fois** sur cette page, dans le bandeau de l'onglet et dans l'en-tête de la
//! fiche — et l'en-tête est hors de la zone d'onglets. Un swap limité à la zone
//! laisserait deux chiffres contradictoires à l'écran.
//!
//! L'**erreur** ne remplace que le message du panneau, qui reste ouvert avec sa
//! saisie. Trois en-têtes, et `HX-Reselect` n'est pas une précaution : le
//! formulaire porte `hx-select`, qui filtrerait aussi la réponse d'erreur, n'y
//! trouverait pas `#app-content`, et **rien ne s'afficherait**. Le piège est
//! documenté par `finalize_team.rs` pour l'avoir payé.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::shared_kernel::identity::ids::UserId;
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::treasury::MovementDirection;
use crate::app::teams::domain::value_objects::{AdjustmentAmount, AdjustmentNote};
use crate::app::teams::io::web::team_detail::rendre_fiche;
use crate::app::teams::io::web::treasury_view_models::AdjustErrorVm;
use crate::app::teams::use_cases::adjust_treasury_use_case::{self as uc, AdjustTreasuryError};
use crate::app::teams::use_cases::commands::AdjustTreasuryCommand;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

/// Les primitives sont assumées : c'est le format HTTP, pas la commande. Leur
/// traduction en value objects est à la charge de ce contrôleur.
#[derive(Debug, Deserialize)]
pub struct AdjustTreasuryForm {
    pub direction: String,
    pub amount_kpo: u32,
    pub note: String,
}

#[derive(Template)]
#[template(path = "teams-treasury-adjust-error.html")]
struct AdjustErrorTemplate {
    vm: AdjustErrorVm,
}

pub async fn adjust_treasury(
    Path((space_id, team_id)): Path<(String, String)>,
    perms: SpacePermissions,
    State(state): State<AppState>,
    auth_session: AuthSession,
    // **En dernier, et axum l'exige** : il consomme le corps de la requête.
    Form(form): Form<AdjustTreasuryForm>,
) -> Response {
    if !perms.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(user) = auth_session.user.clone() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let cmd = match construire_commande(&form, &team_id, &user) {
        Ok(cmd) => cmd,
        Err(refus) => return refus,
    };

    match uc::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => rendre_fiche(&space_id, &team_id, auth_session, &state, "treasury").await,
        Err(e) => traduire(e, &team_id),
    }
}

/// Le sens illisible rend **400 et non un message** : le formulaire n'envoie que
/// `Credit` ou `Debit`, donc une autre valeur ne vient pas de l'écran. Il n'y a
/// personne à qui expliquer quoi que ce soit.
fn construire_commande(
    form: &AdjustTreasuryForm,
    team_id: &str,
    user: &crate::app::auth::domain::user::User,
) -> Result<AdjustTreasuryCommand, Response> {
    let direction = MovementDirection::parse(&form.direction)
        .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;
    let team_id = TeamId::try_new(team_id).map_err(|_| StatusCode::BAD_REQUEST.into_response())?;

    let amount = AdjustmentAmount::try_new(form.amount_kpo)
        .map_err(|_| message(&format!("Montant invalide : {}", bornes())))?;
    let note = AdjustmentNote::try_new(form.note.clone())
        .map_err(|_| message("Le motif est obligatoire, et ne dépasse pas 200 caractères."))?;

    Ok(AdjustTreasuryCommand {
        team_id,
        direction,
        amount,
        note,
        admin_id: UserId::try_new(&user.id.to_string())
            .map_err(|_| StatusCode::BAD_REQUEST.into_response())?,
        admin_name: user.coach_name.clone(),
    })
}

fn bornes() -> &'static str {
    "un multiple de 5 kPo, entre 5 et 500."
}

fn traduire(e: AdjustTreasuryError, team_id: &str) -> Response {
    match e {
        AdjustTreasuryError::TeamNotFound => StatusCode::NOT_FOUND.into_response(),
        AdjustTreasuryError::Domain(DomainError::InsufficientTreasury) => {
            message("Le solde ne couvre pas ce retrait.")
        }
        AdjustTreasuryError::Domain(autre) => message(&autre.to_string()),
        AdjustTreasuryError::Repository(
            crate::app::teams::ports::RepositoryError::ConcurrentWrite,
        ) => message("La fiche a changé entre-temps — recommence."),
        AdjustTreasuryError::Repository(autre) => {
            tracing::error!("ajustement de trésorerie {team_id} : {autre}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// **200 et non 4xx.** htmx n'échange pas une réponse non-2xx par défaut : un
/// 422 n'afficherait rien, et le bouton paraîtrait sans effet. Les deux autres
/// points d'erreur du dépôt répondent 200 pour cette raison exacte.
fn message(texte: &str) -> Response {
    let corps = AdjustErrorTemplate {
        vm: AdjustErrorVm {
            message: texte.to_string(),
        },
    }
    .render();

    match corps {
        Ok(html) => Response::builder()
            .header("HX-Retarget", "#adm-msg")
            .header("HX-Reselect", "#adm-msg")
            .header("HX-Reswap", "outerHTML")
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap()
            .into_response(),
        Err(e) => {
            tracing::error!("rendu du message d'ajustement : {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
