//! Le widget d'édition du logo d'équipe : un `GET` qui rend le widget, un
//! `POST` qui l'échange par lui-même — même forme que
//! `competitions/io/web/admin/settings/general_panel.rs`.

use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::CloudinaryImage;
use crate::app::teams::use_cases::change_team_logo::{self, ChangeTeamLogoError};
use crate::app::teams::use_cases::commands::ChangeTeamLogoCommand;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::Form;
use serde::Deserialize;

pub struct TeamLogoVm {
    pub logo_url: String,
    pub initials: String,
    pub error: Option<String>,
    /// Rendu en édition dès le chargement uniquement après un refus de
    /// validation, pour garder le message d'erreur sous les yeux.
    pub start_in_edit_mode: bool,
}

#[derive(Template)]
#[template(path = "widgets/team-logo.html")]
pub struct TeamLogoTemplate {
    pub vm: TeamLogoVm,
    pub post_url: String,
}

impl IntoResponse for TeamLogoTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("team logo widget render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_team_logo_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Response {
    rendre(&state, &space_id, &team_id, None).await
}

#[derive(Deserialize)]
pub struct TeamLogoForm {
    /// Vide = retrait du logo. Cloudinary écrit toujours une valeur non vide
    /// dans le champ caché ; un champ vide ne peut venir que du bouton de
    /// retrait, qui le vide explicitement côté client.
    pub logo_url: String,
}

pub async fn post_team_logo_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    Form(form): Form<TeamLogoForm>,
) -> Response {
    let logo_url = match construire(form) {
        Ok(logo_url) => logo_url,
        Err(motif) => return rendre(&state, &space_id, &team_id, Some(motif)).await,
    };

    let cmd = ChangeTeamLogoCommand {
        team_id: match crate::app::shared_kernel::bloodbowl::team::TeamId::try_new(&team_id) {
            Ok(id) => id,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        },
        logo_url,
    };

    match change_team_logo::execute(cmd, state.teams.team_repository.as_ref()).await {
        Ok(()) => rendre(&state, &space_id, &team_id, None).await,
        Err(ChangeTeamLogoError::TeamNotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("post_team_logo_widget {team_id}: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn construire(form: TeamLogoForm) -> Result<Option<CloudinaryImage>, String> {
    if form.logo_url.trim().is_empty() {
        return Ok(None);
    }
    CloudinaryImage::try_new(form.logo_url)
        .map(Some)
        .map_err(|_| "Le logo doit être une image Cloudinary.".to_string())
}

/// Le widget, relu depuis la base — jamais reconstruit depuis le formulaire
/// (même raison que `general_panel.rs` : un refus ne doit pas laisser croire
/// qu'une saisie a été enregistrée).
async fn rendre(state: &AppState, space_id: &str, team_id: &str, error: Option<String>) -> Response {
    let team = match state.teams.team_repository.find_by_id(team_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("team logo widget: chargement de {team_id}: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let start_in_edit_mode = error.is_some();
    let vm = TeamLogoVm {
        logo_url: team.logo_url.clone().unwrap_or_default(),
        initials: team.initials.clone(),
        error,
        start_in_edit_mode,
    };
    TeamLogoTemplate {
        vm,
        post_url: AppRoutes::default()
            .teams
            .team_logo_widget(space_id, team_id),
    }
    .into_response()
}
