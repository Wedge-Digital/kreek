use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::ChangeTeamLogoCommand;

#[derive(Debug)]
pub enum ChangeTeamLogoError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ChangeTeamLogoCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ChangeTeamLogoError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(ChangeTeamLogoError::Repository)?
        .ok_or(ChangeTeamLogoError::TeamNotFound)?;

    let event = team
        .change_logo(cmd.logo_url)
        .map_err(ChangeTeamLogoError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(ChangeTeamLogoError::Repository)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, RosterId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CloudinaryImage, CoachId, SpaceId};
    use crate::app::teams::domain::team::TeamDomainEvent;
    use crate::app::teams::domain::value_objects::{DedicatedFans, Kpo, RosterName, TeamName};
    use crate::app::teams::use_cases::test_doubles::FakeTeamRepository;

    const TEAM: &str = "00000000000000000000000001";
    const LOGO: &str = "https://res.cloudinary.com/demo/image/upload/v1/logo.jpg";

    fn team_id() -> TeamId {
        TeamId::try_new(TEAM).unwrap()
    }

    fn equipe_creee(logo_url: Option<&str>) -> TeamDomainEvent {
        TeamDomainEvent::TeamCreated {
            team_id: team_id(),
            space_id: SpaceId::try_new("00000000000000000000000002").unwrap(),
            competition_id: CompetitionId::try_new("00000000000000000000000003").unwrap(),
            competition_name: "Ligue de Condate".into(),
            season_id: SeasonId::try_new("00000000000000000000000004").unwrap(),
            season_name: "Saison 2025".into(),
            name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
            logo_url: logo_url.map(str::to_string),
            roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
            roster_name: RosterName::try_new("Nains du Granit".to_string()).unwrap(),
            coach_id: CoachId::try_new("00000000000000000000000006").unwrap(),
            coach_name: "Colonel Castor".into(),
            treasury: Kpo(1000),
            dedicated_fans: DedicatedFans::try_new(2).unwrap(),
            rerolls: RerollCount(0),
            apothecaries: ApothecaryCount(0),
            assistants: AssistantCount(0),
            cheerleaders: CheerleaderCount(0),
        }
    }

    fn commande(logo_url: Option<&str>) -> ChangeTeamLogoCommand {
        ChangeTeamLogoCommand {
            team_id: team_id(),
            logo_url: logo_url.map(|u| CloudinaryImage::try_new(u).unwrap()),
        }
    }

    /// Le nouveau logo est écrit dans l'event store, en un seul lot.
    #[tokio::test]
    async fn changer_le_logo_ajoute_l_evenement() {
        let depot = FakeTeamRepository::with_events(vec![equipe_creee(None)]);

        execute(commande(Some(LOGO)), &depot).await.unwrap();

        assert_eq!(depot.batch_count(), 1);
        assert!(matches!(
            depot.appended().as_slice(),
            [TeamDomainEvent::LogoChanged { logo_url: Some(url) }] if url == LOGO
        ));
    }

    /// Retirer le logo s'écrit `None` : c'est ce qui remet la projection à NULL.
    #[tokio::test]
    async fn retirer_le_logo_ajoute_un_evenement_sans_url() {
        let depot = FakeTeamRepository::with_events(vec![equipe_creee(Some(LOGO))]);

        execute(commande(None), &depot).await.unwrap();

        assert!(matches!(
            depot.appended().as_slice(),
            [TeamDomainEvent::LogoChanged { logo_url: None }]
        ));
    }

    /// Une équipe inconnue est refusée, et rien n'est écrit.
    #[tokio::test]
    async fn une_equipe_inconnue_est_refusee_sans_rien_ecrire() {
        let depot = FakeTeamRepository::default();

        let resultat = execute(commande(Some(LOGO)), &depot).await;

        assert!(matches!(resultat, Err(ChangeTeamLogoError::TeamNotFound)));
        assert_eq!(depot.batch_count(), 0);
    }
}
