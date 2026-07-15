use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::ValidateImprovementPhaseCommand;

pub enum ValidateImprovementPhaseError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: ValidateImprovementPhaseCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ValidateImprovementPhaseError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(ValidateImprovementPhaseError::Repository)?
        .ok_or(ValidateImprovementPhaseError::TeamNotFound)?;

    let event = team
        .validate_improvement_phase()
        .map_err(ValidateImprovementPhaseError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(ValidateImprovementPhaseError::Repository)?;

    Ok(())
}
