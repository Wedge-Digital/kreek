use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::D3Roll;
use crate::app::shared_kernel::common_types::{CoachId, MatchReportId};

pub struct RecordFanFactorCommand {
    pub match_report_id: MatchReportId,
    pub home_fan_roll: D3Roll,
    pub away_fan_roll: D3Roll,
    pub recorded_by: CoachId,
}

#[derive(Debug)]
pub enum RecordFanFactorError {
    NotFound,
    NotInPreMatchPhase,
    Repository(String),
}

pub async fn execute(
    cmd: RecordFanFactorCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<MatchReportId, RecordFanFactorError> {
    let mr_id_str = cmd.match_report_id.to_string();

    let state = repo
        .find_by_id(&mr_id_str)
        .await
        .map_err(|e| RecordFanFactorError::Repository(e.to_string()))?
        .ok_or(RecordFanFactorError::NotFound)?;

    let pre_match = match state {
        MatchReportState::PreMatch(pm) => pm,
        _ => return Err(RecordFanFactorError::NotInPreMatchPhase),
    };

    let (updated, event) =
        pre_match.record_fan_factor(cmd.home_fan_roll, cmd.away_fan_roll, cmd.recorded_by);

    repo.append(&mr_id_str, &event, updated.version - 1)
        .await
        .map_err(|e| RecordFanFactorError::Repository(e.to_string()))?;

    Ok(cmd.match_report_id)
}
