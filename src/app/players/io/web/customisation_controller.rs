//! Les sept mutations de customisation : cinq gestes unitaires, la validation,
//! l'annulation.
//!
//! Toutes répondent par **le panneau re-rendu**, refus compris. Un refus métier
//! sort en `200` pour la même raison que l'édition d'effectif (carte 294) : un
//! `4xx` ferait échouer le swap HTMX et laisserait le commissaire devant un
//! panneau figé, sans savoir ce qui s'est passé.
//!
//! `ConcurrentWrite` n'est **pas** une erreur d'utilisateur : le panneau
//! re-rendu porte l'état réel, le commissaire voit que son geste n'a pas pris
//! et le refait. Un message sur un événement aussi rare qu'invisible ferait
//! plus de bruit que de bien.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::players::domain::customisation_basket::CustomisationLine;
use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::PlayerId;
use crate::app::players::domain::value_objects::{
    BasketLineId, CustomisationId, KpoDelta, SkillId, SppAmount, StatCrans,
};
use crate::app::players::io::web::customisation_access::garde;
use crate::app::players::io::web::widgets::player_customisation_widget::{
    journal, rendre_panneau, RefusalTarget, RefusalVm,
};
use crate::app::players::ports::{
    ICustomisationBasketRepository, IPlayerRepository, ISkillCatalogPort,
};
use crate::app::players::use_cases::commands::{
    AddCustomisationSkillCommand, AddCustomisationSppCommand, AddCustomisationStatCommand,
    AdjustCustomisationPriceCommand, CancelCustomisationCommand, RemoveCustomisationLineCommand,
    ValidateCustomisationCommand,
};
use crate::app::players::use_cases::customisation_basket_mutation::{
    self, CustomisationBasketError,
};
use crate::app::players::use_cases::validate_customisation_use_case::{
    self, ValidateCustomisationError,
};
use crate::app::shared_kernel::identity::sulid::SUlid;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

// ── DTOs de transport ─────────────────────────────────────────────────────────
//
// Primitives assumées : ce sont des DTOs de **transport**, validés par les
// smart constructors au moment de bâtir la commande. Un formulaire mal formé
// est un `400`, pas une erreur métier.

#[derive(Deserialize)]
pub struct AddSkillForm {
    pub skill_id: String,
    pub expected_version: u32,
}

#[derive(Deserialize)]
pub struct AddStatForm {
    pub stat: String,
    /// En **qualité du joueur** : `+1` améliore, `-1` dégrade. Jamais l'offset
    /// brut — la traduction appartient au domaine, seul détenteur de la table
    /// des directions.
    pub crans: i8,
    pub expected_version: u32,
}

#[derive(Deserialize)]
pub struct AdjustPriceForm {
    pub delta_kpo: i32,
    pub expected_version: u32,
}

#[derive(Deserialize)]
pub struct AddSppForm {
    pub amount: u8,
    pub expected_version: u32,
}

#[derive(Deserialize)]
pub struct RemoveLineForm {
    pub line_id: String,
    pub expected_version: u32,
}

#[derive(Deserialize)]
pub struct VersionForm {
    pub expected_version: u32,
}

/// Les trois dépôts que toute mutation traverse. Les nommer une fois évite de
/// répéter cinq fois la même triade d'accesseurs.
fn depots(
    state: &AppState,
) -> (
    &dyn IPlayerRepository,
    &dyn ICustomisationBasketRepository,
    &dyn ISkillCatalogPort,
) {
    (
        state.players.repository.as_ref(),
        state.players.customisation_basket_repository.as_ref(),
        state.players.skill_catalog.as_ref(),
    )
}

/// Le segment d'URL des caractéristiques, tel que le panneau l'émet.
fn parse_stat(cle: &str) -> Option<StatKind> {
    match cle {
        "ma" => Some(StatKind::Ma),
        "st" => Some(StatKind::St),
        "ag" => Some(StatKind::Ag),
        "pa" => Some(StatKind::Pa),
        "av" => Some(StatKind::Av),
        _ => None,
    }
}

