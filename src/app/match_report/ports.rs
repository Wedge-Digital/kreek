use async_trait::async_trait;

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
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn is_team_ready_to_play(
        &self,
        team_id: &str,
    ) -> Result<bool, String>;

    async fn find_team_info(&self, team_id: &str) -> Option<TeamInfoDto>;

    async fn find_team_value(&self, team_id: &str) -> Option<u32>;

    async fn find_team_treasury(&self, team_id: &str) -> Option<u32>;
}
