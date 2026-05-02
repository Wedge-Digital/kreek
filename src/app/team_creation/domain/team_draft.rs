use serde::{Deserialize, Serialize};
use crate::app::shared_kernel::common_types::{Entity, UserId};
use crate::app::shared_kernel::id_service::IdService;
use crate::app::shared_kernel::team::{BaseTeamInfo, TeamId};
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::ruleset::Ruleset;
use crate::app::team_creation::domain::team_ruleset_selected::RulesetSelectedTeam;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DraftTeam {
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

    pub fn base_infos(&self) -> &BaseTeamInfo {
        &self.base_infos
    }

    pub fn select_ruleset(self, ruleset: Ruleset) -> Result<RulesetSelectedTeam, DomainError> {
        Ok(RulesetSelectedTeam::new(self, ruleset))
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
