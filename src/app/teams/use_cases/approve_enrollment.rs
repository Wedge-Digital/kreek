use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::ApproveEnrollmentCommand;

pub enum ApproveEnrollmentError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: ApproveEnrollmentCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ApproveEnrollmentError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(ApproveEnrollmentError::Repository)?
        .ok_or(ApproveEnrollmentError::TeamNotFound)?;

    let event = team
        .enroll(
            cmd.competition_id,
            cmd.competition_name,
            cmd.season_id,
            cmd.season_name,
        )
        .map_err(ApproveEnrollmentError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(ApproveEnrollmentError::Repository)?;

    Ok(())
}
