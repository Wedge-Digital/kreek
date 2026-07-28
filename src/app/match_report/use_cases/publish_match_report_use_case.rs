use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::shared_kernel::identity::ids::CoachId;
use crate::app::shared_kernel::bloodbowl::ids::MatchReportId;
use crate::common::services::event_bus::event_bus::EventBus;

pub struct PublishMatchReportCommand {
    pub match_report_id: MatchReportId,
    pub published_by: CoachId,
}

#[derive(Debug)]
pub enum PublishMatchReportError {
    NotFound,
    AlreadyPublished,
    Cancelled,
    Repository(String),
}

pub async fn execute(
    cmd: PublishMatchReportCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<(), PublishMatchReportError> {
    let mr_id_str = cmd.match_report_id.to_string();

    let state = repo
        .find_by_id(&mr_id_str)
        .await
        .map_err(|e| PublishMatchReportError::Repository(e.to_string()))?
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
        .map_err(|e| PublishMatchReportError::Repository(e.to_string()))?;

    let _ = bus.send(event.to_enveloppe(&mr_id_str));

    Ok(())
}
