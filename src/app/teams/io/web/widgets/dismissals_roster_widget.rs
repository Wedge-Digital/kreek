//! L'effectif de la phase de renvois : joueurs et staff, dans un seul widget.
//!
//! Ils ne sont pas séparés pour la même raison qu'au recrutement, à l'envers :
//! marquer un joueur peut faire basculer **tous** les autres en « Minimum 11 »
//! d'un seul coup. Un swap ligne par ligne serait faux.

use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
use crate::app::shared_kernel::bloodbowl::team::TeamId;
use crate::app::teams::domain::team::GamePhase;
use crate::app::teams::io::web::dismissals::{charger, suite_de, CIBLE_ERREUR_EFFECTIF};
use crate::app::teams::io::web::dismissals_view_models::DismissalsRosterVm;
use crate::app::teams::io::web::recruitment::{fragment, SuiteMutation};
use crate::app::teams::io::web::view_models::staff_type_from_form;
use crate::app::teams::use_cases::basket_mutation;
use crate::app::teams::use_cases::commands::{
    MarkPlayerForDismissalCommand, MarkStaffForDismissalCommand, RemoveBasketLineCommand,
};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "widgets/dismissals-roster.html")]
pub struct DismissalsRosterTemplate {
    pub vm: DismissalsRosterVm,
}

#[derive(Deserialize)]
pub struct MarkPlayerBody {
    pub player_id: String,
    pub version: u32,
}

#[derive(Deserialize)]
pub struct MarkStaffBody {
    pub staff_uid: String,
    pub version: u32,
}

/// L'annulation depuis la ligne du joueur poste un `line_id`, comme le « × » du
/// panier : une seule façon de retirer une ligne, celle de la carte 268.
#[derive(Deserialize)]
pub struct UnmarkBody {
    pub line_id: String,
    pub version: u32,
}

pub async fn dismissals_roster(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    rendre(&state, &space_id, &team_id, false, false).await
}

pub async fn mark_player(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<MarkPlayerBody>,
) -> Response {
    let (Ok(id), Ok(player_id)) = (
        TeamId::try_new(&team_id),
        PlayerId::try_new(&form.player_id),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = MarkPlayerForDismissalCommand {
        team_id: id,
        player_id,
        expected_version: form.version,
    };

    let issue = basket_mutation::mark_player(
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

pub async fn mark_staff(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<MarkStaffBody>,
) -> Response {
    let (Ok(id), Some(staff_type)) = (
        TeamId::try_new(&team_id),
        staff_type_from_form(&form.staff_uid),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let cmd = MarkStaffForDismissalCommand {
        team_id: id,
        staff_type,
        expected_version: form.version,
    };

    let issue = basket_mutation::mark_staff(
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

/// Annulation **depuis la ligne du joueur**. Elle rend l'effectif, parce que
/// c'est là que se trouve le bouton ; le panier se resynchronise sur
/// `basketChanged`.
pub async fn unmark_player(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<UnmarkBody>,
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

    apres_mutation(&state, &space_id, &team_id, issue.err()).await
}

/// Succès et écriture concurrente rendent le **même** fragment, reconstruit
/// depuis l'état persisté ; seul le bandeau les distingue.
async fn apres_mutation(
    state: &AppState,
    space_id: &str,
    team_id: &str,
    erreur: Option<basket_mutation::BasketMutationError>,
) -> Response {
    let bandeau = match erreur.map(|e| suite_de(e, CIBLE_ERREUR_EFFECTIF)) {
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
    let vm = DismissalsRosterVm::from_domain(&team, &basket, space_id);
    let vm = if bandeau {
        vm.with_concurrent_notice()
    } else {
        vm
    };
    fragment(DismissalsRosterTemplate { vm }, notifier)
}
