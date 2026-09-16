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

/// Admin d'espace ou de compétition — **sans** les coachs des deux équipes.
///
/// `is_authorized` répond « a le droit d'agir sur ce rapport », ce qui inclut
/// les deux coachs : c'est leur match. Celui-ci répond « a le droit de changer
/// la sélection », ce qui les exclut quand la compétition interdit les matchs
/// hors calendrier (carte 550).
///
/// Deux prédicats et non un paramètre : mêler les deux questions dans une
/// fonction unique cacherait **laquelle** a répondu, et c'est le reproche que le
/// port de `ranking` fait déjà à `require_admin_access`.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn est_administrateur(
    deps: &AccesRapportDeps<'_>,
    user: &User,
    space_id: &str,
    competition_id: &str,
) -> bool {
    let user_id = user.id.to_string();
    deps.space_admin.is_space_admin(&user_id, space_id).await
        || is_competition_admin(deps, competition_id, &user_id).await
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
