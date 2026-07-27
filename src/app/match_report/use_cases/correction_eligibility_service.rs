//! Garde-fou de correction d'un rapport publié.
//!
//! Compose deux consultations inter-BC — phase de jeu des équipes, SPP déjà
//! dépensés — en un value object du domaine. Ni les handlers ni les use cases
//! ne manipulent les booléens bruts des ports (cf. CLAUDE.md, « Domain services
//! pour données inter-BCs »).

use crate::app::match_report::domain::value_objects::{
    CorrectionBlocker, CorrectionEligibility, TeamSide,
};
use crate::app::match_report::ports::{IPlayerDataPort, ITeamDataPort};
use crate::app::shared_kernel::common_types::MatchReportId;
use crate::app::shared_kernel::team::TeamId;

/// Réponses des ports pour un camp. Regroupées pour que le verdict se lise sans
/// suivre quatre variables parallèles, et se teste sans mock de port.
struct SideStatus {
    spp_spent:      Result<bool, String>,
    in_improvement: Result<bool, String>,
}

pub async fn evaluate(
    home_team_id:    &TeamId,
    away_team_id:    &TeamId,
    match_report_id: &MatchReportId,
    team_data:       &dyn ITeamDataPort,
    player_data:     &dyn IPlayerDataPort,
) -> CorrectionEligibility {
    // Les `join!` imbriqués gardent les quatre consultations concurrentes : les
    // enchaîner doublerait la latence d'une page déjà chargée.
    let (home, away) = tokio::join!(
        status_of(home_team_id, match_report_id, team_data, player_data),
        status_of(away_team_id, match_report_id, team_data, player_data),
    );
    verdict_from(home, away)
}

async fn status_of(
    team_id:         &TeamId,
    match_report_id: &MatchReportId,
    team_data:       &dyn ITeamDataPort,
    player_data:     &dyn IPlayerDataPort,
) -> SideStatus {
    let team = team_id.to_string();
    let mr_id = match_report_id.to_string();
    let (spp_spent, in_improvement) = tokio::join!(
        player_data.has_spent_spp_since_match(&team, &mr_id),
        team_data.is_team_in_player_improvement(&team),
    );
    SideStatus { spp_spent, in_improvement }
}

/// Home avant away : un seul message est affiché, et lever le premier blocage
/// ne rendrait pas le rapport corrigeable pour autant.
fn verdict_from(home: SideStatus, away: SideStatus) -> CorrectionEligibility {
    for (side, status) in [(TeamSide::Home, home), (TeamSide::Away, away)] {
        if let Some(blocker) = blocker_for_side(side, &status) {
            return CorrectionEligibility::Blocked(blocker);
        }
    }
    CorrectionEligibility::Eligible
}

/// Un motif certain l'emporte sur l'indéterminé : les deux bloquent
/// identiquement, mais « votre adversaire a dépensé ses SPP » est actionnable là
/// où « impossible de vérifier » ne l'est pas.
///
/// Un port qui n'a pas pu répondre bloque quand même — le garde-fou échoue
/// fermé.
fn blocker_for_side(side: TeamSide, status: &SideStatus) -> Option<CorrectionBlocker> {
    if matches!(status.spp_spent, Ok(true)) {
        return Some(CorrectionBlocker::SppAlreadySpent { side });
    }
    if matches!(status.in_improvement, Ok(false)) {
        return Some(CorrectionBlocker::PhaseAdvanced { side });
    }
    if status.spp_spent.is_err() || status.in_improvement.is_err() {
        return Some(CorrectionBlocker::EligibilityUnknown);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sain() -> SideStatus {
        SideStatus { spp_spent: Ok(false), in_improvement: Ok(true) }
    }

    fn spp_depenses() -> SideStatus {
        SideStatus { spp_spent: Ok(true), in_improvement: Ok(true) }
    }

    fn phase_avancee() -> SideStatus {
        SideStatus { spp_spent: Ok(false), in_improvement: Ok(false) }
    }

    fn port_en_erreur() -> SideStatus {
        SideStatus { spp_spent: Err("indisponible".into()), in_improvement: Ok(true) }
    }

    #[test]
    fn deux_camps_sains_donnent_eligible() {
        assert_eq!(verdict_from(sain(), sain()), CorrectionEligibility::Eligible);
    }

    #[test]
    fn spp_depenses_cote_home() {
        assert_eq!(
            verdict_from(spp_depenses(), sain()),
            CorrectionEligibility::Blocked(CorrectionBlocker::SppAlreadySpent {
                side: TeamSide::Home
            })
        );
    }

    #[test]
    fn phase_avancee_cote_away() {
        assert_eq!(
            verdict_from(sain(), phase_avancee()),
            CorrectionEligibility::Blocked(CorrectionBlocker::PhaseAdvanced {
                side: TeamSide::Away
            })
        );
    }

    /// Règle 3a : un seul message quand les deux camps bloquent, celui de home.
    #[test]
    fn home_l_emporte_quand_les_deux_camps_bloquent() {
        assert_eq!(
            verdict_from(phase_avancee(), spp_depenses()),
            CorrectionEligibility::Blocked(CorrectionBlocker::PhaseAdvanced {
                side: TeamSide::Home
            })
        );
    }

    /// Règle 3b : pour un même camp, les SPP passent avant la phase — c'est la
    /// cause que le coach peut relier à une action concrète.
    #[test]
    fn spp_l_emportent_sur_la_phase_pour_un_meme_camp() {
        let les_deux = SideStatus { spp_spent: Ok(true), in_improvement: Ok(false) };
        assert_eq!(
            verdict_from(les_deux, sain()),
            CorrectionEligibility::Blocked(CorrectionBlocker::SppAlreadySpent {
                side: TeamSide::Home
            })
        );
    }

    /// Règle 12 : échouer fermé.
    #[test]
    fn un_port_en_erreur_bloque() {
        assert_eq!(
            verdict_from(port_en_erreur(), sain()),
            CorrectionEligibility::Blocked(CorrectionBlocker::EligibilityUnknown)
        );
    }

    #[test]
    fn un_motif_certain_l_emporte_sur_l_indetermine() {
        let certain_et_indetermine = SideStatus {
            spp_spent:      Ok(true),
            in_improvement: Err("indisponible".into()),
        };
        assert_eq!(
            verdict_from(certain_et_indetermine, sain()),
            CorrectionEligibility::Blocked(CorrectionBlocker::SppAlreadySpent {
                side: TeamSide::Home
            })
        );
    }

    #[test]
    fn une_erreur_cote_away_bloque_aussi() {
        assert_eq!(
            verdict_from(sain(), port_en_erreur()),
            CorrectionEligibility::Blocked(CorrectionBlocker::EligibilityUnknown)
        );
    }
}
