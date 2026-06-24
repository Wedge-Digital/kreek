use async_trait::async_trait;

pub struct CompetitionOptionDto {
    pub competition_id: String,
    pub name: String,
}

pub struct SeasonOptionDto {
    pub season_id: String,
    pub name: String,
}

pub struct RoundOptionDto {
    pub round_id: String,
    pub name: String,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
}

#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    async fn list_competitions_with_active_season(
        &self,
        space_id: &str,
    ) -> Result<Vec<CompetitionOptionDto>, String>;

    async fn list_seasons(
        &self,
        competition_id: &str,
    ) -> Result<Vec<SeasonOptionDto>, String>;

    async fn list_rounds(
        &self,
        season_id: &str,
    ) -> Result<Vec<RoundOptionDto>, String>;

    async fn is_competition_admin(
        &self,
        competition_id: &str,
        coach_id: &str,
    ) -> Result<bool, String>;
}

pub struct EnrolledTeamDto {
    pub team_id: String,
    pub team_name: String,
    pub coach_id: String,
    pub coach_name: String,
    pub roster_name: String,
    pub team_value: u32,
    pub logo_url: Option<String>,
    pub game_phase: Option<String>,
}

#[async_trait]
pub trait ITeamDataPort: Send + Sync {
    async fn list_enrolled_teams(
        &self,
        season_id: &str,
    ) -> Result<Vec<EnrolledTeamDto>, String>;

    async fn is_team_ready_to_play(
        &self,
        team_id: &str,
    ) -> Result<bool, String>;
}
