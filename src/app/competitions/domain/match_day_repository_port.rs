use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use async_trait::async_trait;

#[derive(Debug)]
pub enum MatchDayRepositoryError {
    Database(String),
}

impl std::fmt::Display for MatchDayRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "database error: {}", e),
        }
    }
}

#[async_trait]
pub trait IMatchDayRepository: Send + Sync {
    async fn find_by_season(
        &self,
        season_id: &str,
    ) -> Result<Vec<MatchDay>, MatchDayRepositoryError>;

    async fn find_by_id(
        &self,
        match_day_id: &str,
    ) -> Result<Option<MatchDay>, MatchDayRepositoryError>;

    async fn save_match_day(
        &self,
        match_day: &MatchDay,
    ) -> Result<(), MatchDayRepositoryError>;

    async fn delete_match_day(
        &self,
        match_day_id: &str,
    ) -> Result<(), MatchDayRepositoryError>;

    async fn save_pairing(
        &self,
        match_day_id: &str,
        pairing: &Pairing,
    ) -> Result<(), MatchDayRepositoryError>;

    async fn delete_pairing(
        &self,
        pairing_id: &str,
    ) -> Result<(), MatchDayRepositoryError>;

    async fn clear_pairings(
        &self,
        match_day_id: &str,
    ) -> Result<(), MatchDayRepositoryError>;

    async fn clear_all_pairings(
        &self,
        season_id: &str,
    ) -> Result<(), MatchDayRepositoryError>;

    async fn ensure_match_days_from_structure(
        &self,
        season_id: &str,
        entries: &[(String, String, String, Option<String>, Option<String>)],
    ) -> Result<(), MatchDayRepositoryError>;
}
