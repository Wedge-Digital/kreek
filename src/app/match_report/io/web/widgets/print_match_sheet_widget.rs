//! La feuille de match imprimée (carte 608) — une page A4, deux colonnes.
//!
//! **Le fragment rend sa feuille** (`print-slot` › `print-frame` ›
//! `print-sheet`), comme les autres fragments d'impression de `match_report`.
//! Deux exemplaires : la page hôte pose deux fois l'emplacement, `copy` dit
//! lequel.
//!
//! Le statut financier est le résultat des fonctions du domaine, transposé ;
//! le bonus de non-temporisation et le plafond des fans sont ceux du code.

use crate::app::match_report::domain::underdog::Side;
use crate::app::match_report::domain::value_objects::NoStallingBonus;
use crate::app::match_report::use_cases::match_sheet_service::{
    self, MatchSheet, MoneyStatus, SheetPorts, SheetRequest, SheetTeam,
};
use crate::app::match_report::use_cases::print_models::{MatchReminders, SheetPlayer, SppScale};
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

/// La feuille prévoit seize lignes, le plafond d'un effectif.
const TALLY_ROWS: usize = 16;

/// Qui reçoit. **Inconnu par défaut** : seule l'option « prochain adversaire »
/// le sait, et une équipe choisie à la main ne doit pas recevoir d'office.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HomeSide {
    Own,
    Other,
    #[default]
    Unknown,
}

