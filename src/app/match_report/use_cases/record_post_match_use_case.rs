use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_pre_match::MatchReportPreMatch;
use crate::app::match_report::domain::match_report_ready_to_publish::MatchReportReadyToPublish;
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
    /// Un refus du domaine : un camp sans MVP (carte 638).
    Domain(DomainError),
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
    // Le refus d'un camp sans MVP vient du domaine (carte 638).
    let (updated_version, event) = record(state, cmd)?;
    repo.append(&mr_id, &event, updated_version - 1)
        .await
        .map_err(RecordPostMatchError::from_repository)?;
    Ok(RecordPostMatchOutcome::Success)
}

/// La clôture de l'après-match, depuis l'avant-match ou en correction.
fn record(
    state: MatchReportState,
    cmd: RecordPostMatchCommand,
) -> Result<(u64, MatchReportDomainEvent), RecordPostMatchError> {
    let recorded = match state {
        MatchReportState::PreMatch(pm) => record_from_pre_match(&pm, cmd),
        MatchReportState::ReadyToPublish(rtp) => record_again(&rtp, cmd),
        MatchReportState::Draft(_)
        | MatchReportState::Cancelled(_)
        | MatchReportState::Published(_) => return Err(RecordPostMatchError::NotInCompatibleState),
    };
    recorded.map_err(RecordPostMatchError::Domain)
}

fn record_from_pre_match(
    pm: &MatchReportPreMatch,
    cmd: RecordPostMatchCommand,
) -> Result<(u64, MatchReportDomainEvent), DomainError> {
    let (ready, ev) = pm.record_post_match(
        cmd.home_gain,
        cmd.away_gain,
        cmd.no_stalling_bonuses,
        cmd.home_fan_mod,
        cmd.away_fan_mod,
        cmd.summary_title,
        cmd.summary_body,
        cmd.recorded_by,
    )?;
    Ok((ready.version, ev))
}

/// En correction : les gains se ressaisissent sur le rapport prêt à publier.
fn record_again(
    rtp: &MatchReportReadyToPublish,
    cmd: RecordPostMatchCommand,
) -> Result<(u64, MatchReportDomainEvent), DomainError> {
    let (updated, ev) = rtp.record_post_match(
        cmd.home_gain,
        cmd.away_gain,
        cmd.no_stalling_bonuses,
        cmd.home_fan_mod,
        cmd.away_fan_mod,
        cmd.summary_title,
        cmd.summary_body,
        cmd.recorded_by,
    )?;
    Ok((updated.version, ev))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::TeamSide;
    use crate::app::match_report::use_cases::test_doubles::{
        pre_match, pre_match_with_mvps, ConflictingRepository, RecordingRepository,
    };

    fn command() -> RecordPostMatchCommand {
        RecordPostMatchCommand {
            match_report_id: MatchReportId::new(),
            home_gain: MatchGain::try_new(10_000).unwrap(),
            away_gain: MatchGain::try_new(10_000).unwrap(),
            no_stalling_bonuses: NoStallingBonuses::default(),
            home_fan_mod: FanFactorMod::try_new(0).unwrap(),
            away_fan_mod: FanFactorMod::try_new(0).unwrap(),
            summary_title: None,
            summary_body: None,
            recorded_by: CoachId::new(),
        }
    }

    #[tokio::test]
    async fn un_conflit_d_ecriture_est_relaye() {
        let repo =
            ConflictingRepository::holding(MatchReportState::PreMatch(pre_match_with_mvps()));
        let resultat = execute(command(), &repo).await;
        assert!(matches!(resultat, Err(RecordPostMatchError::Conflict)));
    }

    /// Carte 638 : sans MVP, l'après-match ne se clôt pas, et rien n'est écrit.
    #[tokio::test]
    async fn un_camp_sans_mvp_est_refuse_sans_ecriture() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pre_match()));
        let resultat = execute(command(), &repo).await;
        assert!(
            matches!(
                resultat,
                Err(RecordPostMatchError::Domain(DomainError::MvpMissing(
                    TeamSide::Home
                )))
            ),
            "{:?}",
            resultat.err()
        );
        assert!(repo.appended().is_empty());
    }

    #[tokio::test]
    async fn avec_ses_deux_mvp_l_apres_match_se_clot() {
        let repo = RecordingRepository::holding(MatchReportState::PreMatch(pre_match_with_mvps()));
        assert!(execute(command(), &repo).await.is_ok());
        assert_eq!(repo.appended().len(), 1);
    }
}
