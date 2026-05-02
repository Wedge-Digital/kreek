use crate::app::team_creation::domain::ruleset::Ruleset;

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

/// Port de lecture pour les données de référence (rulesets).
pub trait RulesetRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Ruleset>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<Ruleset>, RepositoryError>;
}