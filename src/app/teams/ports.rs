use async_trait::async_trait;
use crate::app::teams::domain::team::{Team, TeamDomainEvent};

#[derive(Debug)]
pub enum RepositoryError {
    ConcurrentWrite,
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
    Database(sqlx::Error),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcurrentWrite       => write!(f, "écriture concurrente détectée"),
            Self::Serialization(e)      => write!(f, "erreur de sérialisation : {e}"),
            Self::Deserialization(e)    => write!(f, "erreur de désérialisation : {e}"),
            Self::Database(e)           => write!(f, "erreur base de données : {e}"),
        }
    }
}

#[async_trait]
pub trait ITeamRepository: Send + Sync {
    /// Appende un événement dans l'event store.
    /// Retourne la nouvelle version. Échoue avec ConcurrentWrite si
    /// expected_version ne correspond pas à la version courante en base.
    async fn append(
        &self,
        team_id:          &str,
        event:            &TeamDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError>;

    /// Charge tous les événements d'une équipe et hydrate l'agrégat par rejeu.
    async fn find_by_id(&self, team_id: &str) -> Result<Option<Team>, RepositoryError>;
}
