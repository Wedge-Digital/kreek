//! Les deux compteurs de l'onglet Membres.
//!
//! # Une seule lecture
//!
//! Membres et administrateurs se comptent sur la liste que le dépôt rend déjà.
//! Un `SELECT count(*)` séparé donnerait deux lectures pour une donnée que la
//! première contient.
//!
//! # Il y en avait un troisième
//!
//! « Invitations en attente », figé à zéro, posé en prévision d'un onglet
//! Invitations auquel il n'aurait eu qu'une requête à ajouter. Cet onglet a été
//! abandonné : le compteur n'attendait donc plus rien, et un zéro perpétuel
//! sans destination promet une fonction qui n'arrive pas.

use crate::app::shared_kernel::identity::authorization::SpaceProfile;
use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "widgets/space-admin-stats.html")]
pub struct SpaceAdminStatsTemplate {
    pub membres: usize,
    pub administrateurs: usize,
}

impl IntoResponse for SpaceAdminStatsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("space_admin_stats_widget: rendu impossible: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn space_admin_stats_widget(
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
) -> Response {
    if !perms.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(lignes) = ctx
        .space_repository
        .list_members_with_profile(&perms.space_id)
        .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let administrateurs = lignes
        .iter()
        .filter(|l| l.profile == SpaceProfile::SpaceAdmin.as_str())
        .count();

    SpaceAdminStatsTemplate {
        membres: lignes.len(),
        administrateurs,
    }
    .into_response()
}
