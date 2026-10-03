//! Qui peut agir sur les joueurs d'une équipe (carte 572).
//!
//! La règle d'admin n'est pas écrite ici : « admin d'espace ou de compétition »
//! se demande au service commun, `est_admin`. Ce fichier n'ajoute que ce qui
//! appartient à `players` — **le coach de l'équipe**, inclus pour dépenser des
//! SPP et éditer l'effectif, exclu pour customiser un joueur : un coach qui
//! s'ajouterait des compétences gratuitement ne serait pas la même fonction.
//!
//! Les deux prédicats reçoivent le port et non l'`AppState` : c'est ce qui les
//! rend testables sur `FakeAdminAccess`. Ils n'avaient aucun test auparavant.

use crate::app::players::ports::TeamRosterInfoDto;
use crate::app::shared_kernel::bloodbowl::admin_access::{est_admin, IAdminAccessPort};
use crate::app::shared_kernel::identity::ids::{CoachId, EntityId, SpaceId};

/// Coach de l'équipe, ou admin de son espace ou de sa compétition.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn peut_depenser_des_spp(
    access: &dyn IAdminAccessPort,
    user_id: &CoachId,
    space_id: &SpaceId,
    team: &TeamRosterInfoDto,
) -> bool {
    if team.coach_id == user_id.to_string() {
        return true;
    }
    peut_customiser(access, user_id, space_id, team).await
}

/// Admin de l'espace ou de la compétition de l'équipe — **sans** son coach.
///
/// Une compétition illisible compte comme absente : l'admin d'espace garde son
/// droit, l'admin de compétition ne peut être reconnu.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn peut_customiser(
    access: &dyn IAdminAccessPort,
    user_id: &CoachId,
    space_id: &SpaceId,
    team: &TeamRosterInfoDto,
) -> bool {
    let competition = team
        .competition_id
        .as_deref()
        .and_then(|id| EntityId::try_new(id).ok());
    est_admin(access, user_id, space_id, competition.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;

    const COACH: &str = "00000000000000000000000031";
    const TIERS: &str = "00000000000000000000000032";
    const ESPACE: &str = "00000000000000000000000033";
    const COMPETITION: &str = "00000000000000000000000034";

    fn id(v: &str) -> EntityId {
        EntityId::try_new(v).unwrap()
    }

    fn equipe() -> TeamRosterInfoDto {
        TeamRosterInfoDto {
            team_name: "Les Korrigans FC".into(),
            coach_id: COACH.into(),
            competition_id: Some(COMPETITION.into()),
            in_player_improvement_phase: true,
        }
    }

    #[tokio::test]
    async fn le_coach_depense_les_spp_de_son_equipe_sans_interroger_le_port() {
        let port = FakeAdminAccess::new();
        assert!(peut_depenser_des_spp(&port, &id(COACH), &id(ESPACE), &equipe()).await);
        assert_eq!(port.appels_espace(), 0);
    }

    #[tokio::test]
    async fn le_coach_ne_customise_pas_son_equipe() {
        let port = FakeAdminAccess::new();
        assert!(!peut_customiser(&port, &id(COACH), &id(ESPACE), &equipe()).await);
    }

    #[tokio::test]
    async fn l_admin_d_espace_depense_et_customise() {
        let port = FakeAdminAccess::new().admin_espace(&id(TIERS), &id(ESPACE));
        assert!(peut_depenser_des_spp(&port, &id(TIERS), &id(ESPACE), &equipe()).await);
        assert!(peut_customiser(&port, &id(TIERS), &id(ESPACE), &equipe()).await);
    }

    #[tokio::test]
    async fn l_admin_de_la_competition_depense_et_customise() {
        let port = FakeAdminAccess::new().admin_competition(&id(TIERS), &id(COMPETITION));
        assert!(peut_depenser_des_spp(&port, &id(TIERS), &id(ESPACE), &equipe()).await);
        assert!(peut_customiser(&port, &id(TIERS), &id(ESPACE), &equipe()).await);
    }

    #[tokio::test]
    async fn un_tiers_ne_peut_ni_l_un_ni_l_autre() {
        let port = FakeAdminAccess::new();
        assert!(!peut_depenser_des_spp(&port, &id(TIERS), &id(ESPACE), &equipe()).await);
        assert!(!peut_customiser(&port, &id(TIERS), &id(ESPACE), &equipe()).await);
    }

    /// Une équipe hors compétition : la seconde question n'est pas posée.
    #[tokio::test]
    async fn sans_competition_l_admin_de_competition_n_est_pas_interroge() {
        let port = FakeAdminAccess::new();
        let hors_competition = TeamRosterInfoDto {
            competition_id: None,
            ..equipe()
        };
        assert!(!peut_customiser(&port, &id(TIERS), &id(ESPACE), &hors_competition).await);
        assert_eq!(port.appels_competition(), 0);
    }
}
