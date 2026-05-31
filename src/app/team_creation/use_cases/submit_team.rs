use crate::app::shared_kernel::common_types::Entity;
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::SubmitTeamCommand;

pub enum SubmitTeamError {
    TeamNotFound,
    Domain(Vec<DomainError>),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:       SubmitTeamCommand,
    team_repo: &dyn ITeamRosterRepository,
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

    Ok(())
}

pub fn domain_error_message(e: &DomainError) -> &'static str {
    match e {
        DomainError::InsufficientPlayerCount =>
            "Vous devez engager au moins 11 joueurs pour soumettre votre équipe.",
        _ => "Action impossible.",
    }
}
