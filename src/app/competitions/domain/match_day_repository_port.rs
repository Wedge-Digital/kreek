use crate::app::competitions::domain::match_day::{MatchDay, Pairing};
use async_trait::async_trait;

pub struct PairingDisplayDto {
    pub pairing_id: String,
    pub round_id: String,
    pub round_name: String,
    pub round_position: i32,
    pub round_date_start: Option<String>,
    pub round_date_end: Option<String>,
    pub round_day_type: String,
    pub home_team_id: String,
    pub home_team_name: String,
    pub home_roster_name: String,
    pub home_coach_name: String,
    pub home_logo_url: Option<String>,
    pub home_initials: String,
    pub away_team_id: String,
    pub away_team_name: String,
    pub away_roster_name: String,
    pub away_coach_name: String,
    pub away_logo_url: Option<String>,
    pub away_initials: String,
    pub match_status: String,
    pub home_score: Option<i32>,
    pub away_score: Option<i32>,
    pub home_casualties: Option<i32>,
    pub away_casualties: Option<i32>,
    pub match_report_url: Option<String>,
}

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

    async fn list_resultats(
        &self,
        season_id: &str,
        cursor_position: Option<i32>,
        limit_rounds: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError>;

    async fn list_calendrier(
        &self,
        season_id: &str,
        cursor_position: Option<i32>,
        limit_rounds: u32,
    ) -> Result<Vec<PairingDisplayDto>, MatchDayRepositoryError>;
}
