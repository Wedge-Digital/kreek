use crate::app::shared_kernel::common_types::Entity;
use crate::app::shared_kernel::team::TeamId;
use crate::app::team_creation::domain::error::DomainError;
use crate::app::team_creation::domain::roster::Roster;
use crate::app::team_creation::domain::ruleset::Ruleset;
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RulesetSelectedTeam {
    team: DraftTeam,
    pub ruleset: Ruleset,
}

impl RulesetSelectedTeam {
    pub fn new(team: DraftTeam, ruleset: Ruleset) -> Self {
        RulesetSelectedTeam { team, ruleset }
    }

    pub fn choose_roster(self, roster: Roster) -> Result<RosterSelectedTeam, DomainError> {
        if self.ruleset.is_roster_not_allowed(&roster.id) {
            return Err(DomainError::RosterNotAllowed);
        }
        Ok(RosterSelectedTeam::new(self, roster))
    }
}

impl PartialEq for RulesetSelectedTeam {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Entity for RulesetSelectedTeam {
    fn get_id(&self) -> TeamId {
        self.team.get_id()
    }

    fn get_created_by(&self) -> TeamId {
        self.team.get_created_by()
    }
}