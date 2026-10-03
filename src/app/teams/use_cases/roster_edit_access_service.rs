//! Qui peut modifier l'effectif d'une équipe.
//!
//! # Ce que cette règle décide, et ce qu'elle ne décide pas
//!
//! Elle décide de l'**affichage** du bouton, jamais de l'écriture. Celle-ci
//! reste gardée par `can_spend_spp`, côté `players`, et cette carte n'y touche
//! pas. Masquer un bouton n'est pas un contrôle d'accès — c'est ce qui évite à
//! un visiteur de saisir un effectif entier pour découvrir un 403 à
//! l'enregistrement.
//!
//! # La règle d'admin n'est plus écrite ici (carte 570)
//!
//! « Administrateur d'espace ou de compétition » se demande au service commun,
//! `shared_kernel::bloodbowl::admin_access::is_admin`. Ce fichier n'ajoute que
//! ce qui appartient à `teams` : **le propriétaire**, inclus pour l'effectif
//! (`peut_modifier_effectif`), exclu pour les gestes de commissaire
//! (`is_team_admin`). Les deux ne sont pas interchangeables, et les
//! fondre serait une erreur.

use crate::app::shared_kernel::bloodbowl::admin_access::{is_admin, IAdminAccessPort};
use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::teams::domain::team::Team;

/// La propriété d'abord : c'est la seule question qui ne coûte aucun
/// aller-retour — `Team` la porte. Un coach qui regarde sa propre équipe, le cas
/// de loin le plus fréquent, ne déclenche aucune requête. Ensuite, la règle
/// d'admin commune à tout kreek (carte 570).
// Sur une seule ligne : l'axe 11 n'examine que celle qui précède la fonction,
// et une marque repliée sur deux lignes échoue en silence.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn peut_modifier_effectif(
    team: &Team,
    viewer_id: &CoachId,
    access: &dyn IAdminAccessPort,
) -> bool {
    if team.coach_id.to_string() == viewer_id.to_string() {
        return true;
    }
    is_team_admin(team, viewer_id, access).await
}

/// Admin de l'espace ou de la compétition de l'équipe — **sans** son
/// propriétaire. C'est le droit des gestes de commissaire : ajuster la
/// trésorerie, renvoyer l'équipe.
///
/// L'espace et la compétition sont lus dans l'agrégat, pas dans le chemin : ce
/// sont ceux de l'équipe, quelle que soit l'URL par laquelle on la regarde.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn is_team_admin(
    team: &Team,
    viewer_id: &CoachId,
    access: &dyn IAdminAccessPort,
) -> bool {
    let Ok(space_id) = SpaceId::try_new(&team.space_id.to_string()) else {
        return false;
    };
    is_admin(access, viewer_id, &space_id, team.competition_id.as_ref()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::admin_access::FakeAdminAccess;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::teams::domain::team::TeamDomainEvent;
    use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};

    const PROPRIETAIRE: &str = "00000000000000000000000006";
    const TIERS: &str = "00000000000000000000000009";
    const ESPACE: &str = "00000000000000000000000002";
    const COMPETITION: &str = "00000000000000000000000003";

    fn equipe(avec_competition: bool) -> Team {
        let created = TeamDomainEvent::TeamCreated {
            team_id: TeamId::try_new("00000000000000000000000001").unwrap(),
            space_id: SpaceId::try_new(ESPACE).unwrap(),
            competition_id: CompetitionId::try_new(COMPETITION).unwrap(),
            competition_name: "Ligue de Condate".to_string(),
            season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
            season_name: "Saison 2025".to_string(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: None,
            roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
            roster_name: RosterName::try_new("Elfes Sylvestres".to_string()).unwrap(),
            coach_id: CoachId::try_new(PROPRIETAIRE).unwrap(),
            coach_name: "Colonel Castor".to_string(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(3),
            apothecaries: ApothecaryCount(1),
            assistants: AssistantCount(2),
            cheerleaders: CheerleaderCount(3),
        };
        let mut team = Team::hydrate(&[created]).unwrap();
        if !avec_competition {
            team.competition_id = None;
        }
        team
    }

    fn espace() -> SpaceId {
        SpaceId::try_new(ESPACE).unwrap()
    }

    fn competition() -> CompetitionId {
        CompetitionId::try_new(COMPETITION).unwrap()
    }

    fn tiers() -> CoachId {
        CoachId::try_new(TIERS).unwrap()
    }

    async fn peut(viewer: &str, port: &FakeAdminAccess, avec_competition: bool) -> bool {
        peut_modifier_effectif(
            &equipe(avec_competition),
            &CoachId::try_new(viewer).unwrap(),
            port,
        )
        .await
    }

    /// Le cas le plus fréquent, et le seul qui ne coûte aucun aller-retour.
    #[tokio::test]
    async fn le_proprietaire_peut_et_n_interroge_aucun_port() {
        let port = FakeAdminAccess::new();
        assert!(peut(PROPRIETAIRE, &port, true).await);
        assert_eq!(port.space_calls(), 0);
        assert_eq!(port.competition_calls(), 0);
    }

    #[tokio::test]
    async fn un_admin_d_espace_non_proprietaire_peut() {
        let port = FakeAdminAccess::new().space_admin(&tiers(), &espace());
        assert!(peut(TIERS, &port, true).await);
        // L'espace suffit : la compétition n'est pas interrogée.
        assert_eq!(port.competition_calls(), 0);
    }

    #[tokio::test]
    async fn un_admin_de_competition_par_identifiant_peut() {
        let port = FakeAdminAccess::new().competition_admin(&tiers(), &competition());
        assert!(peut(TIERS, &port, true).await);
    }

    #[tokio::test]
    async fn un_coach_tiers_ne_peut_pas() {
        let port = FakeAdminAccess::new();
        assert!(!peut(TIERS, &port, true).await);
        assert_eq!(port.space_calls(), 1);
        assert_eq!(port.competition_calls(), 1);
    }

    /// Une équipe hors compétition n'a pas d'administrateur de compétition :
    /// l'aller-retour ne pourrait rien rendre, et il n'a pas lieu.
    #[tokio::test]
    async fn une_equipe_sans_competition_n_interroge_pas_le_port_competition() {
        let port = FakeAdminAccess::new();
        assert!(!peut(TIERS, &port, false).await);
        assert_eq!(port.competition_calls(), 0);
    }

    // ── Les gestes de commissaire ─────────────────────────────────────────

    /// Le propriétaire n'est pas commissaire de sa propre équipe.
    #[tokio::test]
    async fn owner_is_not_admin_of_their_team() {
        let port = FakeAdminAccess::new();
        let proprietaire = CoachId::try_new(PROPRIETAIRE).unwrap();
        assert!(!is_team_admin(&equipe(true), &proprietaire, &port).await);
    }

    /// Décision du 2026-10-03 : l'admin de la compétition de l'équipe ajuste sa
    /// trésorerie et la renvoie, comme l'admin d'espace.
    #[tokio::test]
    async fn competition_admin_is_team_admin() {
        let port = FakeAdminAccess::new().competition_admin(&tiers(), &competition());
        assert!(is_team_admin(&equipe(true), &tiers(), &port).await);
    }
}
