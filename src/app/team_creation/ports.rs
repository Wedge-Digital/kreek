use async_trait::async_trait;
use crate::app::shared_kernel::team::TeamId;
use crate::app::team_creation::domain::ruleset::Ruleset;
use crate::app::team_creation::domain::team_draft::DraftTeam;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;

#[derive(Debug)]
pub enum RepositoryError {
    NotFound,
    PersistenceError(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepositoryError::NotFound           => write!(f, "Équipe introuvable"),
            RepositoryError::PersistenceError(msg) => write!(f, "Erreur de persistance : {}", msg),
        }
    }
}

#[async_trait]
pub trait ITeamDraftRepository: Send + Sync {
    async fn save(&self, team: &DraftTeam, space_id: &str) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &TeamId) -> Result<Option<DraftTeam>, RepositoryError>;
    async fn find_by_coach_and_space(&self, coach_id: &str, space_id: &str) -> Result<Vec<DraftTeam>, RepositoryError>;
}

#[async_trait]
pub trait ITeamRosterRepository: Send + Sync {
    async fn save(&self, team: &RosterSelectedTeam, space_id: &str) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: &TeamId) -> Result<Option<RosterSelectedTeam>, RepositoryError>;
}

/// Port de lecture pour les données de référence (rulesets).
/// Conservé pour compatibilité — la source principale est désormais CreationRules.
pub trait RulesetRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Ruleset>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<Ruleset>, RepositoryError>;
}