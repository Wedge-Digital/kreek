use crate::app::teams::domain::error::DomainError;
use crate::app::teams::ports::{ITeamRepository, RepositoryError};
use crate::app::teams::use_cases::commands::ValidateRecruitmentPhaseCommand;

pub enum ValidateRecruitmentPhaseError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: ValidateRecruitmentPhaseCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), ValidateRecruitmentPhaseError> {
    let team = team_repo
        .find_by_id(&cmd.team_id.to_string())
        .await
        .map_err(ValidateRecruitmentPhaseError::Repository)?
        .ok_or(ValidateRecruitmentPhaseError::TeamNotFound)?;

    let event = team
        .validate_recruitment_phase()
        .map_err(ValidateRecruitmentPhaseError::Domain)?;

    team_repo
        .append(&cmd.team_id.to_string(), &event, team.version)
        .await
        .map_err(ValidateRecruitmentPhaseError::Repository)?;

    Ok(())
}
