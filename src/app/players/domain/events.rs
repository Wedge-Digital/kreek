use crate::app::players::domain::player::{AcquisitionMode, PlayerId, Spp, TeamId, ValueKpo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerDomainEvent {
    PlayerCreated {
        player_id:      PlayerId,
        team_id:        TeamId,
        space_id:       String,
        position_name:  String,
        roster_line_id: String,
        personal_name:  String,
        jersey:         Option<u8>,
        base_skills:    Vec<String>,
        starting_spp:   Spp,
        starting_value: ValueKpo,
    },
    InitialSkillEarned {
        player_id:    PlayerId,
        team_id:      TeamId,
        skill_id:     String,
        skill_name:   String,
        category_css: String,
        mode:         AcquisitionMode,
        spp_cost:     u8,
        is_primary:   bool,
        is_elite:     bool,
        value_delta:  ValueKpo,
    },
}
