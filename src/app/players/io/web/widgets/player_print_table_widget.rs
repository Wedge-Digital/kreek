//! Le tableau des joueurs d'une fiche imprimée (carte 598) — lecture seule.
//!
//! **Un gabarit distinct** de `player-table-fragment.html`, qui embarque le
//! formulaire d'édition de l'effectif. Les lignes, elles, sont les mêmes :
//! `build_player_rows`, qui résout caractéristiques et compétences une seule
//! fois pour tout le BC.
//!
//! La colonne SPP est le **solde disponible** (R5), le journalier est marqué
//! (R6), et les lignes vides complètent jusqu'à seize (R15).

use crate::app::players::domain::player::TeamId;
use crate::app::players::io::web::widgets::player_table_widget::{
    build_player_rows, Absence, PlayerRowVm,
};
use crate::app::players::use_cases::player_stats_service::ResolvedPlayerStats;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

/// Le plafond d'un effectif : la feuille prévoit toujours de quoi l'atteindre.
const SQUAD_MAX: usize = 16;

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UnavailableMode {
    Show,
    #[default]
    Strike,
    Hide,
}

#[derive(Debug, Default, Deserialize)]
pub struct PlayerPrintQuery {
    #[serde(default)]
    pub unavailable: UnavailableMode,
    #[serde(default)]
    pub blank_rows: bool,
}

pub struct PlayerPrintRowVm {
    pub jersey: Option<i16>,
    pub name: String,
    pub position_name: String,
    pub stats: Option<ResolvedPlayerStats>,
    pub base_skills: Vec<String>,
    pub acquired_skills: Vec<String>,
    pub persistent_injuries: u16,
    pub spp_remaining: u32,
    pub value_kpo: i32,
    pub absence: Option<Absence>,
    pub is_journeyman: bool,
    /// Barré : indisponible, et l'option « barrer » est choisie.
    pub struck: bool,
}

pub struct PlayerPrintTableVm {
    pub rows: Vec<PlayerPrintRowVm>,
    pub blank_rows: usize,
}

impl PlayerPrintTableVm {
    pub fn build(players: Vec<PlayerRowVm>, query: &PlayerPrintQuery) -> Self {
        let rows: Vec<PlayerPrintRowVm> = players
            .into_iter()
            .filter(|p| !(query.unavailable == UnavailableMode::Hide && unavailable(p)))
            .map(|p| PlayerPrintRowVm::from_row(p, query.unavailable))
            .collect();
        // R15 : masquer les indisponibles libère des lignes, qu'on rend vides.
        let blank_rows = if query.blank_rows {
            SQUAD_MAX.saturating_sub(rows.len())
        } else {
            0
        };
        Self { rows, blank_rows }
    }
}

impl PlayerPrintRowVm {
    fn from_row(p: PlayerRowVm, mode: UnavailableMode) -> Self {
        let struck = mode == UnavailableMode::Strike && unavailable(&p);
        let name = if p.personal_name.is_empty() {
            p.position_name.clone()
        } else {
            p.personal_name
        };
        Self {
            jersey: p.jersey,
            name,
            position_name: p.position_name,
            stats: p.stats,
            base_skills: p.base_skills.into_iter().map(|s| s.name).collect(),
            acquired_skills: p
                .acquired_skills
                .into_iter()
                .map(|s| s.skill_name)
                .collect(),
            persistent_injuries: p.persistent_injuries,
            spp_remaining: p.spp,
            value_kpo: p.value_kpo,
            struck,
            absence: p.absence,
            is_journeyman: p.is_journeyman,
        }
    }
}

#[derive(Template)]
#[template(path = "players-print-table.html")]
pub struct PlayerPrintTableTemplate {
    pub vm: PlayerPrintTableVm,
}

