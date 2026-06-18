use crate::app::shared_kernel::common_types::EntityId;
use crate::app::team_creation::routes::Routes as TeamCreationRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SkillHeaderParams {
    #[serde(default)]
    pub player_id: String,
}

pub struct AcquiredSkillVm {
    pub skill_id: String,
    pub name: String,
    pub mode_label: String,
    pub spp_cost: u8,
}

pub struct SkillHeaderVm {
    pub player_id: String,
    pub jersey: u8,
    pub name: String,
    pub position_name: String,
    pub roster_name: String,
    pub spp_pool: u8,
    pub acquired_skills: Vec<AcquiredSkillVm>,
}

#[derive(Template)]
#[template(path = "widgets/skill-header-widget.html")]
pub struct SkillHeaderWidgetTemplate {
    pub team_routes: TeamCreationRoutes,
    pub space_id: String,
    pub team_id: String,
    pub player: Option<SkillHeaderVm>,
}

impl IntoResponse for SkillHeaderWidgetTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn skill_header_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    Query(params): Query<SkillHeaderParams>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if params.player_id.is_empty() {
        return SkillHeaderWidgetTemplate {
            team_routes: Default::default(),
            space_id,
            team_id,
            player: None,
        }
        .into_response();
    }

    let team_id_val = match EntityId::try_new(&team_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let team = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(t)) => t,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("skill_header_widget repo error: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let ref_data = state.team_creation.reference_data.as_ref();

    let player_vm = team
        .hired_players()
        .iter()
        .find(|p| p.instance_id.0 == params.player_id)
        .map(|p| {
            let acquired_skills = p
                .acquired_skills
                .iter()
                .map(|a| {
                    let name = ref_data
                        .resolve_skill_name(&a.skill_id.0)
                        .unwrap_or_else(|| a.skill_id.0.clone());

                    AcquiredSkillVm {
                        skill_id: a.skill_id.0.clone(),
                        name,
                        mode_label: match a.mode {
                            crate::app::team_creation::domain::roster::AcquisitionMode::Chosen => {
                                "Choisie".into()
                            }
                            crate::app::team_creation::domain::roster::AcquisitionMode::Random => {
                                "Aléatoire".into()
                            }
                        },
                        spp_cost: a.spp_cost,
                    }
                })
                .collect();

            SkillHeaderVm {
                player_id: p.instance_id.0.clone(),
                jersey: p.jersey.map(|j| j.0).unwrap_or(0),
                name: if p.personal_name.is_empty() {
                    p.definition.name.0.clone()
                } else {
                    p.personal_name.clone()
                },
                position_name: p.definition.name.0.clone(),
                roster_name: team.roster.name.0.clone(),
                spp_pool: team.spp_pool,
                acquired_skills,
            }
        });

    SkillHeaderWidgetTemplate {
        team_routes: Default::default(),
        space_id,
        team_id,
        player: player_vm,
    }
    .into_response()
}
