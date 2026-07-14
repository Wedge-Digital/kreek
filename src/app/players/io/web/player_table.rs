use crate::app::players::domain::player::TeamId;
use crate::app::players::ports::{AcquiredSkillProjection, PlayerProjection};
use crate::app::references::domain::port::IReferenceRepository;
use crate::app::routes::AppRoutes;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

// ── View models ───────────────────────────────────────────────────────────────

pub struct SkillTagVm {
    pub name:         String,
    pub category_css: String,
}

impl std::fmt::Display for SkillTagVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub struct PlayerRowVm {
    pub player_id:       String,
    pub jersey:          Option<i16>,
    pub personal_name:   String,
    pub position_name:   String,
    pub base_skills:     Vec<SkillTagVm>,
    pub acquired_skills: Vec<AcquiredSkillProjection>,
    pub spp:             i32,
    pub value_kpo:       i32,
}

fn skill_category_css(category: &str) -> &'static str {
    match category {
        "GENERAL"  => "type-general",
        "STRENGTH" => "type-strength",
        "AGILITY"  => "type-agility",
        "PASSING"  => "type-passing",
        "MUTATION" => "type-mutation",
        _          => "type-general",
    }
}

fn build_base_skills(p: &PlayerProjection, ref_repo: &dyn IReferenceRepository) -> Vec<SkillTagVm> {
    let Some(position) = ref_repo.find_position_by_uid(&p.roster_line_id) else {
        return p.base_skills.iter().map(|n| SkillTagVm {
            name:         n.clone(),
            category_css: "type-general".to_string(),
        }).collect();
    };
    position.skills.iter()
        .filter_map(|uid| ref_repo.find_skill_by_uid(uid))
        .map(|s| SkillTagVm {
            name:         s.name.clone(),
            category_css: skill_category_css(&s.category).to_string(),
        })
        .collect()
}

// ── Template ──────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "player-table-fragment.html")]
pub struct PlayerTableTemplate {
    pub app_routes: AppRoutes,
    pub space_id:   String,
    pub players:    Vec<PlayerRowVm>,
}

impl IntoResponse for PlayerTableTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("player_table template render error: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn player_table_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let projections = state
        .players
        .projection_repository
        .find_by_team_id(&TeamId(team_id))
        .await
        .unwrap_or_default();

    let ref_repo = state.references.repository.as_ref();

    let players = projections.into_iter().map(|p| {
        let base_skills = build_base_skills(&p, ref_repo);
        PlayerRowVm {
            player_id:       p.player_id,
            jersey:          p.jersey,
            personal_name:   p.personal_name,
            position_name:   p.position_name,
            base_skills,
            acquired_skills: p.acquired_skills,
            spp:             p.spp,
            value_kpo:       p.value_kpo,
        }
    }).collect();

    PlayerTableTemplate { app_routes: AppRoutes::default(), space_id, players }.into_response()
}
