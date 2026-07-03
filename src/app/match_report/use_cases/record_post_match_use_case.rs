use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::{FanFactorMod, MatchGain};
use crate::app::shared_kernel::common_types::{CoachId, MatchReportId};

pub struct RecordPostMatchCommand {
    pub match_report_id: MatchReportId,
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub recorded_by: CoachId,
}

pub enum RecordPostMatchOutcome {
    Success,
}

#[derive(Debug)]
pub enum RecordPostMatchError {
    NotFound,
    NotInCompatibleState,
    Internal(String),
}

pub async fn execute(
    cmd: RecordPostMatchCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<RecordPostMatchOutcome, RecordPostMatchError> {
    let mr_id = cmd.match_report_id.to_string();

    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(|e| RecordPostMatchError::Internal(e.to_string()))?
        .ok_or(RecordPostMatchError::NotFound)?;

    let (updated_version, event) = match state {
        MatchReportState::PreMatch(pm) => {
            let (ready, ev) = pm.record_post_match(
                cmd.home_gain, cmd.away_gain,
                cmd.home_fan_mod, cmd.away_fan_mod,
                cmd.summary_title, cmd.summary_body,
                cmd.recorded_by,
            );
            (ready.version, ev)
        }
        MatchReportState::ReadyToPublish(rtp) => {
            let (updated, ev) = rtp.record_post_match(
                cmd.home_gain, cmd.away_gain,
                cmd.home_fan_mod, cmd.away_fan_mod,
                cmd.summary_title, cmd.summary_body,
                cmd.recorded_by,
            );
            (updated.version, ev)
        }
        MatchReportState::Draft(_)
        | MatchReportState::Cancelled(_)
        | MatchReportState::Published(_) => {
            return Err(RecordPostMatchError::NotInCompatibleState);
        }
    };

    repo.append(&mr_id, &event, updated_version - 1)
        .await
        .map_err(|e| RecordPostMatchError::Internal(e.to_string()))?;

    Ok(RecordPostMatchOutcome::Success)
}
