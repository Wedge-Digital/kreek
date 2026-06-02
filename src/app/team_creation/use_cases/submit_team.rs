use crate::app::shared_kernel::common_types::Entity;
use crate::app::shared_kernel::common_types::EventId;
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain_event::TeamCreationDomainEvent;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::SubmitTeamCommand;
use crate::lib::services::event_bus::event_bus::EventBus;

pub enum SubmitTeamError {
    TeamNotFound,
    Domain(Vec<DomainError>),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: SubmitTeamCommand,
    team_repo: &dyn ITeamRosterRepository,
    bus: &EventBus,
) -> Result<(), SubmitTeamError> {
    let team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(SubmitTeamError::Repository)?
        .ok_or(SubmitTeamError::TeamNotFound)?;

    team.validate_for_submission()
        .map_err(SubmitTeamError::Domain)?;

    team_repo
        .mark_submitted(&team.get_id())
        .await
        .map_err(SubmitTeamError::Repository)?;

    let base = team.base_infos();
    let event = TeamCreationDomainEvent::TeamSubmitted {
        event_id: EventId::new(),
        team_id: team.get_id().to_string(),
        space_id: cmd.space_id,
        team_name: base.name().clone().into_inner(),
        roster_id: team.roster.id.0.clone(),
        roster_name: team.roster.name.0.clone(),
        coach_id: base.coach_id().to_string(),
        coach_name: cmd.coach_name,
        treasury: team.remaining_budget().unwrap_or(0),
    };
    let _ = bus.send(event.to_enveloppe());

    Ok(())
}

pub fn domain_error_message(e: &DomainError) -> &'static str {
    match e {
        DomainError::InsufficientPlayerCount => {
            "Vous devez engager au moins 11 joueurs pour soumettre votre équipe."
        }
        _ => "Action impossible.",
    }
}
