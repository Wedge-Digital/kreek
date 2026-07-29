use crate::app::players::domain::match_impact::StatKind;
use crate::app::players::domain::player::{AcquisitionMode, PlayerId};
use crate::app::players::domain::value_objects::SkillId;

pub struct PurchaseSkillCommand {
    pub player_id: PlayerId,
    pub skill_id: SkillId,
    pub mode: AcquisitionMode,
}

pub struct IncreaseStatCommand {
    pub player_id: PlayerId,
    pub stat: StatKind,
}
