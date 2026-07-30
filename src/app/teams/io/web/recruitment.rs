//! La page de recrutement et ce que ses deux widgets partagent.
//!
//! Le rendu se fait **toujours depuis une lecture fraîche** — équipe rechargée,
//! panier réhydraté — jamais depuis l'agrégat que vient de retourner une
//! mutation. Deux raisons : le fragment doit montrer ce qui est persisté, et
//! l'agrégat retourné porte encore sa version d'avant écriture, qu'on cuirait
//! sinon dans les `hx-vals` du prochain clic.

use crate::app::routes::AppRoutes;
use crate::app::teams::domain::basket::RejectedLine;
use crate::app::teams::domain::recruitment_basket::RecruitmentBasket;
use crate::app::teams::domain::team::{GamePhase, Team};
use crate::app::teams::io::web::view_models::{BasketErrorVm, CartLineVm};
use crate::app::teams::routes::Routes;
use crate::app::teams::use_cases::basket_hydration_service::hydrate_recruitment_basket;
use crate::app::teams::use_cases::basket_mutation::BasketMutationError;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "teams-recruitment.html")]
pub struct RecruitmentPageTemplate {
    pub app_routes: AppRoutes,
    pub catalog_url: String,
    pub cart_url: String,
}

impl IntoResponse for RecruitmentPageTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => erreur_serveur("rendu de la page", e),
        }
    }
}

pub async fn recruitment_page(
    Path((space_id, team_id)): Path<(String, String)>,
) -> impl IntoResponse {
    RecruitmentPageTemplate {
        app_routes: AppRoutes::default(),
        catalog_url: Routes.recruitment_catalog_widget(&space_id, &team_id),
        cart_url: Routes.recruitment_cart_widget(&space_id, &team_id),
    }
}

/// Emplacements d'erreur des deux widgets. Vidés à chaque rendu réussi : une
/// erreur disparaît donc d'elle-même au geste suivant.
pub(crate) const CIBLE_ERREUR_CATALOGUE: &str = "#rec-catalog-error";
pub(crate) const CIBLE_ERREUR_PANIER: &str = "#rec-cart-error";

// ── Chargement partagé par les deux widgets ──────────────────────────────────

/// Charge l'équipe et hydrate son panier, ou rend la réponse d'échec.
///
/// La phase est vérifiée ici : hors recrutement, le panier n'a pas de sens et
/// l'écran ne doit pas s'afficher.
pub(crate) async fn charger(
    state: &AppState,
    team_id: &str,
) -> Result<(Team, RecruitmentBasket), Response> {
    let team = match state.teams.team_repository.find_by_id(team_id).await {
        Ok(Some(team)) => team,
        Ok(None) => return Err(StatusCode::NOT_FOUND.into_response()),
        Err(e) => return Err(erreur_serveur("chargement de l'équipe", e)),
    };
    if team.game_phase != Some(GamePhase::Recruitment) {
        return Err(StatusCode::UNPROCESSABLE_ENTITY.into_response());
    }

    match hydrate_recruitment_basket(
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

fn erreur_serveur(quoi: &str, e: impl std::fmt::Display) -> Response {
    tracing::error!("recrutement — {quoi} : {e}");
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

// ── Issue d'une mutation ─────────────────────────────────────────────────────

/// Ce que le handler doit faire d'une erreur de mutation.
pub(crate) enum SuiteMutation {
    /// Le panier a bougé ailleurs : on réaffiche l'état du jour avec un bandeau,
    /// sans appliquer le geste et **sans réessai** — il porterait sur un état
    /// que le coach n'a pas vu.
    Resynchroniser,
    /// Rien à réafficher : la réponse est déjà construite.
    Repondre(Response),
}

/// `cible` est l'emplacement d'erreur du widget appelant. Sans lui, le
/// fragment d'erreur remplacerait le widget entier : le coach perdrait le
/// tableau au moment précis où il a besoin de le relire.
pub(crate) fn suite_de(erreur: BasketMutationError, cible: &str) -> SuiteMutation {
    match erreur {
        BasketMutationError::ConcurrentWrite => SuiteMutation::Resynchroniser,
        BasketMutationError::TeamNotFound => {
            SuiteMutation::Repondre(StatusCode::NOT_FOUND.into_response())
        }
        BasketMutationError::WrongPhase(phase) => {
            tracing::warn!("mutation de panier hors phase : {phase:?}");
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

// ── Fragments ────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/basket-error.html")]
pub struct BasketErrorTemplate {
    pub vm: BasketErrorVm,
}

fn fragment_erreur(vm: &BasketErrorVm, cible: &str) -> Response {
    let rendu = BasketErrorTemplate {
        vm: BasketErrorVm {
            message: vm.message.clone(),
            lines: vm.lines.clone(),
        },
    }
    .render();
    match rendu {
        Ok(html) => Response::builder()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .header("HX-Retarget", cible)
            .header("HX-Reswap", "innerHTML")
            .header("content-type", "text/html; charset=utf-8")
            .body(Body::from(html))
            .unwrap(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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

/// « Danseur de Guerre — Le quota de ce poste est atteint. » Une ligne dont le
/// panier ne connaît plus l'identifiant garde au moins sa cause.
fn nommer(basket: &RecruitmentBasket, rejetees: &[RejectedLine]) -> Vec<String> {
    let etiquettes = CartLineVm::all_from_domain(basket);
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

/// Rend un fragment de widget. `basketChanged` n'est émis que par les
/// mutations : un simple affichage n'a rien à resynchroniser.
pub(crate) fn fragment<T: Template>(template: T, notifier: bool) -> Response {
    match template.render() {
        Ok(html) => {
            let mut reponse =
                Response::builder().header("content-type", "text/html; charset=utf-8");
            if notifier {
                reponse = reponse.header("HX-Trigger", "basketChanged");
            }
            reponse.body(Body::from(html)).unwrap()
        }
        Err(e) => erreur_serveur("rendu du fragment", e),
    }
}
