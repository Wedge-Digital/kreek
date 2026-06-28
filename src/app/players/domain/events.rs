use crate::app::players::domain::player::{AcquisitionMode, PlayerId, Spp, TeamId, ValueKpo};
use crate::app::players::domain::value_objects::{
    JerseyVo, PositionNameVo, RosterLineId, SkillId, SkillName, SppCost,
};
use crate::app::shared_kernel::common_types::SpaceId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerDomainEvent {
    PlayerCreated {
        player_id:      PlayerId,
        team_id:        TeamId,
        space_id:       SpaceId,
        position_name:  PositionNameVo,
        roster_line_id: RosterLineId,
        jersey:         Option<JerseyVo>,
        base_skills:    Vec<SkillId>,
        starting_spp:   Spp,
        starting_value: ValueKpo,
    },
    InitialSkillEarned {
        player_id:    PlayerId,
        team_id:      TeamId,
        skill_id:     SkillId,
        skill_name:   SkillName,
        category_css: String,
        mode:         AcquisitionMode,
        spp_cost:     SppCost,
        is_primary:   bool,
        is_elite:     bool,
        value_delta:  ValueKpo,
    },
}
