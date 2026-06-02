use crate::app::team_creation::io::web::build_team::{
    build_team, buy_reroll, buy_staff, fire_player, get_roster_players, hire_player, remove_reroll,
    remove_staff, submit_team,
};
use crate::app::team_creation::io::web::draft_team::draft_team;
use crate::app::team_creation::io::web::my_teams::my_teams;
use crate::app::team_creation::io::web::post_draft_team::post_draft_team;
use crate::app::team_creation::io::web::finalize_team::{finalize_team, skill_header};
use crate::app::team_creation::io::web::set_league::set_league;
use crate::app::team_creation::io::web::spp_management::{cancel_spp, spend_spp};
use crate::app::team_creation::io::web::set_player_identity::set_player_identity;
use crate::app::team_creation::io::web::team_detail::team_detail;
use crate::app::team_creation::routes::path;
use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::DRAFT_TEAM, get(draft_team).post(post_draft_team))
        .route(path::TEAM_BUILD, get(build_team))
        .route(path::MY_TEAMS, get(my_teams))
        .route(path::ROSTER_PLAYERS, get(get_roster_players))
        .route(path::HIRE_PLAYER, axum::routing::post(hire_player))
        .route(path::FIRE_PLAYER, axum::routing::post(fire_player))
        .route(path::BUY_STAFF, axum::routing::post(buy_staff))
        .route(path::REMOVE_STAFF, axum::routing::post(remove_staff))
        .route(path::BUY_REROLL, axum::routing::post(buy_reroll))
        .route(path::REMOVE_REROLL, axum::routing::post(remove_reroll))
        .route(path::SUBMIT_TEAM, axum::routing::post(submit_team))
        .route(path::TEAM_DETAIL, get(team_detail))
        .route(
            path::SET_PLAYER_IDENTITY,
            axum::routing::post(set_player_identity),
        )
        .route(path::SET_LEAGUE, axum::routing::post(set_league))
        .route(path::SPEND_SPP, axum::routing::post(spend_spp))
        .route(path::CANCEL_SPP, axum::routing::delete(cancel_spp))
        .route(path::FINALIZE_TEAM, get(finalize_team))
        .route(path::SKILL_HEADER, get(skill_header))
}
