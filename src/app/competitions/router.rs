use crate::app::competitions::io::web::admin::admin_page::admin_page;
use crate::app::competitions::io::web::admin::enrollments_tab::enrollments_tab;
use crate::app::competitions::io::web::admin::groups_actions::{
    post_assign_team, post_random_draw, post_reset_groups,
};
use crate::app::competitions::io::web::admin::groups_tab::groups_tab;
use crate::app::competitions::io::web::admin::groups_widgets::{
    group_cards_widget, unassigned_pool_widget,
};
use crate::app::competitions::io::web::admin::schedule_actions::{
    delete_match, delete_round, post_add_match, post_add_rest, post_add_round, post_clear_all,
    post_clear_round_pairings, post_generate_all, post_generate_round_pairings, put_update_round,
};
use crate::app::competitions::io::web::admin::schedule_tab::schedule_tab;
use crate::app::competitions::io::web::admin::schedule_widgets::{
    schedule_round_detail_widget, schedule_sidebar_widget,
};
use crate::app::competitions::io::web::admin::settings::general_panel::{
    get_settings_general, post_settings_general,
};
use crate::app::competitions::io::web::admin::settings::pools_panel::{
    get_settings_pools, post_settings_pools,
};
use crate::app::competitions::io::web::admin::settings::ranking_panel::{
    get_settings_ranking, post_settings_ranking,
};
use crate::app::competitions::io::web::admin::settings::settings_tab::settings_tab;
use crate::app::competitions::io::web::admin::settings::tiers_panel::{
    get_settings_tiers, post_settings_tiers,
};
use crate::app::competitions::io::web::admin::summary_tab::summary_tab_fragment;
use crate::app::competitions::io::web::all_competition::get_all_competition;
use crate::app::competitions::io::web::calendrier_tab_controller::get_calendrier_tab;
use crate::app::competitions::io::web::competition_detail::{
    get_competition_detail, get_tab_detailed_standings, get_tab_standings, get_tab_stats,
    get_tab_teams,
};
use crate::app::competitions::io::web::competition_widget::{
    get_competition_widget, get_competition_widget_detail, get_json_competitions, get_json_rounds,
    get_json_seasons,
};
use crate::app::competitions::io::web::new_competition::{
    get_new_competition_phase_1, get_new_competition_phase_1_edit, get_new_competition_phase_2,
    post_competition_rules, post_new_competition, post_update_competition,
};
use crate::app::competitions::io::web::new_competition_phase_3::{
    get_new_competition_phase_3, post_competition_structure,
};
use crate::app::competitions::io::web::new_competition_phase_4::{
    get_new_competition_phase_4, post_competition_invitations,
};
use crate::app::competitions::io::web::new_competition_phase_5::{
    get_new_competition_phase_5, post_finalize_competition,
};
use crate::app::competitions::io::web::resultats_tab_controller::get_resultats_tab;
use crate::app::competitions::io::web::widget_tester_controller::get_competitions_widget_tester;
use crate::app::competitions::io::web::widgets::latest_results_widget::latest_results_widget;
use crate::app::competitions::io::web::widgets::notification_settings_widget::{
    get_notification_settings_widget, post_notification_settings,
};
use crate::app::competitions::routes::path;
use crate::state::AppState;
use axum::routing::{delete, get, post, put};
use axum::Router;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::COMPETITION_LIST, get(get_all_competition))
        .route(path::COMPETITION_DETAIL, get(get_competition_detail))
        .route(
            path::COMPETITION_NEW,
            get(get_new_competition_phase_1).post(post_new_competition),
        )
        .route(
            path::COMPETITION_NEW_INFO,
            get(get_new_competition_phase_1_edit).post(post_update_competition),
        )
        .route(
            path::COMPETITION_NEW_RULES,
            get(get_new_competition_phase_2).post(post_competition_rules),
        )
        .route(
            path::COMPETITION_NEW_STRUCTURE,
            get(get_new_competition_phase_3).post(post_competition_structure),
        )
        .route(
            path::COMPETITION_NEW_INVITATIONS,
            get(get_new_competition_phase_4).post(post_competition_invitations),
        )
        .route(
            path::COMPETITION_NEW_VALIDATION,
            get(get_new_competition_phase_5).post(post_finalize_competition),
        )
        .route(path::COMPETITION_TAB_STANDINGS, get(get_tab_standings))
        .route(
            path::COMPETITION_TAB_DETAILED_STANDINGS,
            get(get_tab_detailed_standings),
        )
        .route(path::COMPETITION_TAB_RESULTATS, get(get_resultats_tab))
        .route(path::COMPETITION_TAB_CALENDRIER, get(get_calendrier_tab))
        .route(path::COMPETITION_TAB_TEAMS, get(get_tab_teams))
        .route(path::COMPETITION_TAB_STATS, get(get_tab_stats))
        .route(path::COMPETITION_WIDGET, get(get_competition_widget))
        .route(
            path::COMPETITION_LATEST_RESULTS_WIDGET,
            get(latest_results_widget),
        )
        .route(
            path::COMPETITION_WIDGET_JSON_COMPETITIONS,
            get(get_json_competitions),
        )
        .route(path::COMPETITION_WIDGET_JSON_SEASONS, get(get_json_seasons))
        .route(path::COMPETITION_WIDGET_JSON_ROUNDS, get(get_json_rounds))
        .route(
            path::COMPETITION_WIDGET_DETAIL,
            get(get_competition_widget_detail),
        )
        .route(
            path::COMPETITION_WIDGET_TESTER,
            get(get_competitions_widget_tester),
        )
        .route(path::COMPETITION_ADMIN, get(admin_page))
        .route(path::COMPETITION_ADMIN_SUMMARY, get(summary_tab_fragment))
        .route(
            path::NOTIFICATION_SETTINGS_WIDGET,
            get(get_notification_settings_widget),
        )
        .route(
            path::NOTIFICATION_SETTINGS,
            axum::routing::post(post_notification_settings),
        )
        .route(path::COMPETITION_ADMIN_ENROLLMENTS, get(enrollments_tab))
        .route(path::COMPETITION_ADMIN_GROUPS, get(groups_tab))
        .route(
            path::COMPETITION_ADMIN_GROUPS_UNASSIGNED,
            get(unassigned_pool_widget),
        )
        .route(
            path::COMPETITION_ADMIN_GROUPS_CARDS,
            get(group_cards_widget),
        )
        .route(
            path::COMPETITION_ADMIN_GROUPS_RANDOM_DRAW,
            post(post_random_draw),
        )
        .route(
            path::COMPETITION_ADMIN_GROUPS_RESET,
            post(post_reset_groups),
        )
        .route(
            path::COMPETITION_ADMIN_GROUPS_ASSIGN,
            post(post_assign_team),
        )
        // ── Schedule tab ──
        .route(path::COMPETITION_ADMIN_SCHEDULE, get(schedule_tab))
        .route(path::COMPETITION_ADMIN_SETTINGS, get(settings_tab))
        .route(
            path::COMPETITION_ADMIN_SETTINGS_GENERAL,
            get(get_settings_general).post(post_settings_general),
        )
        .route(
            path::COMPETITION_ADMIN_SETTINGS_RANKING,
            get(get_settings_ranking).post(post_settings_ranking),
        )
        .route(
            path::COMPETITION_ADMIN_SETTINGS_POOLS,
            get(get_settings_pools).post(post_settings_pools),
        )
        .route(
            path::COMPETITION_ADMIN_SETTINGS_TIERS,
            get(get_settings_tiers).post(post_settings_tiers),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_ROUNDS,
            get(schedule_sidebar_widget),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_ROUND_DETAIL,
            get(schedule_round_detail_widget),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_GENERATE_ALL,
            post(post_generate_all),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_CLEAR_ALL,
            post(post_clear_all),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_ADD_ROUND,
            post(post_add_round),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_ADD_REST,
            post(post_add_rest),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_ROUND,
            put(put_update_round).delete(delete_round),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_GENERATE_ROUND,
            post(post_generate_round_pairings),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_CLEAR_ROUND,
            post(post_clear_round_pairings),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_ADD_MATCH,
            post(post_add_match),
        )
        .route(
            path::COMPETITION_ADMIN_SCHEDULE_DELETE_MATCH,
            delete(delete_match),
        )
}
