use crate::app::shared_kernel::common_types::{Entity, UserId};
use crate::app::shared_kernel::id_service::IdService;
use crate::app::shared_kernel::team::{BaseTeamInfo, TeamId};
use crate::app::team_creation::domain::creation_rules::CreationRules;
use crate::app::team_creation::domain::ruleset::Ruleset;
use crate::app::team_creation::domain::team_ruleset_selected::RulesetSelectedTeam;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DraftTeam {
    entity_id: TeamId,
    created_by: UserId,
    base_infos: BaseTeamInfo,
    coach_name: String,
    /// Référence opaque vers le contexte Compétition — jamais jointé.
    competition_id: String,
    /// Référence opaque vers le contexte Compétition — jamais jointé.
    season_id: String,
    /// Copie dénormalisée des règles de création au moment de la sélection.
    creation_rules: CreationRules,
}

impl DraftTeam {
    pub fn new<T: IdService>(
        id_service: &T,
        created_by: UserId,
        base_team_infos: BaseTeamInfo,
        coach_name: String,
        competition_id: String,
        season_id: String,
        creation_rules: CreationRules,
    ) -> Self {
        DraftTeam {
            entity_id: id_service.generate_id(),
            created_by,
            base_infos: base_team_infos,
            coach_name,
            competition_id,
            season_id,
            creation_rules,
        }
    }

    pub fn base_infos(&self) -> &BaseTeamInfo {
        &self.base_infos
    }

    pub fn coach_name(&self) -> &str {
        &self.coach_name
    }

    pub fn season_id(&self) -> &str {
        &self.season_id
    }

    pub fn competition_id(&self) -> &str {
        &self.competition_id
    }

    pub fn creation_rules(&self) -> &CreationRules {
        &self.creation_rules
    }

    /// Dérive la `Ruleset` domaine depuis les règles dénormalisées embarquées.
    pub fn derive_ruleset(&self) -> Ruleset {
        self.creation_rules.to_ruleset(&self.season_id)
    }

    /// Reconstruit un `DraftTeam` depuis les données persistées (repository → domaine).
    pub fn from_parts(
        entity_id: TeamId,
        created_by: UserId,
        base_infos: BaseTeamInfo,
        coach_name: String,
        competition_id: String,
        season_id: String,
        creation_rules: CreationRules,
    ) -> Self {
        DraftTeam {
            entity_id,
            created_by,
            base_infos,
            coach_name,
            competition_id,
            season_id,
            creation_rules,
        }
    }

    pub fn select_ruleset(self, ruleset: Ruleset) -> RulesetSelectedTeam {
        RulesetSelectedTeam::new(self, ruleset)
    }
}

impl PartialEq<Self> for DraftTeam {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl Entity for DraftTeam {
    fn get_id(&self) -> TeamId {
        self.entity_id.clone()
    }

    fn get_created_by(&self) -> TeamId {
        self.created_by.clone()
    }
}
