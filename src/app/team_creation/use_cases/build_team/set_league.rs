use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::domain::roster::LeagueId;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};

pub struct SetLeagueCommand {
    pub team_id: EntityId,
    pub space_id: String,
    pub league_id: LeagueId,
}

pub enum SetLeagueError {
    TeamNotFound,
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: SetLeagueCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<(), SetLeagueError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(SetLeagueError::Repository)?
        .ok_or(SetLeagueError::TeamNotFound)?;

    team.set_league(cmd.league_id);

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(SetLeagueError::Repository)?;

    Ok(())
}
