//! Le panneau « Informations générales » : renommer, changer le logo.
//!
//! Le premier des cinq, et celui qui pose leur forme commune — un `GET` qui
//! rend le widget, un `POST` qui l'échange par lui-même, et l'emplacement
//! d'erreur réservé sous le champ.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::domain::competition_repository_port::CompetitionBaseInfo;
use crate::app::competitions::domain::season_repository_port::SeasonBaseInfo;
use crate::app::competitions::io::web::admin::admin_page::require_admin_access;
use crate::app::competitions::use_cases::settings::update_general_settings_use_case::{
    self, UpdateGeneralSettingsCommand, UpdateGeneralSettingsError,
};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::bloodbowl::competition_name::CompetitionName;
use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, SeasonId};
use crate::app::shared_kernel::bloodbowl::season_name::SeasonName;
use crate::app::shared_kernel::identity::ids::{CloudinaryImage, SpaceId};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

// ── Vue ───────────────────────────────────────────────────────────────────────

pub struct AdminRowVm {
    pub coach_name: String,
    /// Le premier de la liste : celui qui a créé la compétition. Affiché seul,
    /// ce panneau n'édite pas les administrateurs.
    pub is_owner: bool,
}

pub struct GeneralVm {
    pub name: String,
    pub season_name: String,
    pub logo_url: String,
    pub admins: Vec<AdminRowVm>,
    /// Les motifs de refus, **chacun sous son champ**. Un message d'URL de logo
    /// affiché sous le nom enverrait corriger le mauvais champ.
    pub name_error: Option<String>,
    pub logo_error: Option<String>,
}

impl GeneralVm {
    /// Purement domaine : constructeur co-localisé avec le VM, conformément au
    /// `CLAUDE.md`. Aucun DTO de port n'entre ici.
    pub fn from_domain(competition: &CompetitionBaseInfo, season: &SeasonBaseInfo) -> Self {
        Self {
            name: competition.name.clone(),
            season_name: season.name.clone(),
            logo_url: competition.logo.clone().unwrap_or_default(),
            admins: competition
                .admin_names
                .iter()
                .enumerate()
                .map(|(rang, nom)| AdminRowVm {
                    coach_name: nom.clone(),
                    is_owner: rang == 0,
                })
                .collect(),
            name_error: None,
            logo_error: None,
        }
    }
}

#[derive(Template)]
#[template(path = "admin/widgets/settings-general.html")]
pub struct SettingsGeneralTemplate {
    pub vm: GeneralVm,
    pub post_url: String,
}

impl IntoResponse for SettingsGeneralTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("settings general render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn get_settings_general(
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
    rendre(
        &state,
        &space_id,
        &competition_id,
        &season_id,
        Refus::aucun(),
    )
    .await
}

#[derive(Deserialize)]
pub struct GeneralSettingsForm {
    pub name: String,
    pub season_name: String,
    pub logo_url: String,
}

pub async fn post_settings_general(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    Form(form): Form<GeneralSettingsForm>,
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

    let cmd = match construire(&space_id, &competition_id, &season_id, form) {
        Ok(cmd) => cmd,
        // Un value object refusé est un motif à afficher, pas une erreur de
        // protocole : le widget re-rendu porte le message sous le champ visé.
        Err(refus) => return rendre(&state, &space_id, &competition_id, &season_id, refus).await,
    };

    match update_general_settings_use_case::execute(
        cmd,
        state.competitions.competition_repository.as_ref(),
        state.competitions.season_repository.as_ref(),
    )
    .await
    {
        Ok(()) => {
            rendre(
                &state,
                &space_id,
                &competition_id,
                &season_id,
                Refus::aucun(),
            )
            .await
        }
        Err(cause) => match motif_de(&cause) {
            Some(refus) => rendre(&state, &space_id, &competition_id, &season_id, refus).await,
            None => {
                tracing::error!("settings general {competition_id}: {cause:?}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    }
}

/// Un refus, et **le champ qu'il vise**. Sans cette distinction, un message
/// s'affiche sous un champ que le commissaire n'a pas à corriger.
#[derive(Default)]
pub struct Refus {
    pub name: Option<String>,
    pub logo: Option<String>,
}

impl Refus {
    fn aucun() -> Self {
        Self::default()
    }
    fn nom(motif: impl Into<String>) -> Self {
        Self {
            name: Some(motif.into()),
            logo: None,
        }
    }
    fn logo(motif: impl Into<String>) -> Self {
        Self {
            name: None,
            logo: Some(motif.into()),
        }
    }
}

/// Les value objects sont construits par leurs smart constructors — c'est le
/// travail du handler (« traducteur de protocole »), et le domaine reçoit des
/// types déjà valides.
fn construire(
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    form: GeneralSettingsForm,
) -> Result<UpdateGeneralSettingsCommand, Refus> {
    // Les trois identifiants viennent du chemin, déjà contrôlé par
    // `require_admin_access` : un échec ici n'est pas une saisie fautive.
    let (Ok(comp), Ok(space), Ok(season)) = (
        CompetitionId::try_new(competition_id),
        SpaceId::try_new(space_id),
        SeasonId::try_new(season_id),
    ) else {
        return Err(Refus::nom("Compétition introuvable."));
    };
    Ok(UpdateGeneralSettingsCommand {
        competition_id: comp,
        space_id: space,
        season_id: season,
        name: CompetitionName::try_new(form.name)
            .map_err(|e| Refus::nom(format!("Nom de compétition invalide : {e}")))?,
        season_name: SeasonName::try_new(form.season_name)
            .map_err(|e| Refus::nom(format!("Nom de saison invalide : {e}")))?,
        logo: CloudinaryImage::try_new(form.logo_url)
            .map_err(|_| Refus::logo("Le logo doit être une image Cloudinary."))?,
    })
}

/// `None` pour ce qui n'est pas un motif affichable : une compétition
/// introuvable ou une ligne corrompue relèvent du `500`, pas d'un message sous
/// un champ que le commissaire ne peut pas corriger.
fn motif_de(cause: &UpdateGeneralSettingsError) -> Option<Refus> {
    match cause {
        UpdateGeneralSettingsError::NameAlreadyTaken => {
            Some(Refus::nom("Ce nom est déjà pris dans cet espace."))
        }
        _ => None,
    }
}

/// Le widget, relu depuis la base — jamais reconstruit depuis le formulaire.
///
/// Après un enregistrement, l'écran doit montrer **ce qui est enregistré** ; et
/// après un refus, ce qui l'est toujours. Réafficher la saisie refusée
/// laisserait croire qu'elle a pris.
async fn rendre(
    state: &AppState,
    space_id: &str,
    competition_id: &str,
    season_id: &str,
    refus: Refus,
) -> Response {
    let (Ok(comp_id), Ok(season_vo)) = (
        CompetitionId::try_new(competition_id),
        SeasonId::try_new(season_id),
    ) else {
        return StatusCode::BAD_REQUEST.into_response();
    };

    let competition = match state
        .competitions
        .competition_repository
        .find_base_info(&comp_id)
        .await
    {
        Ok(Some(info)) => info,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("settings general find competition {competition_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let season = match state
        .competitions
        .season_repository
        .find_base_info(&season_vo)
        .await
    {
        Ok(Some(info)) => info,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("settings general find season {season_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let mut vm = GeneralVm::from_domain(&competition, &season);
    vm.name_error = refus.name;
    vm.logo_error = refus.logo;
    SettingsGeneralTemplate {
        vm,
        post_url: AppRoutes::default().competitions.admin_settings_general(
            space_id,
            competition_id,
            season_id,
        ),
    }
    .into_response()
}
