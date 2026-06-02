use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::PlayerId;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::shared_kernel::common_types::EntityId;

pub struct SetPlayerIdentityCommand {
    pub team_id:   EntityId,
    pub space_id:  String,
    pub instance_id: PlayerId,
    pub name:      String,
    pub jersey:    u8,
}

pub enum SetPlayerIdentityError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: SetPlayerIdentityCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<(), SetPlayerIdentityError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(SetPlayerIdentityError::Repository)?
        .ok_or(SetPlayerIdentityError::TeamNotFound)?;

    team.set_player_identity(&cmd.instance_id, cmd.name, cmd.jersey)
        .map_err(SetPlayerIdentityError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(SetPlayerIdentityError::Repository)?;

    Ok(())
}
