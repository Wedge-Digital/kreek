use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};
use crate::app::team_creation::use_cases::commands::RemoveStaffCommand;

pub enum RemoveStaffError {
    TeamNotFound,
    StaffNotFoundInRoster,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: RemoveStaffCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<RosterSelectedTeam, RemoveStaffError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(RemoveStaffError::Repository)?
        .ok_or(RemoveStaffError::TeamNotFound)?;

    let staff_def = team
        .roster
        .allowed_staff
        .iter()
        .find(|s| s.id == cmd.staff_id)
        .cloned()
        .ok_or(RemoveStaffError::StaffNotFoundInRoster)?;

    team.remove_staff(&staff_def)
        .map_err(RemoveStaffError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(RemoveStaffError::Repository)?;

    Ok(team)
}

pub fn domain_error_message(e: &DomainError) -> &'static str {
    match e {
        DomainError::StaffNotPurchased => "Ce poste n'a pas été acheté.",
        _ => "Action impossible.",
    }
}
