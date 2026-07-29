//! Catalogue de recrutement : les deux tableaux et la composition de
//! l'effectif, dans un seul widget.
//!
//! Ils ne sont pas séparés parce qu'ajouter un joueur ne change pas que sa
//! ligne — la trésorerie épuisée ou l'effectif plein désactivent **toutes** les
//! autres. Un swap ligne par ligne serait donc faux.

use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::teams::io::web::recruitment::{
    charger, fragment, suite_de, SuiteMutation, CIBLE_ERREUR_CATALOGUE,
};
use crate::app::teams::io::web::view_models::{staff_type_from_form, RecruitmentCatalogVm};
use crate::app::teams::use_cases::basket_mutation;
use crate::app::teams::use_cases::commands::{AddBasketPlayerCommand, AddBasketStaffCommand};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "widgets/recruitment-catalog.html")]
pub struct RecruitmentCatalogTemplate {
    pub vm: RecruitmentCatalogVm,
}

#[derive(Deserialize)]
pub struct AddPlayerBody {
    pub roster_line_id: String,
    pub version: u32,
}

#[derive(Deserialize)]
pub struct AddStaffBody {
    pub staff_uid: String,
    pub version: u32,
}

pub async fn recruitment_catalog(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    rendre(&state, &space_id, &team_id, false, false).await
}

pub async fn add_player(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<AddPlayerBody>,
) -> Response {
    let Ok(id) = TeamId::try_new(&team_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = AddBasketPlayerCommand {
        team_id: id,
        roster_line_id: form.roster_line_id,
        expected_version: form.version,
    };

    let issue = basket_mutation::add_player(
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

pub async fn add_staff(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<AddStaffBody>,
) -> Response {
    let (Ok(id), Some(staff_type)) = (
        TeamId::try_new(&team_id),
        staff_type_from_form(&form.staff_uid),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = AddBasketStaffCommand {
        team_id: id,
        staff_type,
        expected_version: form.version,
    };

    let issue = basket_mutation::add_staff(
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

/// Succès et écriture concurrente rendent le **même** fragment, reconstruit
/// depuis l'état persisté ; seul le bandeau les distingue. Les deux émettent
/// `basketChanged` : dans un cas le panier a changé, dans l'autre il avait
/// changé sans qu'on le sache.
async fn apres_mutation(
    state: &AppState,
    space_id: &str,
    team_id: &str,
    erreur: Option<basket_mutation::BasketMutationError>,
) -> Response {
    let bandeau = match erreur.map(|e| suite_de(e, CIBLE_ERREUR_CATALOGUE)) {
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
    let vm = RecruitmentCatalogVm::from_domain(&team, &basket, space_id);
    let vm = if bandeau {
        vm.with_concurrent_notice()
    } else {
        vm
    };
    fragment(RecruitmentCatalogTemplate { vm }, notifier)
}
