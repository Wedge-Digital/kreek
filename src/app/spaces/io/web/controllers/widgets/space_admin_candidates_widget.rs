//! La liste des candidats à l'ajout direct.
//!
//! Trois états, et non deux. « Tapez au moins deux caractères » et « aucun coach
//! ne correspond à *xyz* » ne disent pas la même chose, et **seul le second
//! propose de créer un compte** : les confondre ferait proposer une création dès
//! la première frappe.

use crate::app::spaces::context::SpacesContext;
use crate::app::spaces::io::web::builders::{build_candidate_rows, CandidateRowVm};
use crate::app::spaces::io::web::extractors::space_permissions::SpacePermissions;
use crate::app::spaces::routes::Routes;
use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

/// Le nombre maximal de candidats rendus.
///
/// En dur, jamais reçu du client : l'exposer permettrait à n'importe quel
/// appelant de redemander l'annuaire entier.
const PLAFOND: i64 = 20;

/// En deçà, aucune recherche n'est lancée.
///
/// Le seuil s'applique **avant** la lecture, pas après : un seuil qui filtrerait
/// le résultat aurait déjà interrogé l'annuaire, et le garde-fou serait
/// décoratif.
const SEUIL: usize = 2;

#[derive(Deserialize, Default)]
pub struct CandidateSearchQuery {
    #[serde(default)]
    pub q: String,
}

#[derive(Template)]
#[template(path = "widgets/space-admin-candidates.html")]
pub struct SpaceAdminCandidatesTemplate {
    pub routes: Routes,
    pub space_id: String,
    pub candidats: Vec<CandidateRowVm>,
    pub requete: String,
    /// Distinct d'une liste vide : les deux états ne disent pas la même chose.
    pub sous_seuil: bool,
}

impl IntoResponse for SpaceAdminCandidatesTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("space_admin_candidates_widget: rendu impossible: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn space_admin_candidates_widget(
    perms: SpacePermissions,
    State(ctx): State<SpacesContext>,
    Query(query): Query<CandidateSearchQuery>,
) -> Response {
    if !perms.is_admin() {
        return StatusCode::FORBIDDEN.into_response();
    }
    let requete = query.q.trim().to_string();
    let base = SpaceAdminCandidatesTemplate {
        routes: Routes::default(),
        space_id: perms.space_id.to_string(),
        candidats: vec![],
        requete: requete.clone(),
        sous_seuil: true,
    };

    if requete.chars().count() < SEUIL {
        return base.into_response();
    }

    let Ok(lignes) = ctx
        .space_repository
        .search_platform_coaches(&perms.space_id, &requete, PLAFOND)
        .await
    else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    SpaceAdminCandidatesTemplate {
        candidats: build_candidate_rows(lignes),
        sous_seuil: false,
        ..base
    }
    .into_response()
}
