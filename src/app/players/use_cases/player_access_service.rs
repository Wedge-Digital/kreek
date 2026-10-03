//! Qui peut agir sur les joueurs d'une équipe (carte 572).
//!
//! La règle d'admin n'est pas écrite ici : « admin d'espace ou de compétition »
//! se demande au service commun, `is_admin`. Ce fichier n'ajoute que ce qui
//! appartient à `players` — **le coach de l'équipe**, inclus pour dépenser des
//! SPP et éditer l'effectif, exclu pour customiser un joueur : un coach qui
//! s'ajouterait des compétences gratuitement ne serait pas la même fonction.
//!
//! Les deux prédicats reçoivent le port et non l'`AppState` : c'est ce qui les
//! rend testables sur `FakeAdminAccess`. Ils n'avaient aucun test auparavant.

use crate::app::players::ports::TeamRosterInfoDto;
use crate::app::shared_kernel::bloodbowl::admin_access::{is_admin, IAdminAccessPort};
use crate::app::shared_kernel::identity::ids::{CoachId, EntityId, SpaceId};

/// Coach de l'équipe, ou admin de son espace ou de sa compétition.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn can_spend_spp(
    access: &dyn IAdminAccessPort,
    user_id: &CoachId,
    space_id: &SpaceId,
    team: &TeamRosterInfoDto,
) -> bool {
    if team.coach_id == user_id.to_string() {
        return true;
    }
    can_customise(access, user_id, space_id, team).await
}

/// Admin de l'espace ou de la compétition de l'équipe — **sans** son coach.
///
/// Une compétition illisible compte comme absente : l'admin d'espace garde son
/// droit, l'admin de compétition ne peut être reconnu.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn can_customise(
    access: &dyn IAdminAccessPort,
    user_id: &CoachId,
    space_id: &SpaceId,
    team: &TeamRosterInfoDto,
) -> bool {
    let competition = team
        .competition_id
        .as_deref()
        .and_then(|id| EntityId::try_new(id).ok());
    is_admin(access, user_id, space_id, competition.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;

    const COACH: &str = "00000000000000000000000031";
    const OTHER: &str = "00000000000000000000000032";
    const SPACE: &str = "00000000000000000000000033";
    const COMPETITION: &str = "00000000000000000000000034";

    fn id(v: &str) -> EntityId {
        EntityId::try_new(v).unwrap()
    }

    fn team() -> TeamRosterInfoDto {
        TeamRosterInfoDto {
            team_name: "Les Korrigans FC".into(),
            coach_id: COACH.into(),
            competition_id: Some(COMPETITION.into()),
            in_player_improvement_phase: true,
        }
    }

    #[tokio::test]
    async fn coach_spends_spp_without_querying_the_port() {
        let port = FakeAdminAccess::new();
        assert!(can_spend_spp(&port, &id(COACH), &id(SPACE), &team()).await);
        assert_eq!(port.space_calls(), 0);
    }

    #[tokio::test]
    async fn coach_cannot_customise_their_team() {
        let port = FakeAdminAccess::new();
        assert!(!can_customise(&port, &id(COACH), &id(SPACE), &team()).await);
    }

    #[tokio::test]
    async fn space_admin_spends_and_customises() {
        let port = FakeAdminAccess::new().space_admin(&id(OTHER), &id(SPACE));
        assert!(can_spend_spp(&port, &id(OTHER), &id(SPACE), &team()).await);
        assert!(can_customise(&port, &id(OTHER), &id(SPACE), &team()).await);
    }

    #[tokio::test]
    async fn competition_admin_spends_and_customises() {
        let port = FakeAdminAccess::new().competition_admin(&id(OTHER), &id(COMPETITION));
        assert!(can_spend_spp(&port, &id(OTHER), &id(SPACE), &team()).await);
        assert!(can_customise(&port, &id(OTHER), &id(SPACE), &team()).await);
    }

    #[tokio::test]
    async fn other_coach_can_do_neither() {
        let port = FakeAdminAccess::new();
        assert!(!can_spend_spp(&port, &id(OTHER), &id(SPACE), &team()).await);
        assert!(!can_customise(&port, &id(OTHER), &id(SPACE), &team()).await);
    }

    /// Une équipe hors compétition : la seconde question n'est pas posée.
    #[tokio::test]
    async fn without_competition_competition_admin_is_not_queried() {
        let port = FakeAdminAccess::new();
        let without_competition = TeamRosterInfoDto {
            competition_id: None,
            ..team()
        };
        assert!(!can_customise(&port, &id(OTHER), &id(SPACE), &without_competition).await);
        assert_eq!(port.competition_calls(), 0);
    }
}
