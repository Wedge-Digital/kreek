//! Le panier de renvois : les lignes en attente, l'effectif après renvois, et
//! la validation de la phase.
//!
//! Il n'affiche **aucun montant** : un renvoi ne rembourse rien, il n'y a donc
//! pas de total à tenir. Ce qu'il tient à la place, c'est l'effectif et les
//! éligibles après application — les deux nombres qui décident.

use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::teams::domain::team::GamePhase;
use crate::app::teams::io::web::dismissals::{charger, suite_de, CIBLE_ERREUR_PANIER};
use crate::app::teams::io::web::dismissals_view_models::DismissalsCartVm;
use crate::app::teams::io::web::recruitment::{fragment, SuiteMutation};
use crate::app::teams::use_cases::basket_mutation;
use crate::app::teams::use_cases::commands::RemoveBasketLineCommand;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "widgets/dismissals-cart.html")]
pub struct DismissalsCartTemplate {
    pub vm: DismissalsCartVm,
}

#[derive(Deserialize)]
pub struct UnmarkStaffBody {
    pub line_id: String,
    pub version: u32,
}

pub async fn dismissals_cart(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    rendre(&state, &space_id, &team_id, false, false).await
}

/// Annulation **depuis le panier**. Elle rend le panier, parce que c'est là que
/// se trouve le bouton ; l'effectif se resynchronise sur `basketChanged`.
///
/// Même use case que l'annulation depuis la ligne du joueur : seule la réponse
/// diffère, et c'est bien la seule chose qui doit différer.
pub async fn unmark_staff(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<UnmarkStaffBody>,
) -> Response {
    let Ok(id) = TeamId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = RemoveBasketLineCommand {
        team_id: id,
        phase: GamePhase::Dismissals,
        line_id: form.line_id,
        expected_version: form.version,
    };

    let issue = basket_mutation::remove_line(
        cmd,
        &space_id,
        state.teams.team_repository.as_ref(),
        state.teams.basket_repository.as_ref(),
        state.teams.roster_catalog_port.as_ref(),
        state.teams.squad_port.as_ref(),
    )
    .await;

    let bandeau = match issue.err().map(|e| suite_de(e, CIBLE_ERREUR_PANIER)) {
        None => false,
        Some(SuiteMutation::Resynchroniser) => true,
        Some(SuiteMutation::Repondre(reponse)) => return reponse,
    };
    rendre(&state, &space_id, &team_id, true, bandeau).await
}

async fn rendre(
    state: &AppState,
    space_id: &str,
    team_id: &str,
    notifier: bool,
    bandeau: bool,
) -> Response {
    let (_, basket) = match charger(state, team_id).await {
        Ok(couple) => couple,
        Err(reponse) => return reponse,
    };
    let vm = DismissalsCartVm::from_domain(&basket, space_id);
    let vm = if bandeau {
        vm.with_concurrent_notice()
    } else {
        vm
    };
    fragment(DismissalsCartTemplate { vm }, notifier)
}
