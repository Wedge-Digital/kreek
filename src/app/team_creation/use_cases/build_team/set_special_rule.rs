use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::domain::roster::SpecialRuleId;
use crate::app::team_creation::ports::{ITeamRosterRepository, RepositoryError};

pub struct SetSpecialRuleCommand {
    pub team_id: EntityId,
    pub space_id: String,
    pub special_rule_id: SpecialRuleId,
}

pub enum SetSpecialRuleError {
    TeamNotFound,
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: SetSpecialRuleCommand,
    team_repo: &dyn ITeamRosterRepository,
) -> Result<(), SetSpecialRuleError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(SetSpecialRuleError::Repository)?
        .ok_or(SetSpecialRuleError::TeamNotFound)?;

    team.set_special_rule(cmd.special_rule_id);

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(SetSpecialRuleError::Repository)?;

    Ok(())
}
