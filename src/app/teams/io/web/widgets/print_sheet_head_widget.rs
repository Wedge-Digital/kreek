//! L'en-tête d'une fiche imprimée (carte 597) — identité, chiffres, staff.
//!
//! **Une vue transpose, elle ne dérive pas.** La VE est la valeur persistée par
//! le domaine (R4), les alignables ceux que `SquadPresence::alignable` déclare
//! tels (R14), les prix du staff ceux du catalogue du roster. Rien n'est
//! recalculé ici ni dans le gabarit.

use crate::app::teams::io::web::print_builders::build_sheet_head_vm;
use crate::app::teams::io::web::widgets::print_preview_widget::SheetRole;
use crate::app::teams::use_cases::team_print_service::{self, TeamPrintError};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

pub struct StaffPrintLineVm {
    pub label: &'static str,
    pub quantity: u8,
    /// `None` sans catalogue : le gabarit imprime un tiret.
    pub unit_price_kpo: Option<u32>,
}

pub struct TeamSheetHeadVm {
    pub role_label: &'static str,
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,
    pub roster_name: String,
    pub coach_name: String,
    pub competition_name: Option<String>,
    pub season_name: Option<String>,
    pub team_value_kpo: u32,
    /// Sur toutes les fiches (R8) : elle est déjà publique sur la fiche d'équipe.
    pub treasury_kpo: u32,
    pub dedicated_fans: u8,
    pub alignable: usize,
    pub squad_size: usize,
    pub rerolls: u8,
    pub reroll_price_kpo: Option<u32>,
    pub staff: Vec<StaffPrintLineVm>,
}

#[derive(Debug, Default, Deserialize)]
pub struct SheetHeadQuery {
    #[serde(default)]
    pub role: SheetRole,
}

#[derive(Template)]
#[template(path = "widgets/teams-print-sheet-head.html")]
pub struct SheetHeadTemplate {
    pub vm: TeamSheetHeadVm,
}

impl IntoResponse for SheetHeadTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("teams print sheet head render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_print_sheet_head(
    Path((_space_id, team_id)): Path<(String, String)>,
    Query(query): Query<SheetHeadQuery>,
    State(state): State<AppState>,
) -> Response {
    let ctx = &state.teams;
    let loaded = team_print_service::load_sheet_head(
        ctx.team_repository.as_ref(),
        ctx.roster_catalog_port.as_ref(),
        ctx.squad_port.as_ref(),
        &team_id,
    )
    .await;
    match loaded {
        Ok(data) => SheetHeadTemplate {
            vm: build_sheet_head_vm(&data, query.role),
        }
        .into_response(),
        Err(TeamPrintError::Repository(cause)) => {
            tracing::error!("teams print sheet head {team_id}: {cause}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
