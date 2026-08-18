use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::DismissTeamCommand;

pub enum DismissTeamError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: DismissTeamCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), DismissTeamError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(DismissTeamError::Repository)?
        .ok_or(DismissTeamError::TeamNotFound)?;

    let event = team.dismiss().map_err(DismissTeamError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(DismissTeamError::Repository)?;

    Ok(())
}
