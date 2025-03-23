use serde::{Deserialize, Serialize};
use crate::app::global_types::global_type::{Entity};
use crate::app::team_creation::common_types::{BaseTeamInfo, TeamId, UserId};
use crate::services::id_service::IdService;
use crate::User;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DraftTeam{
    entity_id: TeamId,
    created_by: UserId,
    base_infos: BaseTeamInfo,
}

impl DraftTeam {
    pub fn new<T: IdService>(id_service: &T, created_by: UserId, base_team_infos: BaseTeamInfo) -> Self {
        DraftTeam {
            entity_id: id_service.generate_id(),
            created_by: created_by.clone(),
            base_infos: base_team_infos,
        }
    }
}

impl PartialEq<Self> for DraftTeam {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Entity for DraftTeam {
    fn get_id(&self) -> TeamId {
        return self.entity_id.clone();
    }

    fn get_created_by(&self) -> TeamId {
        return self.created_by.clone();
    }
}
