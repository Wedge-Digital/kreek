use axum::{routing::{get, post}, Router};
use crate::app::team_creation::io::web::build_team::{build_team, fire_player, get_roster_players, hire_player};
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
}