use crate::app::ranking::ports::IRankingTeamLinksPort;
use crate::app::routes::AppRoutes;

/// Seul point où `ranking` rencontre les routes de `teams` (carte 567).
pub struct RankingTeamLinksAdapter;

impl IRankingTeamLinksPort for RankingTeamLinksAdapter {
    fn team_detail_url(&self, space_id: &str, team_id: &str) -> String {
        AppRoutes::default().teams.team_detail(space_id, team_id)
    }

    fn team_identity_url(&self, space_id: &str, team_id: &str) -> String {
        AppRoutes::default()
            .teams
            .team_identity_widget(space_id, team_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_adresses_sont_celles_de_teams() {
        let routes = AppRoutes::default().teams;
        let adapter = RankingTeamLinksAdapter;
        assert_eq!(
            adapter.team_detail_url("sp", "tm"),
            routes.team_detail("sp", "tm")
        );
        assert_eq!(
            adapter.team_identity_url("sp", "tm"),
            routes.team_identity_widget("sp", "tm")
        );
    }
}
