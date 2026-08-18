use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::{AcquisitionMode, PlayerId, SkillId, SppCost};
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ports::{
    IReferenceDataPort, ITeamRosterRepository, RepositoryError,
};

#[derive(Debug)]
pub struct SpendCreationSppCommand {
    pub team_id: EntityId,
    pub space_id: String,
    pub instance_id: PlayerId,
    pub skill_id: SkillId,
    pub mode: AcquisitionMode,
}

pub enum SpendSppError {
    TeamNotFound,
    PlayerNotFound,
    SkillCostNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: SpendCreationSppCommand,
    team_repo: &dyn ITeamRosterRepository,
    ref_data: &dyn IReferenceDataPort,
) -> Result<RosterSelectedTeam, SpendSppError> {
    let mut team = team_repo
        .find_by_id(&cmd.team_id)
        .await
        .map_err(SpendSppError::Repository)?
        .ok_or(SpendSppError::TeamNotFound)?;

    let player = team
        .hired_players()
        .iter()
        .find(|p| p.instance_id == cmd.instance_id)
        .ok_or(SpendSppError::PlayerNotFound)?;

    let roster_line_id = player.definition.id.0.clone();

    let mode_str = match cmd.mode {
        AcquisitionMode::Chosen => "chosen",
        AcquisitionMode::Random => "random",
    };

    let cost = ref_data
        .resolve_skill_cost(&roster_line_id, &cmd.skill_id.0, mode_str)
        .ok_or(SpendSppError::SkillCostNotFound)?;

    let spp_cost = SppCost::try_new(cost.spp_cost).map_err(|_| SpendSppError::SkillCostNotFound)?;
    team.spend_spp(&cmd.instance_id, cmd.skill_id, cmd.mode, spp_cost)
        .map_err(SpendSppError::Domain)?;

    team_repo
        .save(&team, &cmd.space_id)
        .await
        .map_err(SpendSppError::Repository)?;

    Ok(team)
}