impl IntoResponse for PlayerPrintTableTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("players print table render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn get_player_print_table(
    Path((_space_id, team_id)): Path<(String, String)>,
    Query(query): Query<PlayerPrintQuery>,
    State(state): State<AppState>,
) -> Response {
    let players = build_player_rows(&state, &TeamId(team_id)).await;
    PlayerPrintTableTemplate {
        vm: PlayerPrintTableVm::build(players, &query),
    }
    .into_response()
}

/// Indisponible au sens du domaine, comme le décompte de la fiche : un statut
/// que `absence` ne sait pas nommer reste indisponible, et la feuille ne peut
/// pas diverger de l'écran.
fn unavailable(p: &PlayerRowVm) -> bool {
    !p.participation.disponible()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::players::domain::match_impact::PlayerParticipationStatus;

    fn joueur(absent: bool, journalier: bool) -> PlayerRowVm {
        PlayerRowVm {
            player_id: "p".to_string(),
            jersey: Some(7),
            personal_name: String::new(),
            position_name: "Percuteur".to_string(),
            base_skills: vec![],
            acquired_skills: vec![],
            spp: 12,
            value_kpo: 90,
            stats: None,
            participation: if absent {
                PlayerParticipationStatus::MissingNextGame
            } else {
                PlayerParticipationStatus::Available
            },
            absence: absent.then_some(Absence::ProchainMatch),
            persistent_injuries: 0,
            is_journeyman: journalier,
            keywords: String::new(),
        }
    }

    fn requete(unavailable: UnavailableMode, blank_rows: bool) -> PlayerPrintQuery {
        PlayerPrintQuery {
            unavailable,
            blank_rows,
        }
    }

    #[test]
    fn les_lignes_vides_completent_jusqu_a_seize() {
        let onze = (0..11).map(|_| joueur(false, false)).collect();
        let vm = PlayerPrintTableVm::build(onze, &requete(UnavailableMode::Show, true));
        assert_eq!((vm.rows.len(), vm.blank_rows), (11, 5));
    }

    #[test]
    fn sans_l_option_aucune_ligne_vide() {
        let vm = PlayerPrintTableVm::build(
            vec![joueur(false, false)],
            &requete(UnavailableMode::Show, false),
        );
        assert_eq!(vm.blank_rows, 0);
    }

    /// R15 : masquer un indisponible libère sa ligne.
    #[test]
    fn masquer_les_indisponibles_ajoute_des_lignes_vides() {
        let joueurs = vec![joueur(false, false), joueur(true, false)];
        let vm = PlayerPrintTableVm::build(joueurs, &requete(UnavailableMode::Hide, true));
        assert_eq!((vm.rows.len(), vm.blank_rows), (1, 15));
    }

    #[test]
    fn barrer_ne_barre_que_les_indisponibles() {
        let joueurs = vec![joueur(false, false), joueur(true, false)];
        let vm = PlayerPrintTableVm::build(joueurs, &requete(UnavailableMode::Strike, false));
        let barres: Vec<bool> = vm.rows.iter().map(|r| r.struck).collect();
        assert_eq!(barres, vec![false, true]);
    }

    /// Un statut que l'absence ne nomme pas — mort, retraité — reste
    /// indisponible : la feuille compte comme l'écran.
    #[test]
    fn l_indisponibilite_vient_de_la_participation() {
        let mut mort = joueur(false, false);
        mort.participation = PlayerParticipationStatus::Dead;
        let vm = PlayerPrintTableVm::build(vec![mort], &requete(UnavailableMode::Hide, false));
        assert!(vm.rows.is_empty());
    }

    /// R5 et R6 : le solde de SPP, et le journalier marqué.
    #[test]
    fn le_solde_de_spp_et_le_journalier_sont_transposes() {
        let vm = PlayerPrintTableVm::build(
            vec![joueur(false, true)],
            &requete(UnavailableMode::Show, false),
        );
        let ligne = &vm.rows[0];
        assert_eq!(ligne.spp_remaining, 12);
        assert!(ligne.is_journeyman);
        assert_eq!(ligne.name, "Percuteur", "sans nom, le poste en tient lieu");
    }
}
