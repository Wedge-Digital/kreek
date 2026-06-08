use crate::app::players::domain::events::PlayerDomainEvent;
use crate::app::players::domain::player::{Player, PlayerId, TeamId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
            Self::ConcurrentWrite    => write!(f, "écriture concurrente détectée"),
            Self::Serialization(e)   => write!(f, "erreur de sérialisation : {e}"),
            Self::Deserialization(e) => write!(f, "erreur de désérialisation : {e}"),
            Self::Database(e)        => write!(f, "erreur base de données : {e}"),
        }
    }
}

// ── Projection read model ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquiredSkillProjection {
    pub skill_id:                  String,
    pub skill_name:                String,
    #[serde(default)]
    pub category_css:              String,
    pub mode:                      String,
    pub spp_cost:                  i32,
}

#[derive(Debug, Clone)]
pub struct PlayerProjection {
    pub player_id:       String,
    pub team_id:         String,
    pub space_id:        String,
    pub position_name:   String,
    pub roster_line_id:  String,
    pub personal_name:   String,
    pub jersey:          Option<i16>,
    pub base_skills:     Vec<String>,
    pub acquired_skills: Vec<AcquiredSkillProjection>,
    pub spp:             i32,
    pub value_kpo:       i32,
}

#[async_trait]
pub trait IPlayerProjectionRepository: Send + Sync {
    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<PlayerProjection>, RepositoryError>;
}

// ── Event store port ───────────────────────────────────────────────────────────

#[async_trait]
pub trait IPlayerRepository: Send + Sync {
    async fn append(
        &self,
        player_id: &PlayerId,
        team_id:   &TeamId,
        event:     &PlayerDomainEvent,
        version:   i32,
    ) -> Result<(), RepositoryError>;

    async fn find_by_id(
        &self,
        player_id: &PlayerId,
    ) -> Result<Option<Player>, RepositoryError>;

    async fn find_by_team_id(
        &self,
        team_id: &TeamId,
    ) -> Result<Vec<Player>, RepositoryError>;
}
