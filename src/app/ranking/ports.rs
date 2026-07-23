use async_trait::async_trait;

// ── ACL vers le BC `competitions` (règles de classement + équipes inscrites) ───
// `ranking` ne parle jamais directement à `teams` — uniquement à `competitions`,
// qui ré-expose ce dont `ranking` a besoin via son propre port vers `teams`
// (`ITeamInfoPort`, déjà en place).

pub struct RankingRulesInfo {
    pub win_points: u32,
    pub draw_points: u32,
    pub lose_points: u32,
}

pub struct EnrolledTeamInfo {
    pub team_id: String,
    pub team_name: String,
}

#[async_trait]
pub trait IRankingCompetitionPort: Send + Sync {
    async fn find_ranking_rules(&self, season_id: &str) -> Option<RankingRulesInfo>;
    async fn find_enrolled_teams(&self, season_id: &str) -> Vec<EnrolledTeamInfo>;
}
