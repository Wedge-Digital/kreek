use crate::app::match_report::domain::events::MatchReportDomainEvent;
use crate::app::match_report::domain::match_report_state::MatchReportState;
use crate::app::match_report::domain::value_objects::TeamSide;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchActionRow {
    pub action_id: String,
    pub turn_number: i16,
    pub player_id: String,
    pub player_type: String,
    pub action_json: serde_json::Value,
    pub player_display_name: String,
}

#[derive(Debug)]
pub enum RepositoryError {
    ConcurrentWrite,
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
    Database(sqlx::Error),
    Rehydration(String),
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConcurrentWrite => write!(f, "écriture concurrente détectée"),
            Self::Serialization(e) => write!(f, "erreur de sérialisation : {e}"),
            Self::Deserialization(e) => write!(f, "erreur de désérialisation : {e}"),
            Self::Database(e) => write!(f, "erreur base de données : {e}"),
            Self::Rehydration(e) => write!(f, "erreur de rehydratation : {e}"),
        }
    }
}

#[async_trait]
pub trait IMatchReportRepository: Send + Sync {
    async fn append(
        &self,
        match_report_id: &str,
        event: &MatchReportDomainEvent,
        expected_version: u64,
    ) -> Result<u64, RepositoryError>;

    async fn find_by_id(
        &self,
        match_report_id: &str,
    ) -> Result<Option<MatchReportState>, RepositoryError>;

    async fn append_many(
        &self,
        match_report_id: &str,
        events: Vec<MatchReportDomainEvent>,
        expected_version: u64,
    ) -> Result<u64, RepositoryError>;

    async fn find_id_by_pairing(
        &self,
        pairing_id: &str,
    ) -> Result<Option<String>, RepositoryError>;

    async fn find_id_by_round_and_teams(
        &self,
        round_id: &str,
        team_a: &str,
        team_b: &str,
    ) -> Result<Option<String>, RepositoryError>;

    async fn find_actions_by_match_and_side(
        &self,
        match_report_id: &str,
        side: TeamSide,
    ) -> Result<Vec<MatchActionRow>, RepositoryError>;
}
