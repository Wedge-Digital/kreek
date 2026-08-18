use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::FirePlayerCommand;

pub enum FirePlayerError {
    TeamNotFound,
    PlayerNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: FirePlayerCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, FirePlayerError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(FirePlayerError::Repository)?
        .ok_or(FirePlayerError::TeamNotFound)?;

    let player_def = team
        .roster
        .player_definitions
        .iter()
        .find(|p| p.id == cmd.player_id)
        .cloned()
        .ok_or(FirePlayerError::PlayerNotFound)?;

    team.remove_player(&player_def)
        .map_err(FirePlayerError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(FirePlayerError::Repository)?;

    Ok(team)
}

pub fn domain_error_message(e: &DomainError) -> &'static str {
    match e {
        DomainError::PlayerNotHired => "Ce joueur n'est pas dans votre équipe.",
        _ => "Action impossible.",
    }
}
