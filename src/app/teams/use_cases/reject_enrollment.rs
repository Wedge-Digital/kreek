use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::RejectEnrollmentCommand;

#[derive(Debug)]
pub enum RejectEnrollmentError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: RejectEnrollmentCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), RejectEnrollmentError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(RejectEnrollmentError::Repository)?
        .ok_or(RejectEnrollmentError::TeamNotFound)?;

    let event = team
        .reject_enrollment()
        .map_err(RejectEnrollmentError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(RejectEnrollmentError::Repository)?;

    Ok(())
}
