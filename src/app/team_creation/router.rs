use crate::app::team_creation::io::web::build_team::{
    build_team, buy_reroll, buy_staff, remove_reroll, remove_staff, submit_team,
};
use crate::app::team_creation::io::web::widgets::cart_widget::cart_widget;
use crate::app::team_creation::io::web::widgets::player_table_widget::{
    fire_player, hire_player, player_table_widget,
};
use crate::app::team_creation::io::web::widgets::roster_picker_widget::roster_picker_widget;
use crate::app::team_creation::io::web::draft_team::draft_team;
use crate::app::team_creation::io::web::finalize_team::{finalize_team, post_finalize_team};
use crate::app::team_creation::io::web::my_teams::my_teams;
use crate::app::team_creation::io::web::post_draft_team::post_draft_team;
use crate::app::team_creation::io::web::set_league::set_league;
use crate::app::team_creation::io::web::set_player_identity::set_player_identity;
use crate::app::team_creation::io::web::set_special_rule::set_special_rule;
use crate::app::team_creation::io::web::team_detail::team_detail;
use crate::app::team_creation::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::DRAFT_TEAM, get(draft_team).post(post_draft_team))
        .route(path::TEAM_BUILD, get(build_team))
        .route(path::MY_TEAMS, get(my_teams))
        .route(path::PLAYER_TABLE_WIDGET, get(player_table_widget))
        .route(path::HIRE_PLAYER, axum::routing::post(hire_player))
        .route(path::FIRE_PLAYER, axum::routing::post(fire_player))
        .route(path::BUY_STAFF, axum::routing::post(buy_staff))
        .route(path::REMOVE_STAFF, axum::routing::post(remove_staff))
        .route(path::BUY_REROLL, axum::routing::post(buy_reroll))
        .route(path::REMOVE_REROLL, axum::routing::post(remove_reroll))
        .route(path::SUBMIT_TEAM, axum::routing::post(submit_team))
        .route(path::TEAM_DETAIL, get(team_detail))
        .route(path::SET_PLAYER_IDENTITY, axum::routing::post(set_player_identity))
        .route(path::SET_LEAGUE, axum::routing::post(set_league))
        .route(path::SET_SPECIAL_RULE, axum::routing::post(set_special_rule))
        .route(path::FINALIZE_TEAM, get(finalize_team).post(post_finalize_team))
        .route(path::CART_WIDGET, get(cart_widget))
        .route(path::ROSTER_PICKER_WIDGET, get(roster_picker_widget))
}
