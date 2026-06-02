use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::{PlayerId, SkillId};
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::shared_kernel::common_types::EntityId;

pub struct CancelCreationSppCommand {
    pub team_id:     EntityId,
    pub space_id:    String,
    pub instance_id: PlayerId,
    pub skill_id:    SkillId,
}

pub enum CancelCreationSppError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: CancelCreationSppCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<u8, CancelCreationSppError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(CancelCreationSppError::Repository)?
        .ok_or(CancelCreationSppError::TeamNotFound)?;

    let refunded = team
        .cancel_spp(&cmd.instance_id, &cmd.skill_id)
        .map_err(CancelCreationSppError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(CancelCreationSppError::Repository)?;

    Ok(refunded)
}
