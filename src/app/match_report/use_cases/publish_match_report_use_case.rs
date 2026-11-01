use crate::app::match_report::domain::match_report_repository_port::{
    IMatchReportRepository, RepositoryError,
};
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct PublishMatchReportCommand {
    pub match_report_id: MatchReportId,
    pub published_by: CoachId,
}

#[derive(Debug)]
pub enum PublishMatchReportError {
    NotFound,
    AlreadyPublished,
    Cancelled,
    /// Une autre écriture est passée entre le chargement et l'append (carte 623).
    Conflict,
    Repository(String),
}

impl PublishMatchReportError {
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
    cmd: PublishMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<(), PublishMatchReportError> {
    let mr_id_str = cmd.match_report_id.to_string();

    let state = repo
        .find_by_id(&mr_id_str)
        .await
        .map_err(PublishMatchReportError::from_repository)?
        .ok_or(PublishMatchReportError::NotFound)?;

    let rtp = match state {
        MatchReportState::ReadyToPublish(rtp) => rtp,
        MatchReportState::Draft(_) | MatchReportState::PreMatch(_) => {
            return Err(PublishMatchReportError::NotFound);
        }
        MatchReportState::Published(_) => {
            return Err(PublishMatchReportError::AlreadyPublished);
        }
        MatchReportState::Cancelled(_) => {
            return Err(PublishMatchReportError::Cancelled);
        }
    };

    let (published, event) = rtp.publish(cmd.published_by);

    repo.append(&mr_id_str, &event, published.version - 1)
        .await
        .map_err(PublishMatchReportError::from_repository)?;

    emettre(bus, event.to_enveloppe(&mr_id_str));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::match_report::domain::value_objects::{
        FanFactorMod, MatchGain, NoStallingBonuses,
    };
    use crate::app::match_report::use_cases::test_doubles::{pre_match, ConflictingRepository};
    use crate::common::services::event_bus::event_bus::new_bus;

    #[tokio::test]
    async fn un_conflit_d_ecriture_est_relaye() {
        let (rtp, _) = pre_match().record_post_match(
            MatchGain::try_new(10_000).unwrap(),
            MatchGain::try_new(10_000).unwrap(),
            NoStallingBonuses::default(),
            FanFactorMod::try_new(0).unwrap(),
            FanFactorMod::try_new(0).unwrap(),
            None,
            None,
            CoachId::new(),
        );
        let repo = ConflictingRepository::holding(MatchReportState::ReadyToPublish(rtp));
        let cmd = PublishMatchReportCommand {
            match_report_id: MatchReportId::new(),
            published_by: CoachId::new(),
        };
        let resultat = execute(cmd, &repo, &new_bus()).await;
        assert!(
            matches!(resultat, Err(PublishMatchReportError::Conflict)),
            "{resultat:?}"
        );
    }
}
