//! Le panneau « Poules » : renommer, ajouter, retirer.
//!
//! Retirer une poule **désaffecte ses équipes** — la cascade s'en charge dès que
//! la ligne de poule disparaît. Leurs points, eux, ne bougent pas : une poule est
//! un regroupement de classement, pas une appartenance.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_structure::{RankingGroupName, UseRankingGroups};
use crate::app::competitions::domain::error::DomainError;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::io::web::admin::settings::builders::build_pools_vm;
use crate::app::competitions::use_cases::settings::update_pools_settings_use_case::{
    self, PoolInput, UpdatePoolsSettingsCommand, UpdatePoolsSettingsError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::app::shared_kernel::bloodbowl::ranking_group_id::RankingGroupId;
use crate::app::shared_kernel::identity::id_service::EntityIdService;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

// ── Vue ───────────────────────────────────────────────────────────────────────

pub struct PoolRowVm {
    pub id: String,
    pub name: String,
    /// Le nombre d'équipes que retirer cette poule désaffecterait. Lu dans
    /// `competition_group_teams`, jamais dans la déclaration.
    pub assigned_teams: u32,
    /// Le même nombre, en toutes lettres. Porté par le VM et non calculé dans
    /// le gabarit : « les structs de template ne portent que des view models ».
    pub assigned_label: String,
}

pub struct PoolsVm {
    pub use_pools: bool,
    pub pools: Vec<PoolRowVm>,
}

pub struct PoolsPanelVm {
    pub pools: PoolsVm,
    /// Ce que le pied annonce après un enregistrement. `None` tant qu'aucun n'a
    /// eu lieu — l'écran dit alors ce qu'un retrait *ferait*, pas ce qu'il a
    /// fait.
    pub outcome: Option<String>,
    pub error: Option<String>,
}

/// Le compte rendu d'un enregistrement, en toutes lettres.
pub fn phrase_de_desaffectation(n: u32) -> String {
    match n {
        0 => "Poules enregistrées. Aucune équipe n'a été désaffectée.".to_string(),
        1 => "Poules enregistrées — 1 équipe a été désaffectée.".to_string(),
        n => format!("Poules enregistrées — {n} équipes ont été désaffectées."),
    }
}

#[derive(Template)]
#[template(path = "admin/widgets/settings-pools.html")]
pub struct SettingsPoolsTemplate {
    pub vm: PoolsPanelVm,
    pub post_url: String,
}

impl IntoResponse for SettingsPoolsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("settings pools render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_settings_pools(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }
    rendre(&state, &space_id, &competition_id, &season_id, None, None).await
}

/// **Deux tableaux parallèles**, un élément par ligne, dans l'ordre visuel.
///
/// L'extracteur vient d'`axum-extra` et non d'axum : ce dernier s'appuie sur
/// `serde_urlencoded`, qui refuse les clés répétées (« invalid type: string,
/// expected a sequence ») et ferait échouer toute soumission en `422`. Même
/// précédent que `roster_edition_controller`.
///
/// **Aucune liste parallèle ne vient d'une case à cocher** : une case décochée
/// n'est pas soumise, et les deux `Vec` se désynchroniseraient dès la première.
/// C'est une contrainte sur le gabarit autant que sur ce DTO — `use_pools` est
/// un booléen unique, jamais un élément de liste.
#[derive(Deserialize)]
pub struct PoolsSettingsForm {
    #[serde(default)]
    pub use_pools: bool,
    /// Vide pour une poule neuve.
    #[serde(default)]
    pub pool_id: Vec<String>,
    #[serde(default)]
    pub pool_name: Vec<String>,
}

pub async fn post_settings_pools(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum_extra::extract::Form(form): axum_extra::extract::Form<PoolsSettingsForm>,
) -> Response {
    if let Err(refus) = require_admin_access(
        &auth_session,
        &space_id,
        &competition_id,
        &season_id,
        &state,
    )
    .await
    {
        return refus;
    }

    // **Un écart de longueur est un `400`, jamais un `zip`.** Celui-ci s'arrête
    // sur la plus courte et perdrait une poule sans rien dire — le commissaire
    // verrait son enregistrement réussir et une poule disparaître.
    if form.pool_id.len() != form.pool_name.len() {
        tracing::error!(
            "settings pools {season_id}: {} identifiants pour {} noms",
            form.pool_id.len(),
            form.pool_name.len()
        );
        return StatusCode::BAD_REQUEST.into_response();
    }

    let (Ok(season), Ok(pools)) = (SeasonId::try_new(&season_id), construire(&form)) else {
        return rendre(
            &state,
            &space_id,
            &competition_id,
            &season_id,
            None,
            Some("Un nom de poule est invalide.".to_string()),
        )
        .await;
    };

    let issue = update_pools_settings_use_case::execute(
        UpdatePoolsSettingsCommand {
            season_id: season,
            use_pools: UseRankingGroups(form.use_pools),
            pools,
        },
        state.competitions.season_repository.as_ref(),
        &EntityIdService {},
    )
    .await;

    match issue {
        Ok(rapport) => {
            rendre(
                &state,
                &space_id,
                &competition_id,
                &season_id,
                Some(rapport.unassigned_teams),
                None,
            )
            .await
        }
        Err(UpdatePoolsSettingsError::InvalidPools(cause)) => {
            rendre(
                &state,
                &space_id,
                &competition_id,
                &season_id,
                None,
                Some(cause.to_string()),
            )
            .await
        }
        Err(UpdatePoolsSettingsError::SeasonNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(autre) => {
            tracing::error!("settings pools {season_id}: {autre:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Les deux tableaux, appariés. Un identifiant vide désigne une poule neuve.
fn construire(form: &PoolsSettingsForm) -> Result<Vec<PoolInput>, DomainError> {
    form.pool_id
        .iter()
        .zip(form.pool_name.iter())
        .map(|(id, nom)| {
            Ok(PoolInput {
                id: match id.trim().is_empty() {
                    true => None,
                    false => Some(
                        RankingGroupId::try_new(id.clone())
                            .map_err(|_| DomainError::DuplicatePoolId { id: id.clone() })?,
                    ),
                },
                name: RankingGroupName::try_new(nom.clone())
                    .map_err(|_| DomainError::DuplicatePoolName { name: nom.clone() })?,
            })
        })
        .collect()
}

/// Le widget, relu depuis la base — déclaration **et** affectations.
async fn rendre(
    state: &AppState,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    unassigned: Option<u32>,
    error: Option<String>,
) -> Response {
    let Ok(season) = SeasonId::try_new(season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let structure = match state
        .competitions
        .season_repository
        .find_structure(&season)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("settings pools find structure {season_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let affectations = state
        .competitions
        .group_repository
        .find_groups(season_id)
        .await
        .unwrap_or_default();

    SettingsPoolsTemplate {
        vm: PoolsPanelVm {
            pools: build_pools_vm(&structure.ranking_group, &affectations),
            outcome: unassigned.map(phrase_de_desaffectation),
            error,
        },
        post_url: AppRoutes::default().competitions.admin_settings_pools(
            space_id,
            competition_id,
            season_id,
        ),
    }
    .into_response()
}
