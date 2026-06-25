use crate::app::teams::domain::team::{Team, TeamDomainEvent};
use async_trait::async_trait;

#[async_trait]
pub trait IPlayerCountPort: Send + Sync {
    async fn count_for_team(&self, team_id: &str) -> u32;
}

pub trait IJourneymanTypePort: Send + Sync {
    fn journeyman_type_for_roster(&self, roster_id: &str) -> String;
}

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
            Self::ConcurrentWrite => write!(f, "écriture concurrente détectée"),
            Self::Serialization(e) => write!(f, "erreur de sérialisation : {e}"),
            Self::Deserialization(e) => write!(f, "erreur de désérialisation : {e}"),
            Self::Database(e) => write!(f, "erreur base de données : {e}"),
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
        team_id: &str,
        event: &TeamDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError>;

    /// Charge tous les événements d'une équipe et hydrate l'agrégat par rejeu.
    async fn find_by_id(&self, team_id: &str) -> Result<Option<Team>, RepositoryError>;

    /// Liste les équipes inscrites à une saison par statut.
    async fn find_by_season_and_status(
        &self,
        season_id: &str,
        status: &str,
    ) -> Result<Vec<TeamEnrollmentRow>, RepositoryError>;

    async fn find_enrolled_for_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<TeamCardRow>, RepositoryError>;
}

pub struct TeamEnrollmentRow {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub status: String,
}

pub struct TeamCardRow {
    pub team_id: String,
    pub team_name: String,
    pub coach_id: String,
    pub coach_name: String,
    pub roster_name: String,
    pub logo_url: Option<String>,
    pub team_value: u32,
    pub game_phase: Option<String>,
}
