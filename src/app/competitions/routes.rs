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
    pub const COMPETITION_TAB_DETAILED_STANDINGS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/detailed-standings";
    pub const COMPETITION_TAB_MATCHES: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/matches";
    pub const COMPETITION_TAB_RESULTATS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/resultats";
    pub const COMPETITION_TAB_CALENDRIER: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/calendrier";
    pub const COMPETITION_TAB_TEAMS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/teams";
    pub const COMPETITION_TAB_STATS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/stats";
    pub const COMPETITION_WIDGET: &str = "/app/{space_id}/competitions/widget";
    pub const COMPETITION_WIDGET_DETAIL: &str = "/app/{space_id}/competitions/widget/{season_id}";
    pub const COMPETITION_WIDGET_JSON_COMPETITIONS: &str =
        "/app/{space_id}/competitions/widget/json/competitions";
    pub const COMPETITION_WIDGET_JSON_SEASONS: &str =
        "/app/{space_id}/competitions/widget/json/seasons";
    pub const COMPETITION_WIDGET_JSON_ROUNDS: &str =
        "/app/{space_id}/competitions/widget/json/rounds";
    pub const COMPETITION_WIDGET_TESTER: &str = "/competitions/widget/tester";
    pub const COMPETITION_LATEST_RESULTS_WIDGET: &str =
        "/app/{space_id}/competitions/widget/latest-results";
    /// Les matchs d'une équipe. **Le chemin ne porte que le `team_id`** : la
    /// compétition s'y résout côté serveur, et l'y mettre laisserait un admin
    /// d'une autre compétition la forcer (cf. `team_matches_widget.rs`).
    pub const TEAM_MATCHES_WIDGET: &str = "/app/{space_id}/competitions/teams/{team_id}/matches";
    pub const COMPETITION_ADMIN: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin";
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
    pub const COMPETITION_ADMIN_SCHEDULE: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule";
    pub const COMPETITION_ADMIN_SCHEDULE_ROUNDS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/rounds";
    pub const COMPETITION_ADMIN_SCHEDULE_ROUND_DETAIL: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/round";
    pub const COMPETITION_ADMIN_SCHEDULE_GENERATE_ALL: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/generate-all";
    pub const COMPETITION_ADMIN_SCHEDULE_CLEAR_ALL: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/clear-all";
    pub const COMPETITION_ADMIN_SCHEDULE_ADD_ROUND: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/add-round";
    pub const COMPETITION_ADMIN_SCHEDULE_ADD_REST: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/add-rest";
    pub const COMPETITION_ADMIN_SCHEDULE_ROUND: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/rounds/{round_id}";
    pub const COMPETITION_ADMIN_SCHEDULE_GENERATE_ROUND: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/generate-round";
    pub const COMPETITION_ADMIN_SCHEDULE_CLEAR_ROUND: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/clear-round";
    pub const COMPETITION_ADMIN_SCHEDULE_ADD_MATCH: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/add-match";
    pub const COMPETITION_ADMIN_SCHEDULE_DELETE_MATCH: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/schedule/delete-match";
    pub const COMPETITION_ADMIN_SUMMARY: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/summary";
    pub const COMPETITION_ADMIN_SETTINGS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings";
    pub const COMPETITION_ADMIN_SETTINGS_GENERAL: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings/general";
    pub const COMPETITION_ADMIN_SETTINGS_RANKING: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings/ranking";
    pub const COMPETITION_ADMIN_SETTINGS_POOLS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings/pools";
    pub const COMPETITION_ADMIN_SETTINGS_TIERS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings/tiers";
    pub const COMPETITION_ADMIN_SETTINGS_VISIBILITY: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings/visibility";
    pub const NOTIFICATION_SETTINGS_WIDGET: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/notifications-widget";
    pub const NOTIFICATION_SETTINGS: &str =
        "/app/{space_id}/competitions/{competition_id}/{season_id}/notifications";
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
    pub fn competition_tab_detailed_standings(
        &self,
        sid: &str,
        cid: &str,
        season_id: &str,
    ) -> String {
        path::COMPETITION_TAB_DETAILED_STANDINGS
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
    pub fn competition_tab_resultats(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_TAB_RESULTATS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn competition_tab_calendrier(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_TAB_CALENDRIER
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
    pub fn team_matches_widget(&self, sid: &str, team_id: &str) -> String {
        path::TEAM_MATCHES_WIDGET
            .replace("{space_id}", sid)
            .replace("{team_id}", team_id)
    }

    pub fn latest_results_widget(&self, sid: &str) -> String {
        path::COMPETITION_LATEST_RESULTS_WIDGET.replace("{space_id}", sid)
    }
    pub fn competition_widget_json_competitions(&self, sid: &str) -> String {
        path::COMPETITION_WIDGET_JSON_COMPETITIONS.replace("{space_id}", sid)
    }
    pub fn competition_widget_json_seasons(&self, sid: &str) -> String {
        path::COMPETITION_WIDGET_JSON_SEASONS.replace("{space_id}", sid)
    }
    pub fn competition_widget_json_rounds(&self, sid: &str) -> String {
        path::COMPETITION_WIDGET_JSON_ROUNDS.replace("{space_id}", sid)
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
    pub fn admin_schedule(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_rounds(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_ROUNDS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_round_detail(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_ROUND_DETAIL
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_generate_all(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_GENERATE_ALL
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_clear_all(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_CLEAR_ALL
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_add_round(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_ADD_ROUND
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_add_rest(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_ADD_REST
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_round(
        &self,
        sid: &str,
        cid: &str,
        season_id: &str,
        round_id: &str,
    ) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_ROUND
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
            .replace("{round_id}", round_id)
    }
    pub fn admin_schedule_generate_round(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_GENERATE_ROUND
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_clear_round(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_CLEAR_ROUND
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_add_match(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_ADD_MATCH
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_schedule_delete_match(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SCHEDULE_DELETE_MATCH
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_summary(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SUMMARY
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_settings(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SETTINGS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_settings_general(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SETTINGS_GENERAL
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_settings_ranking(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SETTINGS_RANKING
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_settings_pools(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SETTINGS_POOLS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_settings_tiers(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SETTINGS_TIERS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    pub fn admin_settings_visibility(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::COMPETITION_ADMIN_SETTINGS_VISIBILITY
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
    /// Le `mode` est porté par la query : `deferred` pour le magicien,
    /// `autosave` pour l'onglet Synthèse. Un mode inconnu vaut 400.
    pub fn notification_settings_widget(
        &self,
        sid: &str,
        cid: &str,
        season_id: &str,
        mode: &str,
    ) -> String {
        format!(
            "{}?mode={mode}",
            path::NOTIFICATION_SETTINGS_WIDGET
                .replace("{space_id}", sid)
                .replace("{competition_id}", cid)
                .replace("{season_id}", season_id)
        )
    }
    pub fn notification_settings(&self, sid: &str, cid: &str, season_id: &str) -> String {
        path::NOTIFICATION_SETTINGS
            .replace("{space_id}", sid)
            .replace("{competition_id}", cid)
            .replace("{season_id}", season_id)
    }
}
