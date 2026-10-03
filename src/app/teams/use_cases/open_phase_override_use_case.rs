use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::OpenPhaseOverrideCommand;

pub enum OpenPhaseOverrideError {
    TeamNotFound,
    /// L'équipe n'est plus prête à jouer : le contrôleur l'affiche au pied du
    /// panneau.
    Domain(DomainError),
    Repository(RepositoryError),
}

/// Ouvre à la main une phase d'après-match (carte 577).
///
/// **Il ne décide rien** : « depuis prête à jouer seulement » et le choix de
/// l'événement appartiennent à l'agrégat. **Il ne contrôle aucun droit** : le
/// contrôleur a appliqué `require_team_admin`, comme pour l'ajustement de
/// trésorerie. `append` écrit l'événement et `team_proj` dans une même
/// transaction ; aucun app event ne sort — `players` lit la phase en direct.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: OpenPhaseOverrideCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), OpenPhaseOverrideError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(OpenPhaseOverrideError::Repository)?
        .ok_or(OpenPhaseOverrideError::TeamNotFound)?;

    let event = team
        .open_phase_override(cmd.phase, cmd.admin_id, cmd.admin_name, cmd.reason)
        .map_err(OpenPhaseOverrideError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(OpenPhaseOverrideError::Repository)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RosterId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::staff_counts::{
        ApothecaryCount, AssistantCount, CheerleaderCount, RerollCount,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::coach_name::CoachName;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId, UserId};
    use crate::app::teams::domain::team::{GamePhase, OverridablePhase, TeamDomainEvent};
    use crate::app::teams::domain::value_objects::{
        DedicatedFans, Kpo, OverrideReason, RosterName, TeamName,
    };
    use crate::app::teams::use_cases::test_doubles::FakeTeamRepository;

    const TEAM: &str = "00000000000000000000000001";
    const REASON: &str = "Recrutement oublié après la journée 3";

    fn team_id() -> TeamId {
        TeamId::try_new(TEAM).unwrap()
    }

    fn ready_team() -> Vec<TeamDomainEvent> {
        let competition_id = CompetitionId::try_new("00000000000000000000000003").unwrap();
        let season_id = SeasonId::try_new("00000000000000000000000004").unwrap();
        vec![
            TeamDomainEvent::TeamCreated {
                team_id: team_id(),
                space_id: SpaceId::try_new("00000000000000000000000002").unwrap(),
                competition_id: competition_id.clone(),
                competition_name: "Ligue de Condate".into(),
                season_id: season_id.clone(),
                season_name: "Saison 2025".into(),
                name: TeamName::try_new("Les Korrigans FC".to_string()).unwrap(),
                logo_url: None,
                roster_id: RosterId::try_new("00000000000000000000000005").unwrap(),
                roster_name: RosterName::try_new("Nains du Granit".to_string()).unwrap(),
                coach_id: CoachId::try_new("00000000000000000000000006").unwrap(),
                coach_name: "Colonel Castor".into(),
                treasury: Kpo(85),
                dedicated_fans: DedicatedFans::try_new(2).unwrap(),
                rerolls: RerollCount(0),
                apothecaries: ApothecaryCount(0),
                assistants: AssistantCount(0),
                cheerleaders: CheerleaderCount(0),
            },
            TeamDomainEvent::TeamEnrolled {
                competition_id,
                competition_name: "Ligue de Condate".into(),
                season_id,
                season_name: "Saison 2025".into(),
            },
        ]
    }

    fn command() -> OpenPhaseOverrideCommand {
        OpenPhaseOverrideCommand {
            team_id: team_id(),
            phase: OverridablePhase::Recruitment,
            reason: Some(OverrideReason::try_new(REASON.to_string()).unwrap()),
            admin_id: UserId::try_new("00000000000000000000000007").unwrap(),
            admin_name: CoachName::try_new("Bagouze".to_string()).unwrap(),
        }
    }

    #[tokio::test]
    async fn a_missing_team_writes_nothing() {
        let teams = FakeTeamRepository::default();

        let outcome = execute(command(), &teams).await;

        assert!(matches!(outcome, Err(OpenPhaseOverrideError::TeamNotFound)));
        assert!(teams.appended().is_empty());
    }

    /// Une équipe en saisie de rapport n'est pas prête à jouer : le refus du
    /// domaine remonte tel quel, et rien n'est écrit.
    #[tokio::test]
    async fn a_team_not_ready_to_play_is_refused_without_writing() {
        let mut events = ready_team();
        events.push(TeamDomainEvent::MatchReportingStarted {
            match_report_id: MatchReportId::try_new("00000000000000000000000008").unwrap(),
        });
        let teams = FakeTeamRepository::with_events(events);

        let outcome = execute(command(), &teams).await;

        assert!(matches!(
            outcome,
            Err(OpenPhaseOverrideError::Domain(DomainError::WrongGamePhase(
                Some(GamePhase::MatchReporting)
            )))
        ));
        assert!(teams.appended().is_empty(), "un refus n'écrit rien");
    }

    /// Le motif et le nom ne vivent que dans cet événement : perdus ici, ils ne
    /// se retrouveraient nulle part.
    #[tokio::test]
    async fn the_opening_is_appended_with_its_reason_and_author() {
        let teams = FakeTeamRepository::with_events(ready_team());

        assert!(execute(command(), &teams).await.is_ok());

        let appended = teams.appended();
        assert_eq!(appended.len(), 1, "un seul événement");
        let TeamDomainEvent::ManualRecruitmentPhaseOpened {
            admin_name, reason, ..
        } = &appended[0]
        else {
            panic!(
                "attendu ManualRecruitmentPhaseOpened, reçu {:?}",
                appended[0]
            )
        };
        assert_eq!(admin_name.clone().into_inner(), "Bagouze");
        assert_eq!(reason.as_ref().map(|r| r.as_ref()), Some(REASON));
    }
}
