use crate::app::match_report::domain::value_objects::{ActionPlayer, MatchAction};
use async_trait::async_trait;

#[async_trait]
pub trait IPlayerDataPort: Send + Sync {
    async fn count_available_players(&self, team_id: &str) -> Result<usize, String>;
    async fn find_player_display(&self, player_id: &str) -> Option<String>;
    async fn find_player_position(&self, player_id: &str) -> Option<String>;
    async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto>;
}

#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String>;

    async fn find_tier_rules_for_roster(
        &self,
        season_id: &str,
        roster_id: &str,
    ) -> Option<TierRulesDto>;

    async fn find_round_context(
        &self,
        season_id: &str,
        round_id: &str,
    ) -> Option<RoundContextDto>;
}

#[derive(Debug, Clone)]
pub struct RoundContextDto {
    pub competition_name: String,
    pub season_name: String,
    pub round_name: String,
}

#[derive(Debug, Default)]
pub struct TierRulesDto {
    pub allowed_inducements: Vec<InducementSpecDto>,
    pub allowed_star_players: Vec<InducementSpecDto>,
}

#[derive(Debug, Clone)]
pub struct InducementSpecDto {
    pub uid: String,
    pub max_qty: u8,
    pub unit_cost: u32,
}

#[derive(Default)]
pub struct TeamInfoDto {
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub roster_id: String,
    pub logo_url: Option<String>,
    pub dedicated_fans: u32,
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn is_team_ready_to_play(
        &self,
        team_id: &str,
    ) -> Result<bool, String>;

    async fn is_coach_of_team(&self, team_id: &str, user_id: &str) -> Result<bool, String>;

    async fn find_team_info(&self, team_id: &str) -> Option<TeamInfoDto>;

    async fn find_team_value(&self, team_id: &str) -> Option<u32>;

    async fn find_team_treasury(&self, team_id: &str) -> Option<u32>;

    async fn find_journeyman_position(&self, team_id: &str) -> Option<JourneymanPositionDto>;

    async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto>;
}

#[derive(Debug)]
pub struct JourneymanPositionDto {
    pub position_uid: String,
    pub position_name: String,
}

pub struct RosterPositionDto {
    pub position_uid:  String,
    pub position_name: String,
    pub base_cost:     u32,
    pub max_qty:       u8,
    pub is_journeyman: bool,
}

pub struct PositionCountDto {
    pub position_uid: String,
    pub count:        u8,
}

#[async_trait]
pub trait ICoachDataPort: Send + Sync {
    async fn find_coach_name(&self, coach_id: &str) -> Option<String>;
}

/// Consultation du profil d'un membre d'espace, pour les contrôles d'accès.
///
/// Le BC `match_report` ne connaît pas le BC propriétaire des espaces : il pose
/// la seule question dont il a besoin. Sans ce port, le contrôle d'accès du
/// recap devrait atteindre `state.spaces` directement — une référence croisée
/// entre BCs, et un contrôle non testable puisque `AppState` n'est pas
/// constructible en test unitaire.
#[async_trait]
pub trait ISpaceAdminPort: Send + Sync {
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool;
}

#[async_trait]
pub trait ISppCalculatorPort: Send + Sync {
    async fn calculate_match_spp(
        &self,
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        home_roster_id: &str,
        away_roster_id: &str,
    ) -> SppMatchResult;
}

#[derive(Debug, Default)]
pub struct SppMatchResult {
    pub home: Vec<PlayerSppDto>,
    pub away: Vec<PlayerSppDto>,
}

#[derive(Debug, Clone)]
pub struct PlayerSppDto {
    pub action_player: ActionPlayer,
    pub spp: u8,
}
