//! Le panneau « Tiers & coups de pouce ».
//!
//! Ce panneau n'ouvre **que** les coups de pouce et les star players. Le nom, le
//! budget, l'XP de départ et les rosters sont affichés mais figés — et le
//! domaine refuse tout écart (`with_inducements_from`, carte 417). Les montrer
//! grisés dit pourquoi ils ne se saisissent pas ; les cacher laisserait croire
//! qu'ils n'existent pas.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_rules::TierRule;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::io::web::admin::settings::builders::build_tiers_vm;
use crate::app::competitions::use_cases::settings::update_tiers_settings_use_case::{
    self, UpdateTiersSettingsCommand, UpdateTiersSettingsError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;

// ── Vue ───────────────────────────────────────────────────────────────────────

pub struct ChipVm {
    pub uid: String,
    pub label: String,
}

pub struct TierVm {
    /// 1, 2, 3… — porte la teinte du bloc (`.tier-1`, `.tier-2`, …).
    pub index: u8,
    pub name: String,
    pub budget_kpo: u32,
    pub starting_xp: u32,
    pub roster_names: Vec<String>,
    pub inducements: Vec<ChipVm>,
    pub star_players: Vec<ChipVm>,
    /// L'instance du sélecteur pour ce tier. Le widget n'a **pas de champ
    /// caché** : il n'émet qu'un événement portant cet identifiant, et c'est par
    /// lui que le JS de collecte range la sélection.
    pub picker_instance_id: String,
    /// Les uid déjà sélectionnés, tels que le sélecteur les attend en query.
    pub selected_inducements: String,
    pub selected_star_players: String,
    /// Les champs figés, **sérialisés par le serveur** et renvoyés tels quels.
    ///
    /// Les recomposer côté navigateur risquerait d'introduire l'écart même que
    /// le domaine refuse — un budget relu depuis un libellé formaté, un roster
    /// perdu en route. Ce que le serveur accepte en retour, il l'écrit lui-même.
    pub frozen_json: String,
}

pub struct TiersPanelVm {
    pub tiers: Vec<TierVm>,
    pub saved: bool,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "admin/widgets/settings-tiers.html")]
pub struct SettingsTiersTemplate {
    pub vm: TiersPanelVm,
    pub post_url: String,
    pub inducement_picker_url: String,
    pub star_player_picker_url: String,
}

impl IntoResponse for SettingsTiersTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("settings tiers render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_settings_tiers(
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
    rendre(&state, &space_id, &competition_id, &season_id, false, None).await
}

/// JSON : la cible est un agrégat imbriqué que les nutypes valident à la
/// désérialisation.
#[derive(Deserialize)]
pub struct TiersSettingsPayload {
    pub tiers: Vec<TierRule>,
}

pub async fn post_settings_tiers(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Json(payload): Json<TiersSettingsPayload>,
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
    let Ok(season) = SeasonId::try_new(&season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    match update_tiers_settings_use_case::execute(
        UpdateTiersSettingsCommand {
            season_id: season,
            tiers: payload.tiers,
        },
        state.competitions.season_repository.as_ref(),
    )
    .await
    {
        Ok(()) => rendre(&state, &space_id, &competition_id, &season_id, true, None).await,
        // Le refus du domaine est **affiché**, jamais corrigé : le message dit ce
        // qui a bougé, et l'écran montre l'état réellement enregistré.
        Err(UpdateTiersSettingsError::Rejected(cause)) => {
            rendre(
                &state,
                &space_id,
                &competition_id,
                &season_id,
                false,
                Some(cause.to_string()),
            )
            .await
        }
        Err(UpdateTiersSettingsError::SeasonNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(autre) => {
            tracing::error!("settings tiers {season_id}: {autre:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn rendre(
    state: &AppState,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    saved: bool,
    error: Option<String>,
) -> Response {
    let Ok(season) = SeasonId::try_new(season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let regles = match state
        .competitions
        .season_repository
        .find_rules(&season)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("settings tiers find rules {season_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let routes = AppRoutes::default();
    SettingsTiersTemplate {
        vm: TiersPanelVm {
            tiers: build_tiers_vm(&regles.tiers, state.competitions.reference_port.as_ref()),
            saved,
            error,
        },
        post_url: routes
            .competitions
            .admin_settings_tiers(space_id, competition_id, season_id),
        inducement_picker_url: routes.references.inducement_picker().to_string(),
        star_player_picker_url: routes.references.star_player_picker().to_string(),
    }
    .into_response()
}
