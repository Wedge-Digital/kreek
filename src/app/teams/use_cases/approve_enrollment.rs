use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::shared_kernel::identity::ids::EntityId;

#[derive(Debug)]
pub enum ApproveEnrollmentError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    team_id: &EntityId,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ApproveEnrollmentError> {
    let team = team_repo
        .find_by_id(&team_id.to_string())
        .await
        .map_err(ApproveEnrollmentError::Repository)?
        .ok_or(ApproveEnrollmentError::TeamNotFound)?;

    let competition_id = team.competition_id.clone().ok_or(ApproveEnrollmentError::TeamNotFound)?;
    let competition_name = team.competition_name.clone().unwrap_or_default();
    let season_id = team.season_id.clone().ok_or(ApproveEnrollmentError::TeamNotFound)?;
    let season_name = team.season_name.clone().unwrap_or_default();

    let event = team
        .enroll(competition_id, competition_name, season_id, season_name)
        .map_err(ApproveEnrollmentError::Domain)?;

    team_repo
        .append(&team_id.to_string(), &event, team.version)
        .await
        .map_err(ApproveEnrollmentError::Repository)?;

    Ok(())
}
