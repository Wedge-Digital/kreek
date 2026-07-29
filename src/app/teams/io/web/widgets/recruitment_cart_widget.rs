//! Panier de recrutement : les lignes accumulées, le reste après achats, la
//! validation de phase.
//!
//! Deux routes de retrait, une par famille de ligne, pour la symétrie avec les
//! deux routes d'ajout. Elles mènent au même use case : retirer une ligne par
//! son identifiant est la même opération, et le panier sait de quel type elle
//! est.

use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::teams::domain::team::GamePhase;
use crate::app::teams::io::web::recruitment::{
    charger, fragment, suite_de, SuiteMutation, CIBLE_ERREUR_PANIER,
};
use crate::app::teams::io::web::view_models::RecruitmentCartVm;
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
#[template(path = "widgets/recruitment-cart.html")]
pub struct RecruitmentCartTemplate {
    pub vm: RecruitmentCartVm,
}

#[derive(Deserialize)]
pub struct RemoveLineBody {
    pub line_id: String,
    pub version: u32,
}

pub async fn recruitment_cart(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    rendre(&state, &space_id, &team_id, false, false).await
}

pub async fn remove_player(
    path: Path<(String, String)>,
    state: State<AppState>,
    form: Form<RemoveLineBody>,
) -> Response {
    retirer(path, state, form).await
}

pub async fn remove_staff(
    path: Path<(String, String)>,
    state: State<AppState>,
    form: Form<RemoveLineBody>,
) -> Response {
    retirer(path, state, form).await
}

async fn retirer(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<RemoveLineBody>,
) -> Response {
    let Ok(id) = TeamId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = RemoveBasketLineCommand {
        team_id: id,
        phase: GamePhase::Recruitment,
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

    apres_mutation(&state, &space_id, &team_id, issue.err()).await
}

async fn apres_mutation(
    state: &AppState,
    space_id: &str,
    team_id: &str,
    erreur: Option<basket_mutation::BasketMutationError>,
) -> Response {
    let bandeau = match erreur.map(|e| suite_de(e, CIBLE_ERREUR_PANIER)) {
        None => false,
        Some(SuiteMutation::Resynchroniser) => true,
        Some(SuiteMutation::Repondre(reponse)) => return reponse,
    };
    rendre(state, space_id, team_id, true, bandeau).await
}

async fn rendre(
    state: &AppState,
    space_id: &str,
    team_id: &str,
    notifier: bool,
    bandeau: bool,
) -> Response {
    let (team, basket) = match charger(state, team_id).await {
        Ok(couple) => couple,
        Err(reponse) => return reponse,
    };
    let vm = RecruitmentCartVm::from_domain(&team, &basket, space_id);
    let vm = if bandeau {
        vm.with_concurrent_notice()
    } else {
        vm
    };
    fragment(RecruitmentCartTemplate { vm }, notifier)
}
