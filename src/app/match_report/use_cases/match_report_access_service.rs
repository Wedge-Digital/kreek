//! Qui a le droit d'agir sur un rapport de match.
//!
//! **Une seule définition**, appelée par le récapitulatif et par l'annulation
//! (carte 433). Trois prédicats identiques divergeraient, et c'est ce qui donne
//! un bouton visible sur une action refusée — la carte 389 vient d'en corriger
//! un de cette famille.
//!
//! Le prédicat vivait dans `recap_controller` ; il en a été **déplacé**, pas
//! réécrit.

use crate::app::auth::domain::user::User;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::ports::{ICompetitionDataPort, ISpaceAdminPort, ITeamDataPort};
use crate::state::AppState;

pub struct AccesRapportDeps<'a> {
    pub space_admin: &'a dyn ISpaceAdminPort,
    pub competition_data: &'a dyn ICompetitionDataPort,
    pub team_data: &'a dyn ITeamDataPort,
}

impl<'a> AccesRapportDeps<'a> {
    pub fn from_state(state: &'a AppState) -> Self {
        Self {
            space_admin: state.match_report.space_admin.as_ref(),
            competition_data: state.match_report.competition_data.as_ref(),
            team_data: state.match_report.team_data.as_ref(),
        }
    }
}

pub struct PorteeRapport {
    pub competition_id: String,
    pub home_team_id: String,
    pub away_team_id: String,
}

impl PorteeRapport {
    /// Depuis un état **annulable**, que la portée du récapitulatif ne sait pas
    /// lire : celle-ci refuse `PreMatch`, or c'est l'état où l'on abandonne le
    /// plus souvent un rapport ouvert par erreur.
    pub fn depuis_etat_annulable(state: &MatchReportState) -> Option<Self> {
        match state {
            MatchReportState::PreMatch(pm) => Some(Self {
                competition_id: pm.competition_id.to_string(),
                home_team_id: pm.home_team_id.to_string(),
                away_team_id: pm.away_team_id.to_string(),
            }),
            MatchReportState::ReadyToPublish(rtp) => Some(Self {
                competition_id: rtp.competition_id.to_string(),
                home_team_id: rtp.home_team_id.to_string(),
                away_team_id: rtp.away_team_id.to_string(),
            }),
            _ => None,
        }
    }
}

/// Autorisé si l'utilisateur est admin d'espace, admin de la compétition du
/// rapport, ou coach de l'une des deux équipes concernées.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn is_authorized(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    scope: &PorteeRapport,
) -> bool {
    let user_id = user.id.to_string();
    if deps.space_admin.is_space_admin(&user_id, space_id).await {
        return true;
    }
    if is_competition_admin(deps, &scope.competition_id, &user_id).await {
        return true;
    }
    is_coach_of_either_team(deps, scope, &user_id).await
}

async fn is_competition_admin(
    deps: &AccesRapportDeps<'_>,
    competition_id: &str,
    user_id: &str,
) -> bool {
    deps.competition_data
        .is_competition_admin(competition_id, user_id)
        .await
        .unwrap_or(false)
}

/// Une erreur de port vaut « pas coach » : un contrôle d'accès échoue fermé.
async fn is_coach_of_either_team(
    deps: &AccesRapportDeps<'_>,
    scope: &PorteeRapport,
    user_id: &str,
) -> bool {
    let (home, away) = tokio::join!(
        deps.team_data
            .is_coach_of_team(&scope.home_team_id, user_id),
        deps.team_data
            .is_coach_of_team(&scope.away_team_id, user_id),
    );
    home.unwrap_or(false) || away.unwrap_or(false)
}
