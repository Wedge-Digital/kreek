use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::ChangeTeamLogoCommand;

#[derive(Debug)]
pub enum ChangeTeamLogoError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: ChangeTeamLogoCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ChangeTeamLogoError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(ChangeTeamLogoError::Repository)?
        .ok_or(ChangeTeamLogoError::TeamNotFound)?;

    let event = team
        .change_logo(cmd.logo_url)
        .map_err(ChangeTeamLogoError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(ChangeTeamLogoError::Repository)?;

    Ok(())
}
