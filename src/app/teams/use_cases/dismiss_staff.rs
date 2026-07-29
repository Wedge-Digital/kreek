use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::value_objects::{StaffQuantity, StaffType};
use crate::app::teams::ports::{ITeamRepository, RepositoryError};

pub struct DismissStaffCommand {
    pub team_id: String,
    pub staff_type: StaffType,
    pub quantity: StaffQuantity,
}

pub enum DismissStaffError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: DismissStaffCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), DismissStaffError> {
    let team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(DismissStaffError::Repository)?
        .ok_or(DismissStaffError::TeamNotFound)?;

    let event = team
        .dismiss_staff(cmd.staff_type, cmd.quantity)
        .map_err(DismissStaffError::Domain)?;

    team_repo
        .append(&cmd.team_id, &event, team.version)
        .await
        .map_err(DismissStaffError::Repository)?;

    Ok(())
}
