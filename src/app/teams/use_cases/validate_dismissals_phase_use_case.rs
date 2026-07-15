use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::ValidateDismissalsPhaseCommand;

pub enum ValidateDismissalsPhaseError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: ValidateDismissalsPhaseCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ValidateDismissalsPhaseError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(ValidateDismissalsPhaseError::Repository)?
        .ok_or(ValidateDismissalsPhaseError::TeamNotFound)?;

    let event = team
        .validate_dismissals_phase()
        .map_err(ValidateDismissalsPhaseError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(ValidateDismissalsPhaseError::Repository)?;

    Ok(())
}
