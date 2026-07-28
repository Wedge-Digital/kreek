use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::io::web::view_models::FinalizePlayerVm;
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

#[derive(Template)]
#[template(path = "widgets/player-list-widget.html")]
pub struct PlayerListWidgetTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
    pub spp_pool: u8,
    pub players: Vec<FinalizePlayerVm>,
}

impl IntoResponse for PlayerListWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn player_list_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let mut team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("player_list_widget repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    team.assign_missing_jerseys();

    let ref_data = state.team_creation.reference_data.as_ref();

    let players: Vec<FinalizePlayerVm> = team
        .hired_players()
        .iter()
        .map(|p| {
            let base_skills = ref_data.resolve_base_skills(&p.definition.id.0);
            let acquired_csv: String = p
                .acquired_skills
                .iter()
                .map(|a| a.skill_id.0.as_str())
                .collect::<Vec<_>>()
                .join(",");

            FinalizePlayerVm {
                id: p.instance_id.0.clone(),
                jersey: p.jersey.map(|j| j.into_inner()).unwrap_or(0),
                name: if p.personal_name.is_empty() {
                    p.definition.name.as_ref().to_string()
                } else {
                    p.personal_name.clone()
                },
                position_name: p.definition.name.as_ref().to_string(),
                roster_line_id: p.definition.id.0.clone(),
                base_skills,
                acquired_count: p.acquired_skills.len(),
                acquired_csv,
            }
        })
        .collect();

    PlayerListWidgetTemplate {
        app_routes: Default::default(),
        space_id,
        team_id,
        spp_pool: team.spp_pool.0,
        players,
    }
    .into_response()
}
