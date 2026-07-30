//! La page de renvois et ce que ses deux widgets partagent.
//!
//! Même discipline qu'au recrutement : le rendu se fait **toujours depuis une
//! lecture fraîche** — équipe rechargée, panier réhydraté — jamais depuis
//! l'agrégat que vient de retourner une mutation. Depuis la carte 268 les
//! mutations ne retournent d'ailleurs plus rien, précisément pour rendre
//! l'erreur impossible.

use crate::app::routes::AppRoutes;
use crate::app::teams::domain::basket::RejectedLine;
use crate::app::teams::domain::dismissals_basket::DismissalsBasket;
use crate::app::teams::domain::team::{GamePhase, Team};
use crate::app::teams::io::web::dismissals_view_models::DismissalCartLineVm;
use crate::app::teams::io::web::recruitment::{fragment_erreur, SuiteMutation};
use crate::app::teams::io::web::view_models::BasketErrorVm;
use crate::app::teams::routes::Routes;
use crate::app::teams::use_cases::basket_hydration_service::hydrate_dismissals_basket;
use crate::app::teams::use_cases::basket_mutation::BasketMutationError;
use crate::state::AppState;
use askama::Template;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "teams-dismissals.html")]
pub struct DismissalsPageTemplate {
    pub app_routes: AppRoutes,
    pub roster_url: String,
    pub cart_url: String,
}

impl IntoResponse for DismissalsPageTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => erreur_serveur("rendu de la page", e),
        }
    }
}

pub async fn dismissals_page(
    Path((space_id, team_id)): Path<(String, String)>,
) -> impl IntoResponse {
    DismissalsPageTemplate {
        app_routes: AppRoutes::default(),
        roster_url: Routes.dismissals_roster_widget(&space_id, &team_id),
        cart_url: Routes.dismissals_cart_widget(&space_id, &team_id),
    }
}

/// Emplacements d'erreur des deux widgets. Vidés à chaque rendu réussi : une
/// erreur disparaît donc d'elle-même au geste suivant.
pub(crate) const CIBLE_ERREUR_EFFECTIF: &str = "#dis-roster-error";
pub(crate) const CIBLE_ERREUR_PANIER: &str = "#dis-cart-error";

/// Charge l'équipe et hydrate son panier de renvois, ou rend la réponse d'échec.
///
/// La phase est vérifiée ici : hors renvois, le panier n'a pas de sens et
/// l'écran ne doit pas s'afficher.
pub(crate) async fn charger(
    state: &AppState,
    team_id: &str,
) -> Result<(Team, DismissalsBasket), Response> {
    let team = match state.teams.team_repository.find_by_id(team_id).await {
        Ok(Some(team)) => team,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => return Err(erreur_serveur("chargement de l'équipe", e)),
    };
    if team.game_phase != Some(GamePhase::Dismissals) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    }

    match hydrate_dismissals_basket(
        &team,
        state.teams.basket_repository.as_ref(),
        state.teams.roster_catalog_port.as_ref(),
        state.teams.squad_port.as_ref(),
    )
    .await
    {
        Ok(basket) => Ok((team, basket)),
        Err(e) => Err(erreur_serveur("hydratation du panier", e)),
    }
}

pub(crate) fn erreur_serveur(quoi: &str, e: impl std::fmt::Display) -> Response {
    tracing::error!("renvois — {quoi} : {e}");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

/// Ce que le handler doit faire d'une erreur de mutation.
///
/// Même découpage qu'au recrutement, avec ses propres emplacements d'erreur.
/// `EligibleFloorReached` arrive ici en `Domain` : le bouton était déjà
/// désactivé, mais une version périmée peut le laisser passer — la garde du
/// domaine est la seconde barrière, et c'est elle qui compte.
pub(crate) fn suite_de(erreur: BasketMutationError, cible: &str) -> SuiteMutation {
    match erreur {
        BasketMutationError::ConcurrentWrite => SuiteMutation::Resynchroniser,
        BasketMutationError::TeamNotFound => {
            SuiteMutation::Repondre(StatusCode::NOT_FOUND.into_response())
        }
        BasketMutationError::WrongPhase(phase) => {
            tracing::warn!("mutation de panier de renvois hors phase : {phase:?}");
            SuiteMutation::Repondre(StatusCode::UNPROCESSABLE_ENTITY.into_response())
        }
        BasketMutationError::Domain(cause) => {
            SuiteMutation::Repondre(fragment_erreur(&BasketErrorVm::from_domain(&cause), cible))
        }
        BasketMutationError::Hydration(e) => {
            SuiteMutation::Repondre(erreur_serveur("hydratation du panier", e))
        }
        BasketMutationError::Repository(e) => {
            SuiteMutation::Repondre(erreur_serveur("écriture du panier", e))
        }
    }
}

/// Refus en bloc à la validation : on réhydrate pour nommer les lignes
/// fautives, puis on rend l'erreur dans l'emplacement du panier — c'est là que
/// se trouve le bouton qui vient d'échouer.
pub(crate) async fn refus_en_bloc(
    state: &AppState,
    team_id: &str,
    rejetees: Vec<RejectedLine>,
) -> Response {
    let libelles = match charger(state, team_id).await {
        Ok((_, basket)) => nommer(&basket, &rejetees),
        Err(_) => rejetees
            .iter()
            .map(|r| BasketErrorVm::raison_de(&r.cause))
            .collect(),
    };
    fragment_erreur(&BasketErrorVm::refus_en_bloc(libelles), CIBLE_ERREUR_PANIER)
}

/// « Grumpf — L'effectif ne peut pas descendre sous onze joueurs éligibles. »
/// Une ligne dont le panier ne connaît plus l'identifiant garde au moins sa
/// cause.
fn nommer(basket: &DismissalsBasket, rejetees: &[RejectedLine]) -> Vec<String> {
    let etiquettes = DismissalCartLineVm::all_from_domain(basket);
    rejetees
        .iter()
        .map(|r| {
            let raison = BasketErrorVm::raison_de(&r.cause);
            match etiquettes.iter().find(|l| l.line_id == r.id.0) {
                Some(l) => format!("{} — {raison}", l.label),
                None => raison,
            }
        })
        .collect()
}
