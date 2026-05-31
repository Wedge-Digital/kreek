use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::BuyStaffCommand;

pub enum BuyStaffError {
    TeamNotFound,
    StaffNotFoundInRoster,
    Domain(Vec<DomainError>),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:       BuyStaffCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, BuyStaffError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(BuyStaffError::Repository)?
        .ok_or(BuyStaffError::TeamNotFound)?;

    let staff_def = team.roster.allowed_staff
        .iter()
        .find(|s| s.id == cmd.staff_id)
        .cloned()
        .ok_or(BuyStaffError::StaffNotFoundInRoster)?;

    team.buy_staff(staff_def)
        .map_err(BuyStaffError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(BuyStaffError::Repository)?;

    Ok(team)
}

pub fn domain_error_message(errors: &[DomainError]) -> &'static str {
    for e in errors {
        match e {
            DomainError::StaffMaxReached      => return "Vous avez atteint le nombre maximum pour ce poste.",
            DomainError::InsufficientBudget   => return "Budget insuffisant pour ce poste.",
            DomainError::StaffNotAllowed      => return "Ce poste n'est pas disponible pour ce roster.",
            _                                 => {}
        }
    }
    "Action impossible."
}