impl HomeSide {
    /// Le camp de mon équipe, pour les fonctions du domaine.
    pub fn own_side(self) -> Option<Side> {
        match self {
            Self::Own => Some(Side::Home),
            Self::Other => Some(Side::Away),
            Self::Unknown => None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct MatchSheetQuery {
    #[serde(default)]
    pub home: HomeSide,
    #[serde(default)]
    pub round_id: Option<String>,
    /// 1 ou 2 quand deux exemplaires sont demandés.
    #[serde(default)]
    pub copy: Option<u8>,
}

pub struct SheetPlayerVm {
    pub jersey: String,
    pub name: String,
    pub is_journeyman: bool,
}

pub struct SheetColumnVm {
    pub name: Option<String>,
    pub subtitle: Option<String>,
    pub dedicated_fans: Option<u32>,
    pub money: MoneyStatus,
    pub players: Vec<SheetPlayerVm>,
    pub blank_rows: usize,
    pub spp: Option<SppScale>,
}

pub struct MatchSheetVm {
    pub round_label: Option<String>,
    pub copy_label: Option<String>,
    /// Mon équipe, puis l'autre — à gauche, puis à droite.
    pub columns: [SheetColumnVm; 2],
    pub reminders: MatchReminders,
    pub no_stalling_bonus_kpo: u32,
}

impl MatchSheetVm {
    pub fn build(sheet: MatchSheet, copy: Option<u8>) -> Self {
        Self {
            round_label: sheet.round.map(|r| {
                format!(
                    "{} · {} · {}",
                    r.competition_name, r.season_name, r.round_name
                )
            }),
            copy_label: copy.map(|n| format!("exemplaire {n} / 2")),
            columns: [
                column(Some(sheet.own), sheet.own_money),
                column(sheet.other, sheet.other_money),
            ],
            reminders: sheet.reminders,
            no_stalling_bonus_kpo: NoStallingBonus::AMOUNT_KPO,
        }
    }
}

/// Sans équipe — l'adversaire inconnu —, une colonne à remplir à la main.
fn column(team: Option<SheetTeam>, money: MoneyStatus) -> SheetColumnVm {
    match team {
        Some(t) => team_column(t, money),
        None => SheetColumnVm {
            name: None,
            subtitle: None,
            dedicated_fans: None,
            money,
            players: Vec::new(),
            blank_rows: TALLY_ROWS,
            spp: None,
        },
    }
}

fn team_column(t: SheetTeam, money: MoneyStatus) -> SheetColumnVm {
    let subtitle_text = subtitle(&t);
    let players: Vec<SheetPlayerVm> = t.players.into_iter().map(sheet_player).collect();
    SheetColumnVm {
        subtitle: Some(subtitle_text),
        name: Some(t.name),
        dedicated_fans: Some(t.dedicated_fans),
        money,
        blank_rows: TALLY_ROWS.saturating_sub(players.len()),
        players,
        spp: Some(t.spp),
    }
}

/// « Granitiers · Coach Brunhild · VE 1440 kPo » — sans « Coach » quand le
/// nom manque, plutôt qu'un mot seul.
fn subtitle(t: &SheetTeam) -> String {
    let coach = if t.coach_name.is_empty() {
        String::new()
    } else {
        format!(" · Coach {}", t.coach_name)
    };
    let tv = t
        .team_value_kpo
        .map_or_else(|| "—".to_string(), |v| format!("{v} kPo"));
    format!("{}{coach} · VE {tv}", t.roster_name)
}

fn sheet_player(p: SheetPlayer) -> SheetPlayerVm {
    SheetPlayerVm {
        jersey: p.jersey.map(|j| j.to_string()).unwrap_or_default(),
        name: p.name,
        is_journeyman: p.is_journeyman,
    }
}

#[derive(Template)]
#[template(path = "match-report-print-sheet.html")]
pub struct MatchSheetTemplate {
    pub vm: MatchSheetVm,
}

impl IntoResponse for MatchSheetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("match report print sheet render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

async fn render(
    state: &AppState,
    own_id: &str,
    other_id: Option<&str>,
    query: &MatchSheetQuery,
) -> Response {
    let ctx = &state.match_report;
    let ports = SheetPorts {
        team_data: ctx.team_data.as_ref(),
        player_data: ctx.player_data.as_ref(),
        competition_data: ctx.competition_data.as_ref(),
        catalog: ctx.reference_catalog.as_ref(),
    };
    let request = SheetRequest {
        own_id,
        other_id,
        own_side: query.home.own_side(),
        round_id: query.round_id.as_deref(),
    };
    match match_sheet_service::build(&ports, &request).await {
        Ok(sheet) => MatchSheetTemplate {
            vm: MatchSheetVm::build(sheet, query.copy),
        }
        .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn get_print_match_sheet(
    Path((_space_id, team_id)): Path<(String, String)>,
    Query(query): Query<MatchSheetQuery>,
    State(state): State<AppState>,
) -> Response {
    render(&state, &team_id, None, &query).await
}

/// `{other_team_id}` est contrôlé par `space_scope` (carte 594).
pub async fn get_print_match_sheet_with(
    Path((_space_id, team_id, other_team_id)): Path<(String, String, String)>,
    Query(query): Query<MatchSheetQuery>,
    State(state): State<AppState>,
) -> Response {
    render(&state, &team_id, Some(&other_team_id), &query).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equipe(joueurs: usize) -> SheetTeam {
        SheetTeam {
            name: "Les Tailleurs de Roc".into(),
            roster_name: "Granitiers".into(),
            coach_name: "Brunhild".into(),
            team_value_kpo: Some(1440),
            dedicated_fans: 4,
            players: (0..joueurs)
                .map(|i| SheetPlayer {
                    jersey: Some(i as i16 + 1),
                    name: format!("J{i}"),
                    is_journeyman: i == 0,
                })
                .collect(),
            spp: SppScale::default(),
        }
    }

    #[test]
    fn la_colonne_complete_jusqu_a_seize_lignes() {
        let col = column(Some(equipe(12)), MoneyStatus::Unknown);
        assert_eq!((col.players.len(), col.blank_rows), (12, 4));
        assert!(
            col.players[0].is_journeyman,
            "le journalier est marqué (R6)"
        );
        assert_eq!(
            col.subtitle.as_deref(),
            Some("Granitiers · Coach Brunhild · VE 1440 kPo")
        );
    }

    #[test]
    fn sans_nom_de_coach_le_sous_titre_ne_dit_pas_coach() {
        let mut t = equipe(11);
        t.coach_name = String::new();
        assert_eq!(subtitle(&t), "Granitiers · VE 1440 kPo");
    }

    #[test]
    fn une_ve_absente_s_imprime_en_tiret() {
        let mut t = equipe(11);
        t.team_value_kpo = None;
        assert_eq!(subtitle(&t), "Granitiers · Coach Brunhild · VE —");
    }

    #[test]
    fn sans_home_dans_l_url_personne_ne_recoit() {
        assert_eq!(MatchSheetQuery::default().home.own_side(), None);
        assert_eq!(HomeSide::Own.own_side(), Some(Side::Home));
        assert_eq!(HomeSide::Other.own_side(), Some(Side::Away));
    }

    #[test]
    fn sans_adversaire_la_colonne_se_remplit_a_la_main() {
        let col = column(None, MoneyStatus::Unknown);
        assert!(col.name.is_none() && col.spp.is_none());
        assert_eq!(col.blank_rows, 16);
    }

    #[test]
    fn le_bonus_de_non_temporisation_est_celui_du_domaine() {
        let sheet = MatchSheet {
            own: equipe(11),
            other: None,
            own_money: MoneyStatus::Unknown,
            other_money: MoneyStatus::Unknown,
            round: None,
            reminders: MatchReminders::default(),
        };
        let vm = MatchSheetVm::build(sheet, Some(2));
        assert_eq!(vm.no_stalling_bonus_kpo, NoStallingBonus::AMOUNT_KPO);
        assert_eq!(vm.copy_label.as_deref(), Some("exemplaire 2 / 2"));
        assert!(vm.round_label.is_none());
    }
}
