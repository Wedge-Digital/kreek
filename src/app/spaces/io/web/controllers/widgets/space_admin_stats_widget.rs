//! Les trois compteurs de l'onglet Membres.
//!
//! # Une seule lecture
//!
//! Membres et administrateurs se comptent sur la liste que le dépôt rend déjà.
//! Un `SELECT count(*)` séparé donnerait deux lectures pour une donnée que la
//! première contient.
//!
//! # Le troisième vaut zéro, et c'est délibéré
//!
//! Les invitations d'espace n'existent pas — ni table, ni use case. Elles
//! arrivent avec leur onglet, qui n'aura qu'une requête à ajouter ici. Le zéro
//! est honnête : il n'y a effectivement aucune invitation en attente, faute
//! d'invitations tout court.
//!
//! Livrer deux compteurs et rouvrir la carte plus tard découperait ce widget en
//! deux moitiés dont la seconde n'a pas de valeur propre.

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
    /// Toujours zéro jusqu'à l'onglet Invitations.
    pub invitations_en_attente: usize,
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
        invitations_en_attente: 0,
    }
    .into_response()
}
