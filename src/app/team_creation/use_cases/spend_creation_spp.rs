use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::{AcquisitionMode, PlayerId, SkillId};
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::shared_kernel::common_types::EntityId;

pub struct SpendCreationSppCommand {
    pub team_id:     EntityId,
    pub space_id:    String,
    pub instance_id: PlayerId,
    pub skill_id:    SkillId,
    pub mode:        AcquisitionMode,
    pub spp_cost:    u8,
}

pub enum SpendCreationSppError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: SpendCreationSppCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<(), SpendCreationSppError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(SpendCreationSppError::Repository)?
        .ok_or(SpendCreationSppError::TeamNotFound)?;

    team.spend_spp(&cmd.instance_id, cmd.skill_id, cmd.mode, cmd.spp_cost)
        .map_err(SpendCreationSppError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(SpendCreationSppError::Repository)?;

    Ok(())
}
