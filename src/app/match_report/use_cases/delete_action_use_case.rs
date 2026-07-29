use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::ActionId;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;

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
    Repository(String),
}

pub async fn execute(
    cmd: DeleteActionCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<(), DeleteActionError> {
    let mr_id = cmd.match_report_id.to_string();
    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(|e| DeleteActionError::Repository(e.to_string()))?;
    let pm = match state.ok_or(DeleteActionError::NotFound)? {
        MatchReportState::PreMatch(pm) => pm,
        MatchReportState::ReadyToPublish(rtp) => rtp.into_pre_match(),
        _ => return Err(DeleteActionError::NotInPreMatchPhase),
    };

    let (_, event) = pm
        .delete_action(&cmd.action_id, cmd.deleted_by)
        .map_err(DeleteActionError::Domain)?;
    repo.append(&mr_id, &event, pm.version)
        .await
        .map(|_| ())
        .map_err(|e| DeleteActionError::Repository(e.to_string()))
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
            star_engagements: vec![],
            home_temp_players: vec![],
            away_temp_players: vec![],
            home_actions: vec![],
            away_actions: vec![],
            version: 1,
        };
        let action_id = ActionId("ACT01".into());
        let (updated, _) = pm.record_action(
            TeamSide::Home,
            TurnNumber::try_new(1).unwrap(),
            ActionPlayer::Regular(crate::app::shared_kernel::bloodbowl::ids::PlayerId::new()),
            MatchActionType::Touchdown,
            "Player Name".into(),
            String::new(),
            action_id.clone(),
            CoachId::new(),
        );
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
