use crate::app::auth::auth_backend::AuthSession;
use crate::app::team_creation::domain::roster::PlayerId;
use crate::app::team_creation::domain::team_roster_selected::RosterSelectedTeam;
use crate::app::team_creation::routes::Routes;
use crate::app::team_creation::use_cases::set_player_identity as uc;
use crate::app::team_creation::use_cases::set_player_identity::SetPlayerIdentityCommand;
use crate::app::shared_kernel::common_types::EntityId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SetPlayerIdentityBody {
    pub name: Option<String>,
    pub jersey: u8,
}

// ── View models ───────────────────────────────────────────────────────────────

pub struct PlayerRowVm {
    pub instance_id: String,
    pub position_name: String,
    pub personal_name: String,
    pub jersey: u8,
    pub skills: String,
    pub skills_acquired_count: usize,
    pub set_identity_url: String,
}

pub struct SkillHeaderPlayerVm {
    pub instance_id: String,
    pub personal_name: String,
    pub jersey: u8,
    pub position_name: String,
    pub roster_name: String,
    pub existing_skills: Vec<String>,
    pub spp_pool: u8,
    pub set_identity_url: String,
}

// ── Templates ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "finalize-player-row-fragment.html")]
pub struct PlayerRowFragmentTemplate {
    pub player: PlayerRowVm,
}

impl IntoResponse for PlayerRowFragmentTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Template)]
#[template(path = "finalize-skill-header-fragment.html")]
pub struct SkillHeaderFragmentTemplate {
    pub player: Option<SkillHeaderPlayerVm>,
    pub spp_pool: u8,
}

impl IntoResponse for SkillHeaderFragmentTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

// ── Builder helpers ───────────────────────────────────────────────────────────

pub fn build_player_row_vm(
    team: &RosterSelectedTeam,
    instance_id: &str,
    routes: &Routes,
    space_id: &str,
    team_id: &str,
) -> Option<PlayerRowVm> {
    let player = team
        .hired_players()
        .iter()
        .find(|p| p.instance_id.0 == instance_id)?;

    Some(PlayerRowVm {
        instance_id: player.instance_id.0.clone(),
        position_name: player.definition.name.0.clone(),
        personal_name: player.personal_name.clone(),
        jersey: player.jersey.unwrap_or(0),
        skills: String::new(),
        skills_acquired_count: 0,
        set_identity_url: routes.set_player_identity(space_id, team_id, &player.instance_id.0),
    })
}

pub fn build_skill_header_vm(
    team: &RosterSelectedTeam,
    instance_id: &str,
    routes: &Routes,
    space_id: &str,
    team_id: &str,
    spp_pool: u8,
) -> Option<SkillHeaderPlayerVm> {
    let player = team
        .hired_players()
        .iter()
        .find(|p| p.instance_id.0 == instance_id)?;

    Some(SkillHeaderPlayerVm {
        instance_id: player.instance_id.0.clone(),
        personal_name: player.personal_name.clone(),
        jersey: player.jersey.unwrap_or(0),
        position_name: player.definition.name.0.clone(),
        roster_name: team.roster.name.0.clone(),
        existing_skills: vec![],
        spp_pool,
        set_identity_url: routes.set_player_identity(space_id, team_id, &player.instance_id.0),
    })
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn set_player_identity(
    _auth_session: AuthSession,
    Path((space_id, team_id, instance_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
    axum::Json(body): axum::Json<SetPlayerIdentityBody>,
) -> impl IntoResponse {
    let team_entity_id = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let cmd = SetPlayerIdentityCommand {
        team_id: team_entity_id,
        space_id: space_id.clone(),
        instance_id: PlayerId(instance_id.clone()),
        name: body.name.unwrap_or_default().trim().to_string(),
        jersey: body.jersey,
    };

    match uc::execute(cmd, state.team_creation.roster_repository.as_ref()).await {
        Err(uc::SetPlayerIdentityError::TeamNotFound) => {
            StatusCode::NOT_FOUND.into_response()
        }
        Err(uc::SetPlayerIdentityError::Domain(e)) => {
            axum::http::Response::builder()
                .status(422)
                .header("Content-Type", "text/html")
                .body(axum::body::Body::from(format!(
                    r#"<div class="identity-error">{}</div>"#,
                    e
                )))
                .unwrap()
                .into_response()
        }
        Err(uc::SetPlayerIdentityError::Repository(_)) => {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Ok(()) => {
            let team = match state
                .team_creation
                .roster_repository
                .find_by_id(&EntityId::try_new(&team_id).unwrap())
                .await
            {
                Ok(Some(t)) => t,
                _ => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };

            let routes = Routes::default();
            let spp_pool: u8 = 0; // sera calculé depuis les règles de création (carte 58)

            let row_vm =
                build_player_row_vm(&team, &instance_id, &routes, &space_id, &team_id);
            let header_vm =
                build_skill_header_vm(&team, &instance_id, &routes, &space_id, &team_id, spp_pool);

            let row_html = row_vm
                .map(|vm| {
                    PlayerRowFragmentTemplate { player: vm }
                        .render()
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            let header_html = header_vm
                .map(|vm| {
                    SkillHeaderFragmentTemplate {
                        player: Some(vm),
                        spp_pool,
                    }
                    .render()
                    .unwrap_or_default()
                })
                .unwrap_or_else(|| {
                    SkillHeaderFragmentTemplate {
                        player: None,
                        spp_pool,
                    }
                    .render()
                    .unwrap_or_default()
                });

            Html(format!("{}{}", row_html, header_html)).into_response()
        }
    }
}
