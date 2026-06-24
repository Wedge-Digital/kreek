use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::ports::ITeamDataPort;
use crate::app::shared_kernel::common_types::{CoachId, MatchReportId};
use crate::app::shared_kernel::team::TeamId;

pub struct UpdateMatchSelectionCommand {
    pub match_report_id: MatchReportId,
    pub home_team_id: TeamId,
    pub away_team_id: TeamId,
    pub confirmed_by: CoachId,
}

#[derive(Debug)]
pub enum UpdateMatchSelectionError {
    NotFound,
    NotInDraftPhase,
    SameTeam,
    TeamNotAvailable(String),
    Repository(String),
}

pub async fn execute(
    cmd: UpdateMatchSelectionCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
) -> Result<MatchReportId, UpdateMatchSelectionError> {
    let mr_id_str = cmd.match_report_id.to_string();

    let state = repo
        .find_by_id(&mr_id_str)
        .await
        .map_err(|e| UpdateMatchSelectionError::Repository(e.to_string()))?
        .ok_or(UpdateMatchSelectionError::NotFound)?;

    let mut draft = match state {
        MatchReportState::Draft(d) => d,
        _ => return Err(UpdateMatchSelectionError::NotInDraftPhase),
    };

    let selection_changed =
        draft.home_team_id != cmd.home_team_id || draft.away_team_id != cmd.away_team_id;

    if selection_changed {
        let (updated, event) = draft
            .update_selection(cmd.home_team_id, cmd.away_team_id, cmd.confirmed_by)
            .map_err(|_| UpdateMatchSelectionError::SameTeam)?;

        repo.append(&mr_id_str, &event, updated.version - 1)
            .await
            .map_err(|e| UpdateMatchSelectionError::Repository(e.to_string()))?;

        draft = updated;
    }

    let home_ready = team_data
        .is_team_ready_to_play(&draft.home_team_id.to_string())
        .await
        .map_err(|e| UpdateMatchSelectionError::Repository(e))?;
    if !home_ready {
        return Err(UpdateMatchSelectionError::TeamNotAvailable(
            draft.home_team_id.to_string(),
        ));
    }

    let away_ready = team_data
        .is_team_ready_to_play(&draft.away_team_id.to_string())
        .await
        .map_err(|e| UpdateMatchSelectionError::Repository(e))?;
    if !away_ready {
        return Err(UpdateMatchSelectionError::TeamNotAvailable(
            draft.away_team_id.to_string(),
        ));
    }

    let (_pre_match, confirm_event) = draft.confirm_selection(cmd.confirmed_by);

    repo.append(&mr_id_str, &confirm_event, _pre_match.version - 1)
        .await
        .map_err(|e| UpdateMatchSelectionError::Repository(e.to_string()))?;

    Ok(cmd.match_report_id)
}
