use crate::app::shared_kernel::common_types::EntityId;
use crate::app::team_creation::domain::roster::{LeagueId, PlayerId, SpecialRuleId};
use crate::app::team_creation::domain::team_roster_selected::{RosterSelectedTeam, SppPool};
use crate::app::team_creation::io::web::builders::{build_hired_rows, build_player_positions};
use crate::app::team_creation::io::web::view_models::{HiredPlayerRowVm, PlayerPositionVm};
use crate::app::routes::AppRoutes;
use crate::app::team_creation::use_cases::build_team::fire_player as fire_uc;
use crate::app::team_creation::use_cases::build_team::hire_player as hire_uc;
use crate::app::team_creation::use_cases::commands::{FirePlayerCommand, HirePlayerCommand};
use crate::app::team_creation::use_cases::roster_service;
use crate::state::AppState;
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn player_error(msg: &str) -> Response {
    Response::builder()
        .header("HX-Retarget", "#player-table-error")
        .header("HX-Reswap", "innerHTML")
        .header("content-type", "text/html; charset=utf-8")
        .body(Body::from(format!(r#"<p class="table-error">{msg}</p>"#)))
        .unwrap()
}

// ── Widget player table (GET) ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PlayerTableParams {
    #[serde(default)]
    pub roster_uid: String,
}

#[derive(Template)]
#[template(path = "widgets/player-table-widget.html")]
pub struct PlayerTableWidgetTemplate {
    pub positions: Vec<PlayerPositionVm>,
    pub hired_rows: Vec<HiredPlayerRowVm>,
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
    pub has_roster: bool,
}

impl IntoResponse for PlayerTableWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .header("HX-Trigger", "teamMutated")
                .body(Body::from(html))
                .unwrap(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn player_table_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    Query(params): Query<PlayerTableParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if params.roster_uid.is_empty() {
        return PlayerTableWidgetTemplate {
            positions: vec![],
            hired_rows: vec![],
            app_routes: Default::default(),
            space_id,
            team_id,
            has_roster: false,
        }
        .into_response();
    }

    let ref_data = state.team_creation.reference_data.as_ref();

    let roster = match roster_service::load_roster(&params.roster_uid, ref_data) {
        Some(r) => r,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let meta = roster_service::roster_metadata(&params.roster_uid, ref_data).unwrap();

    let roster_def = match ref_data.find_roster_definition(&params.roster_uid) {
        Some(d) => d,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let draft = match state
        .team_creation
        .team_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("player_table_widget draft find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let starting_spp = draft
        .creation_rules()
        .get_starting_xp_for_roster(&params.roster_uid);

    let mut roster_team: RosterSelectedTeam = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(existing)) => match existing.choose_roster(roster) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("choose_roster rejected: {:?}", e);
                return StatusCode::UNPROCESSABLE_ENTITY.into_response();
            }
        },
        Ok(None) => {
            let ruleset = draft.derive_ruleset();
            let ruleset_team = draft.select_ruleset(ruleset);
            match ruleset_team.choose_roster(roster) {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!("choose_roster initial rejected: {:?}", e);
                    return StatusCode::UNPROCESSABLE_ENTITY.into_response();
                }
            }
        }
        Err(e) => {
            tracing::error!("player_table_widget roster_repo find: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    roster_team.spp_pool = SppPool(starting_spp);

    if meta.leagues.len() == 1 {
        roster_team.set_league(LeagueId(meta.leagues[0].clone()));
    }

    if meta.special_rules.len() == 1 {
        roster_team.set_special_rule(SpecialRuleId(meta.special_rules[0].clone()));
    }

    if let Err(e) = state
        .team_creation
        .roster_repository
        .save(&roster_team, &space_id)
        .await
    {
        tracing::error!("player_table_widget roster_repo save: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let positions = build_player_positions(&roster_def);
    let hired_rows = build_hired_rows(&roster_team, &roster_def);

    PlayerTableWidgetTemplate {
        positions,
        hired_rows,
        app_routes: Default::default(),
        space_id,
        team_id,
        has_roster: true,
    }
    .into_response()
}

// ── Fragment ligne joueur ────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "widgets/player-row-fragment.html")]
pub struct PlayerRowFragment {
    pub row: HiredPlayerRowVm,
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
}

impl IntoResponse for PlayerRowFragment {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Response::builder()
                .header("content-type", "text/html; charset=utf-8")
                .header("HX-Trigger", "teamMutated")
                .body(Body::from(html))
                .unwrap(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Handler hire_player ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct HirePlayerBody {
    pub player_id: String,
}

pub async fn hire_player(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<HirePlayerBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = HirePlayerCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        player_id: PlayerId(body.player_id.clone()),
    };

    let updated_team =
        match hire_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
            Ok(t) => t,
            Err(hire_uc::HirePlayerError::TeamNotFound) => {
                return StatusCode::NOT_FOUND.into_response()
            }
            Err(hire_uc::HirePlayerError::PlayerNotFound) => {
                return StatusCode::UNPROCESSABLE_ENTITY.into_response()
            }
            Err(hire_uc::HirePlayerError::Domain(e)) => {
                return player_error(hire_uc::domain_error_message(&e))
            }
            Err(hire_uc::HirePlayerError::Repository(e)) => {
                tracing::error!("hire_player repo error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let roster_def = match state
        .team_creation
        .reference_data
        .find_roster_definition(&updated_team.roster.id.0)
    {
        Some(d) => d,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let rows = build_hired_rows(&updated_team, &roster_def);
    let row = match rows.into_iter().find(|r| r.uid == body.player_id) {
        Some(r) => r,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    PlayerRowFragment {
        row,
        app_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}

// ── Handler fire_player ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FirePlayerBody {
    pub player_id: String,
}

pub async fn fire_player(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<FirePlayerBody>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = FirePlayerCommand {
        team_id: team_id_val,
        space_id: space_id.clone(),
        player_id: PlayerId(body.player_id.clone()),
    };

    let updated_team =
        match fire_uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
            Ok(t) => t,
            Err(fire_uc::FirePlayerError::TeamNotFound) => {
                return StatusCode::NOT_FOUND.into_response()
            }
            Err(fire_uc::FirePlayerError::PlayerNotFound) => {
                return StatusCode::UNPROCESSABLE_ENTITY.into_response()
            }
            Err(fire_uc::FirePlayerError::Domain(e)) => {
                return player_error(fire_uc::domain_error_message(&e))
            }
            Err(fire_uc::FirePlayerError::Repository(e)) => {
                tracing::error!("fire_player repo error: {e}");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let roster_def = match state
        .team_creation
        .reference_data
        .find_roster_definition(&updated_team.roster.id.0)
    {
        Some(d) => d,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let rows = build_hired_rows(&updated_team, &roster_def);
    let row = match rows.into_iter().find(|r| r.uid == body.player_id) {
        Some(r) => r,
        None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    PlayerRowFragment {
        row,
        app_routes: Default::default(),
        space_id,
        team_id,
    }
    .into_response()
}
