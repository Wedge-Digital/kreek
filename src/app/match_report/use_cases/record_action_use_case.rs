use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{ActionId, ActionPlayer, MatchActionType, TeamSide, TempPlayerId, TurnNumber};
use crate::app::match_report::ports::IPlayerDataPort;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;

pub struct RecordActionCommand {
    pub match_report_id: MatchReportId,
    pub team_side:       TeamSide,
    pub turn:            TurnNumber,
    pub player:          ActionPlayer,
    pub action:          MatchActionType,
    pub recorded_by:     CoachId,
}

pub struct RecordActionOutcome {
    pub action_id: String,
}

#[derive(Debug)]
pub enum RecordActionError {
    NotFound,
    NotInPreMatchPhase,
    PlayerNotFound(String),
    TempPlayerNotFound(String),
    Domain(DomainError),
    Repository(String),
}

pub async fn execute(
    cmd: RecordActionCommand,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
) -> Result<RecordActionOutcome, RecordActionError> {
    let mr_id = cmd.match_report_id.to_string();
    let state = repo.find_by_id(&mr_id).await.map_err(|e| RecordActionError::Repository(e.to_string()))?;
    let pm = match state.ok_or(RecordActionError::NotFound)? {
        MatchReportState::PreMatch(pm) => pm,
        MatchReportState::ReadyToPublish(rtp) => rtp.into_pre_match(),
        _ => return Err(RecordActionError::NotInPreMatchPhase),
    };

    let (display_name, position) = resolve_player_info(&cmd.player, cmd.team_side, &pm, player_data).await?;
    let action_id = ActionId(ulid::Ulid::new().to_string());
    let (_, event) = pm.record_action(cmd.team_side, cmd.turn, cmd.player, cmd.action, display_name, position, action_id.clone(), cmd.recorded_by);

    let outcome = RecordActionOutcome { action_id: action_id.0.clone() };
    repo.append(&mr_id, &event, pm.version)
        .await
        .map_err(|e| RecordActionError::Repository(e.to_string()))?;
    Ok(outcome)
}

async fn resolve_player_info(
    player: &ActionPlayer,
    side: TeamSide,
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
    player_data: &dyn IPlayerDataPort,
) -> Result<(String, String), RecordActionError> {
    match player {
        ActionPlayer::Regular(pid) => {
            let id = pid.to_string();
            let display = player_data.find_player_display(&id).await
                .ok_or_else(|| RecordActionError::PlayerNotFound(id.clone()))?;
            let position = player_data.find_player_position(&id).await.unwrap_or_default();
            Ok((display, position))
        }
        ActionPlayer::Temp(tid) => {
            let display = resolve_temp_display(tid, side, pm)?;
            Ok((display, String::new()))
        }
    }
}

fn resolve_temp_display(
    tid: &TempPlayerId,
    side: TeamSide,
    pm: &crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch,
) -> Result<String, RecordActionError> {
    pm.temp_players_for(side)
        .iter()
        .find(|tp| &tp.id == tid)
        .map(|tp| tp.display_name.clone().unwrap_or_else(|| format!("{:?}", tp.kind)))
        .ok_or_else(|| RecordActionError::TempPlayerNotFound(tid.0.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
    use crate::app::match_report::domain::value_objects::{DedicatedFans, MatchReportOrigin, TeamValue, TempPlayer, TempPlayerKind};
    use crate::app::shared_kernel::identity::ids::{CoachId, SpaceId};
    use crate::app::shared_kernel::bloodbowl::ids::{CompetitionId, MatchReportId, RoundId, SeasonId};
    use crate::app::shared_kernel::bloodbowl::team::TeamId;

    fn make_pm() -> MatchReportPreMatch {
        MatchReportPreMatch {
            id: MatchReportId::new(), space_id: SpaceId::new(),
            competition_id: CompetitionId::new(), season_id: SeasonId::new(),
            round_id: RoundId::new(), home_team_id: TeamId::new(), away_team_id: TeamId::new(),
            created_by: CoachId::new(), origin: MatchReportOrigin::Manual, pairing_id: None,
            home_fan_roll: None, away_fan_roll: None,
            home_dedicated_fans: DedicatedFans::default(), away_dedicated_fans: DedicatedFans::default(),
            home_team_value: Some(TeamValue::try_new(1000).unwrap()),
            away_team_value: Some(TeamValue::try_new(1000).unwrap()),
            home_inducements: None, away_inducements: None,
            star_engagements: vec![],
            home_temp_players: vec![], away_temp_players: vec![],
            home_actions: vec![], away_actions: vec![],
            version: 1,
        }
    }

    #[test]
    fn resolve_temp_display_finds_known_temp_player() {
        let mut pm = make_pm();
        let tid = TempPlayerId("TP01".into());
        pm.home_temp_players.push(TempPlayer {
            id: tid.clone(),
            team_id: pm.home_team_id.clone(),
            kind: TempPlayerKind::StarPlayer { ref_uid: "MORG".into(), position_uid: String::new() },
            display_name: Some("Morg 'N' Thorg".into()),
        });
        let result = resolve_temp_display(&tid, TeamSide::Home, &pm);
        assert_eq!(result.unwrap(), "Morg 'N' Thorg");
    }

    #[test]
    fn resolve_temp_display_returns_error_for_unknown_id() {
        let pm = make_pm();
        let tid = TempPlayerId("UNKNOWN".into());
        let result = resolve_temp_display(&tid, TeamSide::Home, &pm);
        assert!(matches!(result, Err(RecordActionError::TempPlayerNotFound(_))));
    }
}
