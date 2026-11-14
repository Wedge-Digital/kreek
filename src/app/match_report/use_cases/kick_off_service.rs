//! Le coup d'envoi en « Chacun son équipe » (carte 656, § 12.3 de E22).
//!
//! L'agrégat dit si le match a commencé (`has_kicked_off`), mais il lui faut
//! les équipes à qui leur tier offre des coups de pouce, qu'il ne connaît pas.
//! Ce service les lit, et dit où renvoyer un coach arrivé au match trop tôt.

use crate::app::match_report::context::MatchReportContext;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::value_objects::{EntryMode, TeamSide};
use crate::app::match_report::ports::{ICompetitionDataPort, ITeamDataPort};
use crate::app::match_report::use_cases::record_fan_factor_use_case::offers_inducements;
use crate::app::shared_kernel::bloodbowl::team::TeamId;

/// Les deux ports d'où se lit le tier de chaque équipe.
pub struct KickOffPorts<'a> {
    pub team_data: &'a dyn ITeamDataPort,
    pub competition_data: &'a dyn ICompetitionDataPort,
}

impl<'a> KickOffPorts<'a> {
    pub fn from_context(ctx: &'a MatchReportContext) -> Self {
        Self {
            team_data: ctx.team_data.as_ref(),
            competition_data: ctx.competition_data.as_ref(),
        }
    }
}

/// Où renvoyer un coach arrivé au match avant le coup d'envoi.
#[derive(Debug, PartialEq, Eq)]
pub enum KickOffDetour {
    /// Son avant-match : son jet reste à lancer, ou il attend celui de l'autre.
    PreMatch,
    /// Les coups de pouce de son équipe : à choisir, ou l'attente de l'autre.
    Inducements { team_id: String },
}

/// Les équipes à qui leur tier offre des coups de pouce. Vide hors de
/// « Chacun son équipe » : le coup d'envoi n'y attend rien, et les ports ne
/// sont pas interrogés.
// arch:no-instrument — service de lecture : une question de tier, aucune intention métier
pub async fn offered_inducements(
    pm: &MatchReportPreMatch,
    ports: &KickOffPorts<'_>,
) -> Vec<TeamId> {
    if pm.entry_mode != EntryMode::TeamEntry {
        return vec![];
    }
    let (home, away) = (pm.home_team_id.to_string(), pm.away_team_id.to_string());
    let (home_offered, away_offered) = tokio::join!(
        offers_inducements(pm, &home, ports.team_data, ports.competition_data),
        offers_inducements(pm, &away, ports.team_data, ports.competition_data),
    );
    [
        (home_offered, &pm.home_team_id),
        (away_offered, &pm.away_team_id),
    ]
    .into_iter()
    .filter_map(|(offered, team_id)| offered.then(|| team_id.clone()))
    .collect()
}

/// Où renvoyer qui arrive au match pour le camp `side` : `None` une fois le
/// match commencé.
// arch:no-instrument — service de lecture : une question d'étape, aucune intention métier
pub async fn kick_off_detour(
    pm: &MatchReportPreMatch,
    side: TeamSide,
    ports: &KickOffPorts<'_>,
) -> Option<KickOffDetour> {
    let offered = offered_inducements(pm, ports).await;
    if pm.has_kicked_off(&offered) {
        return None;
    }
    Some(detour_of(pm, side, &offered))
}

/// Les coups de pouce dès que son jet est lancé et que son tier en offre ;
/// sinon son avant-match.
fn detour_of(pm: &MatchReportPreMatch, side: TeamSide, offered: &[TeamId]) -> KickOffDetour {
    let team_id = pm.team_id_of(side);
    if pm.fan_roll_of(side).is_some() && offered.contains(team_id) {
        return KickOffDetour::Inducements {
            team_id: team_id.to_string(),
        };
    }
    KickOffDetour::PreMatch
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{D3Roll, InducementPurchase};
    use crate::app::match_report::use_cases::test_doubles::pre_match;

    fn team_entry() -> MatchReportPreMatch {
        let mut pm = pre_match();
        pm.entry_mode = EntryMode::TeamEntry;
        pm
    }

    fn rolled(mut pm: MatchReportPreMatch, side: TeamSide) -> MatchReportPreMatch {
        let roll = Some(D3Roll::try_new(2).unwrap());
        match side {
            TeamSide::Home => pm.home_fan_roll = roll,
            TeamSide::Away => pm.away_fan_roll = roll,
        }
        pm
    }

    fn with_inducements(mut pm: MatchReportPreMatch, side: TeamSide) -> MatchReportPreMatch {
        let none: Vec<InducementPurchase> = vec![];
        match side {
            TeamSide::Home => pm.home_inducements = Some(none),
            TeamSide::Away => pm.away_inducements = Some(none),
        }
        pm
    }

    #[test]
    fn sans_jet_le_coach_retourne_a_son_avant_match() {
        let pm = team_entry();
        let offered = vec![pm.home_team_id.clone()];
        assert_eq!(
            detour_of(&pm, TeamSide::Home, &offered),
            KickOffDetour::PreMatch
        );
    }

    #[test]
    fn jet_lance_et_coups_de_pouce_offerts_menent_aux_coups_de_pouce() {
        let pm = rolled(team_entry(), TeamSide::Home);
        let offered = vec![pm.home_team_id.clone()];
        assert_eq!(
            detour_of(&pm, TeamSide::Home, &offered),
            KickOffDetour::Inducements {
                team_id: pm.home_team_id.to_string()
            }
        );
    }

    #[test]
    fn jet_lance_sans_coups_de_pouce_offerts_reste_a_l_avant_match() {
        let pm = rolled(team_entry(), TeamSide::Home);
        assert_eq!(detour_of(&pm, TeamSide::Home, &[]), KickOffDetour::PreMatch);
    }

    #[test]
    fn le_match_commence_quand_tout_est_enregistre() {
        let pm = rolled(rolled(team_entry(), TeamSide::Home), TeamSide::Away);
        let offered = vec![pm.home_team_id.clone(), pm.away_team_id.clone()];
        assert!(!pm.has_kicked_off(&offered));
        let pm = with_inducements(with_inducements(pm, TeamSide::Home), TeamSide::Away);
        assert!(pm.has_kicked_off(&offered));
    }

    #[test]
    fn une_equipe_sans_coups_de_pouce_offerts_n_a_rien_a_valider() {
        let pm = rolled(rolled(team_entry(), TeamSide::Home), TeamSide::Away);
        let pm = with_inducements(pm, TeamSide::Home);
        assert!(pm.has_kicked_off(std::slice::from_ref(&pm.home_team_id)));
    }

    #[test]
    fn en_je_saisis_tout_le_match_est_toujours_commence() {
        let pm = pre_match();
        assert!(pm.has_kicked_off(&[pm.home_team_id.clone()]));
    }
}
