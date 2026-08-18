use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::BuyRerollCommand;

pub enum BuyRerollError {
    TeamNotFound,
    Domain(Vec<DomainError>),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: BuyRerollCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, BuyRerollError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(BuyRerollError::Repository)?
        .ok_or(BuyRerollError::TeamNotFound)?;

    team.purchase_reroll(1).map_err(BuyRerollError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(BuyRerollError::Repository)?;

    Ok(team)
}

pub fn domain_error_message(errors: &[DomainError]) -> &'static str {
    for e in errors {
        match e {
            DomainError::MaxRerollsExceeded => {
                return "Vous ne pouvez pas prendre plus de 8 relances."
            }
            DomainError::InsufficientRerollBudget => {
                return "Budget insuffisant pour cette relance."
            }
            _ => {}
        }
    }
    "Action impossible."
}
