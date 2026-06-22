pub mod path {
    pub const COMPETITION_LIST: &str = "/app/{space_id}/competitions";
    pub const COMPETITION_DETAIL: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}";
    pub const COMPETITION_NEW: &str = "/app/{space_id}/competitions/create";
    pub const COMPETITION_NEW_INFO: &str = "/app/{space_id}/competitions/create/{competition_id}";
    pub const COMPETITION_NEW_RULES: &str =
        "/app/{space_id}/competitions/create/{competition_id}/{season_id}/rules";
    pub const COMPETITION_NEW_STRUCTURE: &str =
        "/app/{space_id}/competitions/create/{competition_id}/{season_id}/structure";
    pub const COMPETITION_NEW_INVITATIONS: &str =
        "/app/{space_id}/competitions/create/{competition_id}/{season_id}/invitations";
    pub const COMPETITION_NEW_VALIDATION: &str =
        "/app/{space_id}/competitions/create/{competition_id}/{season_id}/validation";
    pub const COMPETITION_TAB_STANDINGS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/standings";
    pub const COMPETITION_TAB_MATCHES: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/matches";
    pub const COMPETITION_TAB_TEAMS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/teams";
    pub const COMPETITION_TAB_STATS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/stats";
    pub const COMPETITION_WIDGET: &str = "/app/{space_id}/competitions/widget";
    pub const COMPETITION_WIDGET_SEASONS: &str = "/app/{space_id}/competitions/widget/seasons";
    pub const COMPETITION_WIDGET_DETAIL: &str = "/app/{space_id}/competitions/widget/{season_id}";
    pub const COMPETITION_ADMIN: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin";
    pub const COMPETITION_ADMIN_DASHBOARD: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/dashboard";
    pub const COMPETITION_ADMIN_ENROLLMENTS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/enrollments";
    pub const COMPETITION_ADMIN_GROUPS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/groups";
    pub const COMPETITION_ADMIN_GROUPS_UNASSIGNED: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/groups/unassigned";
    pub const COMPETITION_ADMIN_GROUPS_CARDS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/groups/cards";
    pub const COMPETITION_ADMIN_GROUPS_RANDOM_DRAW: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/groups/random-draw";
    pub const COMPETITION_ADMIN_GROUPS_RESET: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/groups/reset";
    pub const COMPETITION_ADMIN_GROUPS_ASSIGN: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/groups/assign";
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Routes;

impl Routes {
    pub fn all_competitions(&self, sid: &str) -> String {
        path::COMPETITION_LIST.replace("{space_id}", sid)
    }
    pub fn competition_detail(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_DETAIL
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn new_competition(&self, sid: &str) -> String {
        path::COMPETITION_NEW.replace("{space_id}", sid)
    }
    pub fn new_competition_info(&self, sid: &str, cid: &str) -> String {
        path::COMPETITION_NEW_INFO
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
    }
    pub fn new_competition_rules(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_NEW_RULES
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn new_competition_structure(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_NEW_STRUCTURE
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn new_competition_invitations(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_NEW_INVITATIONS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn new_competition_validation(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_NEW_VALIDATION
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn competition_tab_standings(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_TAB_STANDINGS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn competition_tab_matches(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_TAB_MATCHES
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn competition_tab_teams(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_TAB_TEAMS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn competition_tab_stats(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_TAB_STATS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn competition_widget(&self, sid: &str) -> String {
        path::COMPETITION_WIDGET.replace("{space_id}", sid)
    }
    pub fn competition_widget_seasons(&self, sid: &str) -> String {
        path::COMPETITION_WIDGET_SEASONS.replace("{space_id}", sid)
    }
    pub fn competition_widget_detail(&self, sid: &str, season_id: &str) -> String {
        path::COMPETITION_WIDGET_DETAIL
            .replace("{space_id}", sid)
            .replace("{season_id}", season_id)
    }
    pub fn admin(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_dashboard(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_DASHBOARD
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_enrollments(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_ENROLLMENTS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_groups(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_GROUPS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_groups_unassigned(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_GROUPS_UNASSIGNED
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_groups_cards(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_GROUPS_CARDS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_groups_random_draw(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_GROUPS_RANDOM_DRAW
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_groups_reset(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_GROUPS_RESET
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_groups_assign(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_GROUPS_ASSIGN
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
}
