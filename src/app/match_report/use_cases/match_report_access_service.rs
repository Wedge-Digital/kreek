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
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::shared_kernel::bloodbowl::admin_access::{is_admin, IAdminAccessPort};
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::state::AppState;

pub struct AccesRapportDeps<'a> {
    /// Qui est admin — le service commun à tout kreek (carte 570).
    pub admin_access: &'a dyn IAdminAccessPort,
    pub team_data: &'a dyn ITeamDataPort,
}

impl<'a> AccesRapportDeps<'a> {
    pub fn from_state(state: &'a AppState) -> Self {
        Self {
            admin_access: state.match_report.admin_access.as_ref(),
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
    if est_administrateur(deps, user, space_id, &scope.competition_id).await {
        return true;
    }
    is_coach_of_either_team(deps, scope, &user.id.to_string()).await
}

/// Admin d'espace ou de compétition — **sans** les coachs des deux équipes.
/// La règle est celle de tout kreek, `is_admin` (carte 570) ; un identifiant
/// illisible refuse.
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
    let (Ok(space), Ok(competition)) = (
        EntityId::try_new(space_id),
        EntityId::try_new(competition_id),
    ) else {
        return false;
    };
    is_admin(deps.admin_access, &user.id, &space, Some(&competition)).await
}

/// Coach de l'une des deux équipes, ou des deux — un camp, quel qu'il soit.
async fn is_coach_of_either_team(
    deps: &AccesRapportDeps<'_>,
    scope: &PorteeRapport,
    user_id: &str,
) -> bool {
    camp_du_coach(deps.team_data, scope, user_id).await != CampDuCoach::Aucun
}

/// Le camp d'un coach dans ce match. Un coach peut tenir les deux équipes.
///
/// L'admin n'a pas de camp : il agit sur le match, pas pour une équipe —
/// `is_authorized` le traite à part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampDuCoach {
    Domicile,
    Visiteur,
    LesDeux,
    Aucun,
}

/// Qui est ce coach dans ce match. Une erreur de port vaut « pas coach » de
/// cette équipe : un contrôle d'accès échoue fermé.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn camp_du_coach(
    team_data: &dyn ITeamDataPort,
    scope: &PorteeRapport,
    user_id: &str,
) -> CampDuCoach {
    let (home, away) = tokio::join!(
        team_data.is_coach_of_team(&scope.home_team_id, user_id),
        team_data.is_coach_of_team(&scope.away_team_id, user_id),
    );
    match (home.unwrap_or(false), away.unwrap_or(false)) {
        (true, true) => CampDuCoach::LesDeux,
        (true, false) => CampDuCoach::Domicile,
        (false, true) => CampDuCoach::Visiteur,
        (false, false) => CampDuCoach::Aucun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::ports::{JourneymanPositionDto, RosterPositionDto, TeamInfoDto};

    const DOMICILE: &str = "domicile";
    const VISITEUR: &str = "visiteur";

    /// Les équipes que l'utilisateur coache, et celle dont le port échoue.
    struct FakeTeamData {
        coached: Vec<&'static str>,
        failing: Option<&'static str>,
    }

    #[async_trait::async_trait]
    impl ITeamDataPort for FakeTeamData {
        async fn is_coach_of_team(&self, team_id: &str, _: &str) -> Result<bool, String> {
            if self.failing == Some(team_id) {
                return Err("port indisponible".to_string());
            }
            Ok(self.coached.contains(&team_id))
        }
        async fn is_team_ready_to_play(&self, _: &str) -> Result<bool, String> {
            Ok(true)
        }
        async fn is_team_in_player_improvement(&self, _: &str) -> Result<bool, String> {
            Ok(false)
        }
        async fn find_team_info(&self, _: &str) -> Option<TeamInfoDto> {
            None
        }
        async fn find_team_value(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_team_treasury(&self, _: &str) -> Option<u32> {
            None
        }
        async fn find_journeyman_position(&self, _: &str) -> Option<JourneymanPositionDto> {
            None
        }
        async fn find_roster_positions(&self, _: &str) -> Vec<RosterPositionDto> {
            vec![]
        }
        async fn find_team_season(&self, _: &str) -> Option<String> {
            None
        }
    }

    fn scope() -> PorteeRapport {
        PorteeRapport {
            competition_id: "competition".to_string(),
            home_team_id: DOMICILE.to_string(),
            away_team_id: VISITEUR.to_string(),
        }
    }

    async fn camp(coached: Vec<&'static str>, failing: Option<&'static str>) -> CampDuCoach {
        camp_du_coach(&FakeTeamData { coached, failing }, &scope(), "coach").await
    }

    #[tokio::test]
    async fn the_home_coach_is_on_the_home_side() {
        assert_eq!(camp(vec![DOMICILE], None).await, CampDuCoach::Domicile);
    }

    #[tokio::test]
    async fn the_away_coach_is_on_the_away_side() {
        assert_eq!(camp(vec![VISITEUR], None).await, CampDuCoach::Visiteur);
    }

    #[tokio::test]
    async fn a_coach_of_both_teams_holds_both_sides() {
        assert_eq!(
            camp(vec![DOMICILE, VISITEUR], None).await,
            CampDuCoach::LesDeux
        );
    }

    #[tokio::test]
    async fn a_third_party_has_no_side() {
        assert_eq!(camp(vec![], None).await, CampDuCoach::Aucun);
    }

    /// Le port en erreur sur une équipe ne donne pas ce camp — même au coach
    /// qui le tient : un contrôle d'accès échoue fermé.
    #[tokio::test]
    async fn a_failing_port_denies_that_side_only() {
        assert_eq!(
            camp(vec![DOMICILE, VISITEUR], Some(DOMICILE)).await,
            CampDuCoach::Visiteur
        );
    }
}
