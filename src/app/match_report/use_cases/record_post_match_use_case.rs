use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::NoStallingBonuses;
use crate::app::match_report::domain::value_objects::{FanFactorMod, MatchGain};
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;

#[derive(Debug)]
pub struct RecordPostMatchCommand {
    pub match_report_id: MatchReportId,
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    /// Les bonus pour non temporisation des deux équipes (carte 481).
    pub no_stalling_bonuses: NoStallingBonuses,
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
    /// Une autre écriture est passée entre le chargement et l'append (carte 623).
    Conflict,
    Internal(String),
}

impl RecordPostMatchError {
    /// `ConcurrentWrite` garde son sens au lieu de se fondre dans `Internal`.
    fn from_repository(e: RepositoryError) -> Self {
        match e {
            RepositoryError::ConcurrentWrite => Self::Conflict,
            other => Self::Internal(other.to_string()),
        }
    }
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RecordPostMatchCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<RecordPostMatchOutcome, RecordPostMatchError> {
    let mr_id = cmd.match_report_id.to_string();

    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(RecordPostMatchError::from_repository)?
        .ok_or(RecordPostMatchError::NotFound)?;

    let (updated_version, event) = match state {
        MatchReportState::PreMatch(pm) => {
            let (ready, ev) = pm.record_post_match(
                cmd.home_gain,
                cmd.away_gain,
                cmd.no_stalling_bonuses,
                cmd.home_fan_mod,
                cmd.away_fan_mod,
                cmd.summary_title,
                cmd.summary_body,
                cmd.recorded_by,
            );
            (ready.version, ev)
        }
        MatchReportState::ReadyToPublish(rtp) => {
            let (updated, ev) = rtp.record_post_match(
                cmd.home_gain,
                cmd.away_gain,
                cmd.no_stalling_bonuses,
                cmd.home_fan_mod,
                cmd.away_fan_mod,
                cmd.summary_title,
                cmd.summary_body,
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
        .map_err(RecordPostMatchError::from_repository)?;

    Ok(RecordPostMatchOutcome::Success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::use_cases::test_doubles::{pre_match, ConflictingRepository};

    #[tokio::test]
    async fn un_conflit_d_ecriture_est_relaye() {
        let repo = ConflictingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let cmd = RecordPostMatchCommand {
            match_report_id: MatchReportId::new(),
            home_gain: MatchGain::try_new(10_000).unwrap(),
            away_gain: MatchGain::try_new(10_000).unwrap(),
            no_stalling_bonuses: NoStallingBonuses::default(),
            home_fan_mod: FanFactorMod::try_new(0).unwrap(),
            away_fan_mod: FanFactorMod::try_new(0).unwrap(),
            summary_title: None,
            summary_body: None,
            recorded_by: CoachId::new(),
        };
        let resultat = execute(cmd, &repo).await;
        assert!(matches!(resultat, Err(RecordPostMatchError::Conflict)));
    }
}
