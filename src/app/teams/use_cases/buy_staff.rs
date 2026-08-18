use crate::app::teams::domain::error::DomainError;
use crate::app::teams::domain::value_objects::{Kpo, StaffQuantity, StaffType};
use crate::app::teams::ports::{ITeamRepository, RepositoryError};

#[derive(Debug)]
pub struct BuyStaffCommand {
    pub team_id: String,
    pub staff_type: StaffType,
    pub quantity: StaffQuantity,
    pub cost_kpo: Kpo,
}

pub enum BuyStaffError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: BuyStaffCommand,
    team_repo: &dyn ITeamRepository,
) -> Result<(), BuyStaffError> {
    let team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(BuyStaffError::Repository)?
        .ok_or(BuyStaffError::TeamNotFound)?;

    let event = team
        .buy_staff(cmd.staff_type, cmd.quantity, cmd.cost_kpo)
        .map_err(BuyStaffError::Domain)?;

    team_repo
        .append(&cmd.team_id, &event, team.version)
        .await
        .map_err(BuyStaffError::Repository)?;

    Ok(())
}