// ── Les cinq mutations ────────────────────────────────────────────────────────

pub async fn post_add_skill(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(form): Form<AddSkillForm>,
) -> Response {
    if let Err(refus) = garde(&state, auth_session.user.as_ref(), &space_id, &player_id).await {
        return refus;
    }
    let Ok(skill_id) = SkillId::try_new(form.skill_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let (repo, panier, catalogue) = depots(&state);
    let issue = customisation_basket_mutation::add_skill(
        AddCustomisationSkillCommand {
            player_id: PlayerId(player_id.clone()),
            skill_id,
            expected_version: form.expected_version,
        },
        &space_id,
        repo,
        panier,
        catalogue,
    )
    .await;
    apres_mutation(&state, &space_id, &player_id, issue, RefusalTarget::Skills).await
}

pub async fn post_add_stat(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(form): Form<AddStatForm>,
) -> Response {
    if let Err(refus) = garde(&state, auth_session.user.as_ref(), &space_id, &player_id).await {
        return refus;
    }
    let Some(cmd) = commande_stat(&player_id, &form) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let (repo, panier, catalogue) = depots(&state);
    let issue =
        customisation_basket_mutation::add_stat(cmd, &space_id, repo, panier, catalogue).await;
    let cible = RefusalTarget::Stat(form.stat);
    apres_mutation(&state, &space_id, &player_id, issue, cible).await
}

/// `None` sur une clé de caractéristique inconnue ou une amplitude nulle — un
/// formulaire malformé, donc un `400`, jamais un refus métier.
fn commande_stat(player_id: &str, form: &AddStatForm) -> Option<AddCustomisationStatCommand> {
    Some(AddCustomisationStatCommand {
        player_id: PlayerId(player_id.to_string()),
        stat: parse_stat(&form.stat)?,
        crans: StatCrans::try_new(form.crans).ok()?,
        expected_version: form.expected_version,
    })
}

pub async fn post_adjust_price(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(form): Form<AdjustPriceForm>,
) -> Response {
    if let Err(refus) = garde(&state, auth_session.user.as_ref(), &space_id, &player_id).await {
        return refus;
    }
    let Ok(delta) = KpoDelta::try_new(form.delta_kpo) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let (repo, panier, catalogue) = depots(&state);
    let issue = customisation_basket_mutation::adjust_price(
        AdjustCustomisationPriceCommand {
            player_id: PlayerId(player_id.clone()),
            delta,
            expected_version: form.expected_version,
        },
        &space_id,
        repo,
        panier,
        catalogue,
    )
    .await;
    apres_mutation(&state, &space_id, &player_id, issue, RefusalTarget::Price).await
}

pub async fn post_add_spp(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(form): Form<AddSppForm>,
) -> Response {
    if let Err(refus) = garde(&state, auth_session.user.as_ref(), &space_id, &player_id).await {
        return refus;
    }
    let Ok(amount) = SppAmount::try_new(form.amount) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let (repo, panier, catalogue) = depots(&state);
    let issue = customisation_basket_mutation::add_spp(
        AddCustomisationSppCommand {
            player_id: PlayerId(player_id.clone()),
            amount,
            expected_version: form.expected_version,
        },
        &space_id,
        repo,
        panier,
        catalogue,
    )
    .await;
    apres_mutation(&state, &space_id, &player_id, issue, RefusalTarget::Spp).await
}

pub async fn post_remove_line(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(form): Form<RemoveLineForm>,
) -> Response {
    if let Err(refus) = garde(&state, auth_session.user.as_ref(), &space_id, &player_id).await {
        return refus;
    }
    let Ok(line_id) = BasketLineId::try_new(form.line_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let (repo, panier, catalogue) = depots(&state);
    let issue = customisation_basket_mutation::remove_line(
        RemoveCustomisationLineCommand {
            player_id: PlayerId(player_id.clone()),
            line_id,
            expected_version: form.expected_version,
        },
        &space_id,
        repo,
        panier,
        catalogue,
    )
    .await;
    apres_mutation(&state, &space_id, &player_id, issue, RefusalTarget::Pending).await
}

/// Traduit l'issue d'une mutation en réponse. Le panneau est re-rendu dans tous
/// les cas où il a un sens — c'est ce qui fait qu'un refus s'affiche sans
/// quitter le mode.
async fn apres_mutation(
    state: &AppState,
    space_id: &str,
    player_id: &str,
    issue: Result<(), CustomisationBasketError>,
    cible: RefusalTarget,
) -> Response {
    let refus = match issue {
        Ok(()) => None,
        // Le panneau re-rendu porte l'état réel : il dit déjà tout.
        Err(CustomisationBasketError::ConcurrentWrite) => None,
        Err(CustomisationBasketError::Domain(e)) => Some(RefusalVm {
            message: e.to_string(),
            target: cible,
        }),
        Err(CustomisationBasketError::PlayerNotFound) => {
            return StatusCode::NOT_FOUND.into_response()
        }
        Err(autre) => {
            tracing::error!("customisation mutation {player_id}: {autre:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    rendre_panneau(state, space_id, player_id, refus).await
}

// ── Validation et annulation ──────────────────────────────────────────────────

pub async fn post_validate(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
    Form(form): Form<VersionForm>,
) -> Response {
    if let Err(refus) = garde(&state, auth_session.user.as_ref(), &space_id, &player_id).await {
        return refus;
    }
    let Some(user) = auth_session.user.clone() else {
        return StatusCode::UNAUTHORIZED.into_response();
    };

    let nombre = match compter_lignes(&state, &player_id).await {
        Ok(n) => n,
        Err(refus) => return refus,
    };
    let cmd = ValidateCustomisationCommand {
        player_id: PlayerId(player_id.clone()),
        author: user.coach_name.clone().into_inner(),
        customisation_ids: identifiants(nombre),
        expected_version: form.expected_version,
    };
    let issue = appliquer(&state, cmd).await;
    apres_validation(&state, &space_id, &player_id, issue).await
}

async fn appliquer(
    state: &AppState,
    cmd: ValidateCustomisationCommand,
) -> Result<(), ValidateCustomisationError> {
    validate_customisation_use_case::execute(
        cmd,
        state.players.repository.as_ref(),
        state.players.customisation_basket_repository.as_ref(),
        state.players.skill_catalog.as_ref(),
        &state.players.event_bus,
    )
    .await
}

/// Un `CustomisationId` par ligne, engendrés **ici** : ni le domaine ni le use
/// case ne doivent tirer d'aléatoire, sous peine de devenir intestables.
///
/// Le comptage est sûr grâce à `expected_version` : si le panier bouge entre ce
/// dénombrement et la lecture du use case, la version ne correspond plus et
/// l'on retombe sur `ConcurrentWrite` plutôt que sur un décompte incohérent.
fn identifiants(nombre: usize) -> Vec<CustomisationId> {
    (0..nombre)
        .map(|_| {
            CustomisationId::try_new(SUlid::new().to_string()).expect("un ULID n'est jamais vide")
        })
        .collect()
}

async fn compter_lignes(state: &AppState, player_id: &str) -> Result<usize, Response> {
    let etat = state
        .players
        .customisation_basket_repository
        .load(player_id)
        .await
        .map_err(|e| {
            tracing::error!("customisation validate load {player_id}: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })?;

    let Some(etat) = etat else { return Ok(0) };
    let lignes: Vec<CustomisationLine> = serde_json::from_value(etat.state).map_err(|e| {
        tracing::error!("customisation validate panier illisible {player_id}: {e}");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })?;
    Ok(lignes.len())
}

async fn apres_validation(
    state: &AppState,
    space_id: &str,
    player_id: &str,
    issue: Result<(), ValidateCustomisationError>,
) -> Response {
    match issue {
        // La fiche entière change — caractéristiques, prix, SPP, compétences.
        // Un swap partiel laisserait la moitié de la page périmée.
        Ok(()) => [("HX-Refresh", "true")].into_response(),
        Err(ValidateCustomisationError::PlayerNotFound) => StatusCode::NOT_FOUND.into_response(),
        // Rien à appliquer, ou panier déplacé sous les pieds : le panneau
        // re-rendu dit l'état réel sans dramatiser.
        Err(ValidateCustomisationError::NothingToApply)
        | Err(ValidateCustomisationError::ConcurrentWrite) => {
            rendre_panneau(state, space_id, player_id, None).await
        }
        Err(ValidateCustomisationError::LinesRejected(refusees)) => {
            rendre_panneau(state, space_id, player_id, Some(refus_global(&refusees))).await
        }
        Err(autre) => {
            tracing::error!("customisation validate {player_id}: {autre:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Tout ou rien : le message dit **ce qui a bloqué**, et que rien n'a bougé.
/// Le placer dans la zone du panier est le seul choix qui ait du sens — la
/// revalidation juge des lignes ajoutées bien plus tôt, sur des clics
/// différents.
fn refus_global(
    refusees: &[crate::app::players::domain::customisation_basket::RejectedLine],
) -> RefusalVm {
    let motifs: Vec<String> = refusees.iter().map(|l| l.cause.to_string()).collect();
    RefusalVm {
        message: format!("Rien n'a été appliqué — {}", motifs.join(", ")),
        target: RefusalTarget::Pending,
    }
}

pub async fn post_cancel(
    Path((space_id, player_id)): Path<(String, String)>,
    auth_session: AuthSession,
    State(state): State<AppState>,
) -> Response {
    if let Err(refus) = garde(&state, auth_session.user.as_ref(), &space_id, &player_id).await {
        return refus;
    }
    let issue = validate_customisation_use_case::cancel(
        CancelCustomisationCommand {
            player_id: PlayerId(player_id.clone()),
        },
        state.players.customisation_basket_repository.as_ref(),
    )
    .await;

    if let Err(e) = issue {
        tracing::error!("customisation cancel {player_id}: {e:?}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    // Le panier disparu, le slot n'a plus de mode à afficher : retour au
    // journal, exactement ce que verrait un rechargement complet.
    journal(space_id, player_id, state).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stat_couvre_les_cinq_cles_du_panneau() {
        for (cle, attendu) in [
            ("ma", StatKind::Ma),
            ("st", StatKind::St),
            ("ag", StatKind::Ag),
            ("pa", StatKind::Pa),
            ("av", StatKind::Av),
        ] {
            assert_eq!(parse_stat(cle), Some(attendu));
        }
        assert_eq!(parse_stat("MA"), None, "la casse n'est pas tolérée");
        assert_eq!(parse_stat("inconnue"), None);
    }

    /// Les clés émises par le panneau et celles acceptées ici sont **la même
    /// liste**. Une divergence donnerait un `400` sur un bouton du panneau.
    #[test]
    fn les_cles_du_panneau_sont_toutes_acceptees() {
        use crate::app::players::io::web::widgets::stat_display;
        for d in stat_display::ALL.iter() {
            assert_eq!(
                parse_stat(d.key),
                Some(d.stat),
                "la clé {} rendue par le panneau est refusée ici",
                d.key
            );
        }
    }

    #[test]
    fn un_identifiant_est_engendre_par_ligne_et_ils_sont_distincts() {
        let ids = identifiants(3);
        assert_eq!(ids.len(), 3);

        let mut uniques: Vec<_> = ids.iter().map(|i| i.as_ref().to_string()).collect();
        uniques.sort();
        uniques.dedup();
        assert_eq!(uniques.len(), 3, "deux customisations partageraient un id");

        assert!(identifiants(0).is_empty());
    }
}
