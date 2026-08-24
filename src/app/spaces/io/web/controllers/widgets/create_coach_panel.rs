//! Le panneau de création de compte : le formulaire de l'hôte, plus ce qui
//! appartient à ce BC.
//!
//! Le formulaire lui-même vient de l'hôte, qui le rend sous forme de markup — ce
//! BC ne sait ni comment un compte se crée, ni quelles règles le gouvernent. Il
//! y adjoint le **sélecteur de profil**, qui est son concept à lui, et l'écoute
//! de l'événement qui suivra.
//!
//! # Pourquoi un endpoint plutôt qu'un rendu avec la page
//!
//! Le pré-remplissage dépend de ce qui a été cherché, donc d'une action
//! postérieure au chargement. Un fragment rendu une fois avec la page ne peut
//! pas en tenir compte.
//!
//! # La répartition du terme cherché
//!
//! C'est **ce BC** qui décide si la saisie est un pseudo ou une adresse, selon
//! la présence d'un `@`. L'hôte reçoit deux champs déjà triés : lui faire
//! trancher reviendrait à lui faire deviner une intention qu'il n'observe pas.

use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use crate::app::spaces::io::web::host_layout::CoachPrefill;
use crate::app::spaces::routes::Routes;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct PanelQuery {
    #[serde(default)]
    pub q: String,
}

#[derive(Template)]
#[template(path = "widgets/space-admin-create-coach.html")]
pub struct CreateCoachPanelTemplate {
    pub routes: Routes,
    pub space_id: String,
    /// Le fragment de l'hôte, rendu tel quel.
    pub formulaire: String,
}

impl IntoResponse for CreateCoachPanelTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("create_coach_panel: rendu impossible: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn create_coach_panel(
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
    Query(query): Query<PanelQuery>,
) -> Response {
    if !perms.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let saisie = query.q.trim();
    let prefill = if saisie.contains('@') {
        CoachPrefill {
            pseudo: None,
            email: Some(saisie),
        }
    } else if saisie.is_empty() {
        CoachPrefill {
            pseudo: None,
            email: None,
        }
    } else {
        CoachPrefill {
            pseudo: Some(saisie),
            email: None,
        }
    };

    CreateCoachPanelTemplate {
        routes: Routes::default(),
        space_id: perms.space_id.to_string(),
        formulaire: ctx.host_layout.coach_creation_widget(prefill),
    }
    .into_response()
}
