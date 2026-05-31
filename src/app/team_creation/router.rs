use axum::{routing::{get, post}, Router};
use crate::app::team_creation::io::web::build_team::{
    build_team, buy_reroll, buy_staff, fire_player, get_roster_players, hire_player,
    remove_reroll, remove_staff, submit_team,
};
use crate::app::team_creation::io::web::team_detail::team_detail;
use crate::app::team_creation::io::web::draft_team::draft_team;
use crate::app::team_creation::io::web::my_teams::my_teams;
use crate::app::team_creation::io::web::post_draft_team::post_draft_team;
use crate::app::team_creation::routes::path;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(path::DRAFT_TEAM,     get(draft_team).post(post_draft_team))
        .route(path::TEAM_BUILD,     get(build_team))
        .route(path::MY_TEAMS,       get(my_teams))
        .route(path::ROSTER_PLAYERS, get(get_roster_players))
        .route(path::HIRE_PLAYER,    axum::routing::post(hire_player))
        .route(path::FIRE_PLAYER,    axum::routing::post(fire_player))
        .route(path::BUY_STAFF,      axum::routing::post(buy_staff))
        .route(path::REMOVE_STAFF,   axum::routing::post(remove_staff))
        .route(path::BUY_REROLL,     axum::routing::post(buy_reroll))
        .route(path::REMOVE_REROLL,  axum::routing::post(remove_reroll))
        .route(path::SUBMIT_TEAM,    axum::routing::post(submit_team))
        .route(path::TEAM_DETAIL,    get(team_detail))
}