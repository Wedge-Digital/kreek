//! Le panneau « Visibilité » : mode d'accès et validation des inscriptions.
//!
//! Le cinquième et dernier des cinq, et le plus sobre — **aucun JS**. Deux
//! groupes de boutons radio dans un formulaire ordinaire, contrairement au
//! panneau « Tiers » dont la collecte JS a coûté trois diagnostics faux
//! (carte 424). Ce qui peut être un `<form>` doit rester un `<form>`.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_invitations::{
    AccessMode, CompetitionInvitations, RequiresValidation,
};
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::use_cases::settings::update_visibility_settings_use_case::{
    self, UpdateVisibilitySettingsCommand, UpdateVisibilitySettingsError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::ids::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

// ── Vue ───────────────────────────────────────────────────────────────────────

pub struct VisibilityVm {
    pub access_mode: String,
    pub requires_validation: bool,
    /// **Affiché pour une raison précise.** Le POST réécrit le document qui
    /// porte `invited_coaches` ; montrer « 12 coachs invités » rend visible ce
    /// qu'il doit préserver. La préservation cesse d'être une précaution que
    /// seul le code connaît.
    pub invited_count: u32,
    pub saved: bool,
}

impl VisibilityVm {
    /// Purement domaine : constructeur co-localisé avec le VM.
    pub fn from_domain(invitations: &CompetitionInvitations, saved: bool) -> Self {
        Self {
            access_mode: match invitations.access_mode {
                AccessMode::Invitation => "invitation".to_string(),
                AccessMode::Open => "open".to_string(),
            },
            requires_validation: invitations.requires_validation.0,
            invited_count: invitations.invited_coaches.len() as u32,
            saved,
        }
    }
}

#[derive(Template)]
#[template(path = "admin/widgets/settings-visibility.html")]
pub struct SettingsVisibilityTemplate {
    pub vm: VisibilityVm,
    pub post_url: String,
}

impl IntoResponse for SettingsVisibilityTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("settings visibility render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_settings_visibility(
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
    rendre(&state, &space_id, &competition_id, &season_id, false).await
}

#[derive(Deserialize)]
pub struct VisibilitySettingsForm {
    /// « invitation » | « open ». Deux chaînes et non un booléen : un libellé
    /// futur — « sur candidature » — ne doit pas exiger de changer le type.
    pub access_mode: String,
    /// « manual » | « automatic ».
    pub requires_validation: String,
}

pub async fn post_settings_visibility(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<VisibilitySettingsForm>,
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

    let cmd = match construire(&season_id, form) {
        Ok(cmd) => cmd,
        Err(statut) => return statut.into_response(),
    };

    match update_visibility_settings_use_case::execute(
        cmd,
        state.competitions.season_repository.as_ref(),
    )
    .await
    {
        Ok(()) => rendre(&state, &space_id, &competition_id, &season_id, true).await,
        Err(UpdateVisibilitySettingsError::SeasonNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(cause) => {
            tracing::error!("settings visibility {competition_id}: {cause:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// **Une valeur inconnue est un `400`, jamais un repli sur le défaut.**
///
/// Se rabattre sur `AccessMode::default()` — qui vaut `Invitation` — semblerait
/// prudent, mais la faute inverse est possible : un formulaire porteur d'une
/// valeur mal orthographiée refermerait une compétition ouverte, ou pire,
/// laisserait croire qu'elle est fermée. Un refus se voit ; un repli, non.
fn construire(
    season_id: &str,
    form: VisibilitySettingsForm,
) -> Result<UpdateVisibilitySettingsCommand, StatusCode> {
    let season = SeasonId::try_new(season_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let access_mode = match form.access_mode.as_str() {
        "invitation" => AccessMode::Invitation,
        "open" => AccessMode::Open,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let requires_validation = match form.requires_validation.as_str() {
        "manual" => RequiresValidation(true),
        "automatic" => RequiresValidation(false),
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    Ok(UpdateVisibilitySettingsCommand {
        season_id: season,
        access_mode,
        requires_validation,
    })
}

/// Le widget, relu depuis la base — jamais reconstruit depuis le formulaire.
/// Après un enregistrement, l'écran doit montrer **ce qui est enregistré**.
async fn rendre(
    state: &AppState,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    saved: bool,
) -> Response {
    let Ok(season_vo) = SeasonId::try_new(season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let invitations = match state
        .competitions
        .season_repository
        .find_invitations(&season_vo)
        .await
    {
        Ok(Some(inv)) => inv,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("settings visibility find {season_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    SettingsVisibilityTemplate {
        vm: VisibilityVm::from_domain(&invitations, saved),
        post_url: AppRoutes::default().competitions.admin_settings_visibility(
            space_id,
            competition_id,
            season_id,
        ),
    }
    .into_response()
}
