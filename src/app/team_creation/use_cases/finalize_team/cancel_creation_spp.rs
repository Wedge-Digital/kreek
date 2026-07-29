use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::{PlayerId, SkillId};
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};

pub struct CancelCreationSppCommand {
    pub team_id: EntityId,
    pub space_id: String,
    pub instance_id: PlayerId,
    pub skill_id: SkillId,
}

pub enum CancelSppError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: CancelCreationSppCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, CancelSppError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(CancelSppError::Repository)?
        .ok_or(CancelSppError::TeamNotFound)?;

    team.cancel_spp(&cmd.instance_id, &cmd.skill_id)
        .map_err(CancelSppError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(CancelSppError::Repository)?;

    Ok(team)
}
