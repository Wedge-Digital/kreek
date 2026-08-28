//! La page d'assemblage de l'onglet Paramètres.
//!
//! **Aucun calcul, aucun JS** : elle compose cinq conteneurs, chacun rempli par
//! son propre endpoint (pattern « page d'assemblage à widgets » du `CLAUDE.md`).
//! Tant que les cartes 421 à 425 n'ont pas livré leur panneau, les conteneurs
//! restent vides — l'onglet s'ouvre, ne montre rien, et ne casse rien.
//!
//! **Ils ne portent pas encore leur `hx-get`.** La conception les câblait dès
//! cette carte, vers des routes qui n'existent pas : chaque ouverture aurait
//! émis cinq requêtes rendant `404`, et `request_log` journalise toutes les
//! requêtes, statut compris. Cinq lignes `status=404` par ouverture, pour un
//! onglet qui n'a rien à montrer — exactement le bruit que l'épic E11 s'est
//! employée à bannir. Le rendu à l'écran est identique dans les deux cas : sur
//! un `404`, htmx n'échange rien. Chaque carte de panneau apporte donc son
//! `hx-get` en même temps que sa route.

use crate::app::auth::auth_backend::AuthSession;
use crate::app::competitions::io::web::admin::admin_page::{
    render_admin_page, require_admin_access,
};
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "admin/settings.html")]
pub struct SettingsTabTemplate {
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    /// L'URL de chaque panneau, calculée ici plutôt que dans le template : la
    /// page d'assemblage ne porte aucune logique, pas même une construction
    /// d'URL. Les quatre autres suivront.
    pub general_url: String,
    pub ranking_url: String,
    pub pools_url: String,
}

impl IntoResponse for SettingsTabTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("settings tab render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

/// **`require_admin_access` garde aussi ce `GET`.** Sans contrôle sur le chemin
/// htmx, le changement d'onglet contournerait l'autorisation : seul le
/// chargement de page complète serait gardé, et c'est justement par le fragment
/// qu'on navigue.
pub async fn settings_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if headers.contains_key("hx-request") {
        if let Err(resp) = require_admin_access(
            &auth_session,
            &space_id,
            &competition_id,
            &season_id,
            &state,
        )
        .await
        {
            return resp;
        }

        let general_url = AppRoutes::default().competitions.admin_settings_general(
            &space_id,
            &competition_id,
            &season_id,
        );
        let pools_url = AppRoutes::default().competitions.admin_settings_pools(
            &space_id,
            &competition_id,
            &season_id,
        );
        let ranking_url = AppRoutes::default().competitions.admin_settings_ranking(
            &space_id,
            &competition_id,
            &season_id,
        );
        return SettingsTabTemplate {
            space_id,
            competition_id,
            season_id,
            general_url,
            ranking_url,
            pools_url,
        }
        .into_response();
    }

    render_admin_page(
        auth_session,
        &space_id,
        &competition_id,
        &season_id,
        "settings",
        &state,
    )
    .await
}
