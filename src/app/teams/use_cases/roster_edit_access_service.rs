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
//! # La règle existe ailleurs, et c'est assumé
//!
//! « Propriétaire, ou administrateur d'espace, ou administrateur de
//! compétition » s'écrit déjà dans `can_spend_spp`. `teams` ne peut pas
//! l'appeler : les deux BCs ne s'importent pas. C'est le prix de la
//! souveraineté, et il tient tant que chaque copie porte un nom qui dit ce
//! qu'elle autorise.
//!
//! Une troisième variante existe — `can_customise` — qui **exclut** le
//! propriétaire : la customisation est un geste de commissaire. Les trois ne
//! sont donc pas interchangeables, et les fondre serait une erreur.

use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
use crate::app::teams::domain::team::Team;
use crate::app::teams::ports::ITeamAccessPort;

/// L'ordre des trois questions n'est pas indifférent.
///
/// La propriété d'abord : c'est la seule qui ne coûte aucun aller-retour —
/// `Team` la porte. Puis l'espace, puis la compétition, chacune
/// court-circuitant les suivantes. Un coach qui regarde sa propre équipe,
/// le cas de loin le plus fréquent, ne déclenche aucune requête.
// Sur une seule ligne : l'axe 11 n'examine que celle qui précède la fonction,
// et une marque repliée sur deux lignes échoue en silence.
// arch:no-instrument — service de lecture : une question de droit, aucune intention métier
pub async fn peut_modifier_effectif(
    team: &Team,
    viewer_id: &CoachId,
    viewer_name: &str,
    access: &dyn ITeamAccessPort,
) -> bool {
    if team.coach_id.to_string() == viewer_id.to_string() {
        return true;
    }

    let Ok(space_id) = SpaceId::try_new(&team.space_id.to_string()) else {
        return false;
    };
    if access.is_space_admin(viewer_id, &space_id).await {
        return true;
    }

    // Une équipe hors compétition n'a pas d'administrateur de compétition :
    // sortir ici évite un aller-retour qui ne pourrait rien rendre.
    let Some(competition_id) = team.competition_id.as_ref() else {
        return false;
    };
    access
        .is_competition_admin(
            &competition_id.to_string(),
            &viewer_id.to_string(),
            viewer_name,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::teams::domain::team::TeamDomainEvent;
    use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const PROPRIETAIRE: &str = "00000000000000000000000006";
    const TIERS: &str = "00000000000000000000000009";
    const ESPACE: &str = "00000000000000000000000002";
    const COMPETITION: &str = "00000000000000000000000003";

    /// Compte ses appels : c'est ce qui permet de vérifier qu'un propriétaire
    /// n'en déclenche aucun, et qu'une équipe sans compétition n'interroge pas
    /// le port des compétitions.
    #[derive(Default)]
    struct PortFactice {
        espace: bool,
        competition: bool,
        par_nom: bool,
        appels_espace: AtomicUsize,
        appels_competition: AtomicUsize,
    }

    #[async_trait]
    impl ITeamAccessPort for PortFactice {
        async fn is_space_admin(&self, _: &CoachId, _: &SpaceId) -> bool {
            self.appels_espace.fetch_add(1, Ordering::SeqCst);
            self.espace
        }

        async fn is_competition_admin(&self, _: &str, _: &str, coach_name: &str) -> bool {
            self.appels_competition.fetch_add(1, Ordering::SeqCst);
            self.competition || (self.par_nom && coach_name == "Colonel Castor")
        }
    }

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

    async fn peut(viewer: &str, nom: &str, port: &PortFactice, avec_competition: bool) -> bool {
        peut_modifier_effectif(
            &equipe(avec_competition),
            &CoachId::try_new(viewer).unwrap(),
            nom,
            port,
        )
        .await
    }

    /// Le cas le plus fréquent, et le seul qui ne coûte aucun aller-retour.
    #[tokio::test]
    async fn le_proprietaire_peut_et_n_interroge_aucun_port() {
        let port = PortFactice::default();
        assert!(peut(PROPRIETAIRE, "Colonel Castor", &port, true).await);
        assert_eq!(port.appels_espace.load(Ordering::SeqCst), 0);
        assert_eq!(port.appels_competition.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn un_admin_d_espace_non_proprietaire_peut() {
        let port = PortFactice {
            espace: true,
            ..Default::default()
        };
        assert!(peut(TIERS, "Quidam", &port, true).await);
        // L'espace suffit : la compétition n'est pas interrogée.
        assert_eq!(port.appels_competition.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn un_admin_de_competition_par_identifiant_peut() {
        let port = PortFactice {
            competition: true,
            ..Default::default()
        };
        assert!(peut(TIERS, "Quidam", &port, true).await);
    }

    /// Une compétition stocke ses administrateurs par identifiant **et** par
    /// nom. Ne reprendre que le premier priverait du bouton ceux qui n'y
    /// figurent que par le second — et l'affichage cesserait de suivre
    /// l'autorisation, le défaut même que cette carte corrige.
    #[tokio::test]
    async fn un_admin_de_competition_par_nom_peut() {
        let port = PortFactice {
            par_nom: true,
            ..Default::default()
        };
        assert!(peut(TIERS, "Colonel Castor", &port, true).await);
    }

    #[tokio::test]
    async fn un_coach_tiers_ne_peut_pas() {
        let port = PortFactice::default();
        assert!(!peut(TIERS, "Quidam", &port, true).await);
        assert_eq!(port.appels_espace.load(Ordering::SeqCst), 1);
        assert_eq!(port.appels_competition.load(Ordering::SeqCst), 1);
    }

    /// Une équipe hors compétition n'a pas d'administrateur de compétition :
    /// l'aller-retour ne pourrait rien rendre, et il n'a pas lieu.
    #[tokio::test]
    async fn une_equipe_sans_competition_n_interroge_pas_le_port_competition() {
        let port = PortFactice::default();
        assert!(!peut(TIERS, "Quidam", &port, false).await);
        assert_eq!(port.appels_competition.load(Ordering::SeqCst), 0);
    }
}
