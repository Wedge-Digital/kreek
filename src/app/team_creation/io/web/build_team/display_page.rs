use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::identity::ids::EntityId;
use crate::app::team_creation::io::web::view_models::RulesTierVm;
use crate::state::AppState;

pub struct RulesPanelVm {
    pub competition_name: String,
    pub season_name: String,
    pub tiers: Vec<RulesTierVm>,
}

#[derive(Template)]
#[template(path = "build-team.html")]
pub struct BuildTeamTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,
    pub selected_roster_uid: Option<String>,
    pub rules_panel: RulesPanelVm,
}

impl IntoResponse for BuildTeamTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

pub async fn build_team(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
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
            tracing::error!("build_team draft find {team_id}: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let selected_roster_uid = match state
        .team_creation
        .roster_repository
        .find_by_id(&team_id_val)
        .await
    {
        Ok(Some(team)) => Some(team.roster.id.0.clone()),
        _ => None,
    };

    let competition_name = if let Ok(id) = EntityId::try_new(draft.competition_id()) {
        state
            .competitions
            .competition_repository
            .find_base_info(&id)
            .await
            .ok()
            .flatten()
            .map(|i| i.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let season_name = if let Ok(id) = EntityId::try_new(draft.season_id()) {
        state
            .competitions
            .season_repository
            .find_base_info(&id)
            .await
            .ok()
            .flatten()
            .map(|i| i.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let rules_panel = RulesPanelVm {
        competition_name,
        season_name,
        tiers: draft
            .creation_rules()
            .tiers
            .iter()
            .map(|t| RulesTierVm {
                name: t.name.clone().into_inner(),
                budget: t.budget.0,
                start_xp: t.start_xp.into_inner(),
            })
            .collect(),
    };

    BuildTeamTemplate {
        app_routes: Default::default(),
        space_id,
        team_id,
        selected_roster_uid,
        rules_panel,
    }
        .into_response()
}