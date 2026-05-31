use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::RemoveRerollCommand;

pub enum RemoveRerollError {
    TeamNotFound,
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:       RemoveRerollCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, RemoveRerollError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(RemoveRerollError::Repository)?
        .ok_or(RemoveRerollError::TeamNotFound)?;

    team.remove_reroll(1);

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(RemoveRerollError::Repository)?;

    Ok(team)
}
