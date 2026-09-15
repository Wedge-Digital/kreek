//! Le panneau « Réglages généraux » : autoriser ou non les matchs hors
//! calendrier (carte 550).
//!
//! # Aucun JS, comme le panneau Visibilité
//!
//! Une case à cocher dans un `<form hx-post>`. Le navigateur porte l'état, rien
//! à collecter, rien à synchroniser, rien à éprouver en navigateur pour prouver
//! que la collecte marche. C'est la ligne que l'en-tête de `visibility_panel`
//! pose : « ce qui peut être un `<form>` doit rester un `<form>` ».
//!
//! # Une case décochée ne s'envoie pas
//!
//! C'est la seule subtilité du panneau. Un `<input type="checkbox">` non coché
//! est **absent** du corps de la requête — il ne vaut pas `false`, il n'existe
//! pas. `Option<String>` le capte : `Some(_)` vaut coché, `None` vaut décoché.
//!
//! Un `bool` nu aurait fait échouer la désérialisation du formulaire au lieu de
//! rendre `false`, et le panneau aurait refusé l'interdiction — précisément le
//! geste qu'il existe pour offrir.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_options::{
    AutoriseHorsCalendrier, CompetitionOptions,
};
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::use_cases::settings::update_general_options_use_case::{
    self, UpdateGeneralOptionsCommand, UpdateGeneralOptionsError,
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

pub struct GeneralOptionsVm {
    pub autorise_hors_calendrier: bool,
    pub saved: bool,
}

impl GeneralOptionsVm {
    /// Purement domaine : constructeur co-localisé avec le VM.
    pub fn from_domain(options: &CompetitionOptions, saved: bool) -> Self {
        Self {
            autorise_hors_calendrier: options.autorise_hors_calendrier.0,
            saved,
        }
    }
}

#[derive(Template)]
#[template(path = "admin/widgets/settings-general-options.html")]
pub struct SettingsGeneralOptionsTemplate {
    pub vm: GeneralOptionsVm,
    pub post_url: String,
}

impl IntoResponse for SettingsGeneralOptionsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("settings general options render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_settings_general_options(
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
pub struct GeneralOptionsForm {
    /// Absent quand la case est décochée — voir l'en-tête du module.
    pub autorise_hors_calendrier: Option<String>,
}

pub async fn post_settings_general_options(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<GeneralOptionsForm>,
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

    let cmd = UpdateGeneralOptionsCommand {
        season_id: season,
        autorise_hors_calendrier: AutoriseHorsCalendrier(form.autorise_hors_calendrier.is_some()),
    };

    match update_general_options_use_case::execute(
        cmd,
        state.competitions.season_repository.as_ref(),
    )
    .await
    {
        Ok(()) => rendre(&state, &space_id, &competition_id, &season_id, true).await,
        Err(UpdateGeneralOptionsError::SeasonNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(cause) => {
            tracing::error!("settings general options {competition_id}: {cause:?}");
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
) -> Response {
    let Ok(season) = SeasonId::try_new(season_id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    // `None` vaut « jamais réglée », pas « introuvable » : la colonne est `NULL`
    // sur toutes les saisons antérieures à la migration. Le défaut du domaine
    // autorise, ce qui est exactement leur comportement actuel.
    let options = match state
        .competitions
        .season_repository
        .find_options(&season)
        .await
    {
        Ok(o) => o.unwrap_or_default(),
        Err(e) => {
            tracing::error!("settings general options find {season_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let post_url = AppRoutes::default()
        .competitions
        .admin_settings_general_options(space_id, competition_id, season_id);

    SettingsGeneralOptionsTemplate {
        vm: GeneralOptionsVm::from_domain(&options, saved),
        post_url,
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Le cœur du panneau.** Une case décochée n'est pas envoyée par le
    /// navigateur ; c'est `None` qui porte l'interdiction, et rien d'autre.
    #[test]
    fn une_case_absente_vaut_interdire() {
        let form = GeneralOptionsForm {
            autorise_hors_calendrier: None,
        };
        assert!(!form.autorise_hors_calendrier.is_some());
    }

    /// La valeur envoyée par un navigateur pour une case cochée est « on », mais
    /// rien ne l'impose : seule la présence compte. Le figer évite qu'on se mette
    /// un jour à comparer la chaîne.
    #[test]
    fn la_valeur_de_la_case_cochee_n_est_pas_lue() {
        for valeur in ["on", "true", "1", ""] {
            let form = GeneralOptionsForm {
                autorise_hors_calendrier: Some(valeur.to_string()),
            };
            assert!(
                form.autorise_hors_calendrier.is_some(),
                "la valeur {valeur:?} doit valoir coché"
            );
        }
    }

    #[test]
    fn le_vm_transpose_le_domaine_sans_le_recalculer() {
        let interdit = CompetitionOptions {
            autorise_hors_calendrier: AutoriseHorsCalendrier(false),
        };
        assert!(!GeneralOptionsVm::from_domain(&interdit, false).autorise_hors_calendrier);
        assert!(
            GeneralOptionsVm::from_domain(&CompetitionOptions::default(), true)
                .autorise_hors_calendrier
        );
    }
}
