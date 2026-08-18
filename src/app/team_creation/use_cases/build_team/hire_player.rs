use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::HirePlayerCommand;

pub enum HirePlayerError {
    TeamNotFound,
    PlayerNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: HirePlayerCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, HirePlayerError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(HirePlayerError::Repository)?
        .ok_or(HirePlayerError::TeamNotFound)?;

    let player_def = team
        .roster
        .player_definitions
        .iter()
        .find(|p| p.id == cmd.player_id)
        .cloned()
        .ok_or(HirePlayerError::PlayerNotFound)?;

    team.hire_player(player_def)
        .map_err(HirePlayerError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(HirePlayerError::Repository)?;

    Ok(team)
}

pub fn domain_error_message(e: &DomainError) -> &'static str {
    match e {
        DomainError::MaxPlayersReached => "Vous ne pouvez pas engager plus de 16 joueurs.",
        DomainError::MaxPlayersOfTypeReached => "Quota maximum de ce poste atteint.",
        DomainError::InsufficientBudget => "Budget insuffisant pour ce joueur.",
        DomainError::CrossLimitExceeded => "Limite combinée de postes dépassée.",
        DomainError::PlayerNotAllowedInRoster => "Ce poste n'est pas disponible dans ce roster.",
        _ => "Action impossible.",
    }
}
