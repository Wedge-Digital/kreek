use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::ActionId;
use crate::app::match_report::use_cases::conflict_retry::{lost_the_race, settle, ConflictAware};
use crate::app::match_report::use_cases::event_emission::emit_all;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct DeleteActionCommand {
    pub match_report_id: MatchReportId,
    pub action_id: ActionId,
    pub deleted_by: CoachId,
}

#[derive(Debug)]
pub enum DeleteActionError {
    NotFound,
    NotInPreMatchPhase,
    Domain(DomainError),
    /// Une autre écriture est passée entre le chargement et l'append (carte 623).
    Conflict,
    Repository(String),
}

impl DeleteActionError {
    /// `ConcurrentWrite` garde son sens au lieu de se fondre dans `Repository`.
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Repository(other.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: DeleteActionCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<(), DeleteActionError> {
    // Carte 653 : une course perdue contre l'autre camp se rejoue une fois. Une
    // action que l'autre a déjà retirée sort alors en `ActionNotFound`.
    let first = delete_once(&cmd, repo, bus).await;
    if !lost_the_race(COMMAND, &first) {
        return first;
    }
    settle(COMMAND, delete_once(&cmd, repo, bus).await)
}

const COMMAND: &str = "delete_action";

impl ConflictAware for DeleteActionError {
    fn is_conflict(&self) -> bool {
        matches!(self, Self::Conflict)
    }
}

/// Un essai : relire l'agrégat, le laisser décider, appender à la version lue.
async fn delete_once(
    cmd: &DeleteActionCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<(), DeleteActionError> {
    let mr_id = cmd.match_report_id.to_string();
    let pm = load_pre_match(repo, &mr_id).await?;
    let (_, event) = pm
        .delete_action(&cmd.action_id, cmd.deleted_by.clone())
        .map_err(DeleteActionError::Domain)?;
    repo.append(&mr_id, &event, pm.version)
        .await
        .map_err(DeleteActionError::from_repository)?;
    // Le retrait sort du BC une fois écrit (carte 643) : un conflit n'émet rien.
    emit_all(bus, &mr_id, std::slice::from_ref(&event));
    Ok(())
}

async fn load_pre_match(
    repo: &dyn IMatchReportRepository,
    mr_id: &str,
) -> Result<MatchReportPreMatch, DeleteActionError> {
    let state = repo
        .find_by_id(mr_id)
        .await
        .map_err(DeleteActionError::from_repository)?;
    match state.ok_or(DeleteActionError::NotFound)? {
        MatchReportState::PreMatch(pm) => Ok(pm),
        MatchReportState::ReadyToPublish(rtp) => Ok(rtp.into_pre_match()),
        _ => Err(DeleteActionError::NotInPreMatchPhase),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::value_objects::{
        ActionPlayer, DedicatedFans, MatchActionType, MatchReportOrigin, TeamSide, TeamValue,
        TurnNumber,
    };
    use crate::app::shared_kernel::bloodbowl::ids::{
        CompetitionId, MatchReportId, RoundId, SeasonId,
    };
    use crate::app::shared_kernel::bloodbowl::team::TeamId;
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};

    fn make_pm_with_action() -> (MatchReportPreMatch, ActionId) {
        let home_id = TeamId::new();
        let away_id = TeamId::new();
        let pm = MatchReportPreMatch {
            id: MatchReportId::new(),
            space_id: SpaceId::new(),
            competition_id: CompetitionId::new(),
            season_id: SeasonId::new(),
            round_id: RoundId::new(),
            home_team_id: home_id.clone(),
            away_team_id: away_id,
            created_by: CoachId::new(),
            entry_mode: crate::app::match_report::domain::value_objects::EntryMode::SoloEntry,
            started_by: CoachId::new(),
            origin: MatchReportOrigin::Manual,
            pairing_id: None,
            home_fan_roll: None,
            away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(),
            away_dedicated_fans: DedicatedFans::default(),
            home_team_value: Some(TeamValue::try_new(1000).unwrap()),
            away_team_value: Some(TeamValue::try_new(1000).unwrap()),
            home_inducements: None,
            away_inducements: None,
            home_inducements_recorded_by: None,
            away_inducements_recorded_by: None,
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            drawn_mvps: vec![],
            version: 1,
        };
        let action_id = ActionId("ACT01".into());
        let (updated, _) = pm
            .record_action(
                TeamSide::Home,
                TurnNumber::try_new(1).unwrap(),
                ActionPlayer::Regular(crate::app::shared_kernel::bloodbowl::ids::PlayerId::new()),
                MatchActionType::Touchdown,
                "Player Name".into(),
                String::new(),
                action_id.clone(),
                CoachId::new(),
            )
            .expect("action de test valide");
        (updated, action_id)
    }

    #[test]
    fn delete_action_domain_ok_for_existing_action() {
        let (pm, action_id) = make_pm_with_action();
        let result = pm.delete_action(&action_id, CoachId::new());
        assert!(result.is_ok());
    }

    #[test]
    fn delete_action_domain_err_for_unknown_action() {
        let (pm, _) = make_pm_with_action();
        let result = pm.delete_action(&ActionId("UNKNOWN".into()), CoachId::new());
        assert!(matches!(result, Err(DomainError::ActionNotFound(_))));
    }
}

#[cfg(test)]
mod conflict_tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{
        ActionPlayer, MatchActionType, TeamSide, TurnNumber,
    };
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, RecordingRepository, ScriptedRepository,
    };
    use crate::app::shared_kernel::bloodbowl::ids::PlayerId;
    use crate::common::services::event_bus::event_bus::new_bus;

    fn pre_match_with_action(action_id: &ActionId) -> MatchReportState {
        let (pm, _) = pre_match()
            .record_action(
                TeamSide::Home,
                TurnNumber::try_new(1).unwrap(),
                ActionPlayer::Regular(PlayerId::new()),
                MatchActionType::Touchdown,
                "Joueur".into(),
                String::new(),
                action_id.clone(),
                CoachId::new(),
            )
            .expect("action de test valide");
        MatchReportState::PreMatch(pm)
    }

    fn command(action_id: ActionId) -> DeleteActionCommand {
        DeleteActionCommand {
            match_report_id: MatchReportId::new(),
            action_id,
            deleted_by: CoachId::new(),
        }
    }

    /// Carte 653 : deux conflits de suite sont relayés, après deux appends.
    #[tokio::test]
    async fn two_conflicts_are_relayed() {
        let action_id = ActionId("ACT01".into());
        let reads = vec![
            pre_match_with_action(&action_id),
            pre_match_with_action(&action_id),
        ];
        let repo = ScriptedRepository::reading(reads).refusing(2);
        let bus = new_bus();
        let mut subscriber = bus.subscribe();
        let resultat = execute(command(action_id), &repo, &bus).await;
        assert!(
            matches!(resultat, Err(DeleteActionError::Conflict)),
            "{resultat:?}"
        );
        assert_eq!(repo.append_attempts(), 2);
        assert!(subscriber.try_recv().is_err(), "un conflit n'émet rien");
    }

    /// Carte 653 : la course perdue se rejoue, le retrait est écrit.
    #[tokio::test]
    async fn a_lost_race_is_retried_and_the_action_is_deleted() {
        let action_id = ActionId("ACT01".into());
        let reads = vec![
            pre_match_with_action(&action_id),
            pre_match_with_action(&action_id),
        ];
        let repo = ScriptedRepository::reading(reads).refusing(1);
        execute(command(action_id), &repo, &new_bus())
            .await
            .expect("le réessai passe");
        assert_eq!(repo.appended().len(), 1);
    }

    /// Carte 653 : l'autre écriture a déjà retiré l'action. Relu, le domaine
    /// le dit — `ActionNotFound`, pas un 409.
    #[tokio::test]
    async fn an_action_already_deleted_on_reread_is_refused_by_the_domain() {
        let action_id = ActionId("ACT01".into());
        let reads = vec![
            pre_match_with_action(&action_id),
            MatchReportState::PreMatch(pre_match()),
        ];
        let repo = ScriptedRepository::reading(reads).refusing(1);
        let resultat = execute(command(action_id), &repo, &new_bus()).await;
        assert!(
            matches!(
                resultat,
                Err(DeleteActionError::Domain(DomainError::ActionNotFound(_)))
            ),
            "{resultat:?}"
        );
        assert!(repo.appended().is_empty());
    }

    /// Carte 643 : le retrait sort du BC une fois appendé.
    #[tokio::test]
    async fn a_deleted_action_is_emitted_after_append() {
        let action_id = ActionId("ACT01".into());
        let repo = RecordingRepository::holding(pre_match_with_action(&action_id));
        let bus = new_bus();
        let mut subscriber = bus.subscribe();

        execute(command(action_id), &repo, &bus)
            .await
            .expect("action retirée");

        assert_eq!(repo.appended().len(), 1);
        assert_eq!(subscriber.try_recv().unwrap().event_type, "ActionDeleted");
        assert!(subscriber.try_recv().is_err());
    }
}
