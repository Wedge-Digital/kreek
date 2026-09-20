//! Rattacher un rapport à une autre journée (carte 557).
//!
//! Appelé par `pairing_moved_listener` : le fait vient de `competitions`, qui
//! a déjà vérifié la règle de la journée. Ce use case ne la revérifie pas — il
//! ne connaît pas le calendrier — et se borne à ce que le domaine du rapport
//! sait dire : un rapport annulé n'a plus de journée à changer, et une journée
//! déjà en place ne s'écrit pas deux fois.

use crate::app::match_report::domain::error::DomainError;
use crate::app::match_report::domain::match_report_repository_port::IMatchReportRepository;
use crate::app::shared_kernel::bloodbowl::ids::{MatchReportId, RoundId};
use crate::common::services::event_bus::domain_event_publication::emettre;
use crate::common::services::event_bus::event_bus::EventBus;

#[derive(Debug)]
pub struct ReassignRoundCommand {
    pub match_report_id: MatchReportId,
    pub round_id: RoundId,
}

#[derive(Debug)]
pub enum ReassignRoundError {
    NotFound,
    Domain(DomainError),
    Repository(String),
}

/// `Ok(false)` quand la journée était déjà celle-là : rien n'a été écrit, et
/// l'appelant n'a pas à s'en inquiéter.
#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ReassignRoundCommand,
    repo: &dyn IMatchReportRepository,
    bus: &EventBus,
) -> Result<bool, ReassignRoundError> {
    let mr_id = cmd.match_report_id.to_string();
    let state = repo
        .find_by_id(&mr_id)
        .await
        .map_err(|e| ReassignRoundError::Repository(e.to_string()))?
        .ok_or(ReassignRoundError::NotFound)?;

    let Some((version, event)) = state
        .reassign_round(cmd.round_id)
        .map_err(ReassignRoundError::Domain)?
    else {
        return Ok(false);
    };

    repo.append(&mr_id, &event, version)
        .await
        .map_err(|e| ReassignRoundError::Repository(e.to_string()))?;
    emettre(bus, event.to_enveloppe(&mr_id));
    Ok(true)
}
