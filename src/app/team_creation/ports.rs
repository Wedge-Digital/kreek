use crate::app::global_types::global_type::EntityId;
use crate::app::team_creation::team_draft::DraftTeam;
use crate::app::team_creation::team_ruleset_selected::RulesetSelectedTeam;
use crate::app::team_creation::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::ruleset::Ruleset;
use crate::app::team_creation::common_types::TeamId;

#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    PersistenceError(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::NotFound => write!(f, "Équipe introuvable"),
            RepositoryError::PersistenceError(msg) => write!(f, "Erreur de persistance : {}", msg),
        }
    }
}

/// Port de persistance pour toutes les phases de vie d'une équipe.
pub trait TeamRepository: Send + Sync {
    async fn find_draft(&self, id: &TeamId) -> Result<Option<DraftTeam>, RepositoryError>;
    async fn save_draft(&self, team: &DraftTeam) -> Result<(), RepositoryError>;

    async fn find_ruleset_selected(&self, id: &TeamId) -> Result<Option<RulesetSelectedTeam>, RepositoryError>;
    async fn save_ruleset_selected(&self, team: &RulesetSelectedTeam) -> Result<(), RepositoryError>;

    async fn find_roster_selected(&self, id: &TeamId) -> Result<Option<RosterSelectedTeam>, RepositoryError>;
    async fn save_roster_selected(&self, team: &RosterSelectedTeam) -> Result<(), RepositoryError>;
}

/// Port de lecture pour les données de référence (rulesets).
pub trait RulesetRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Ruleset>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<Ruleset>, RepositoryError>;
}