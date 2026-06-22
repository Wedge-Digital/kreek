use crate::app::routes::AppRoutes;
use crate::app::shared_kernel::common_types::SeasonId;
use crate::state::AppState;
use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use std::collections::HashSet;

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

// ── Unassigned pool ──────────────────────────────────────────────────────────

pub struct UnassignedTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
}

#[derive(Template)]
#[template(path = "admin/widgets/unassigned-pool.html")]
pub struct UnassignedPoolTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub teams: Vec<UnassignedTeamVm>,
}

impl IntoResponse for UnassignedPoolTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("unassigned pool render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn unassigned_pool_widget(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let enrolled = match state.competitions.team_info_port.find_enrolled_teams(&season_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("unassigned_pool enrolled: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let groups = match state.competitions.group_repository.find_groups(&season_id).await {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("unassigned_pool groups: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let assigned: HashSet<String> = groups
        .iter()
        .flat_map(|g| g.team_ids.iter().cloned())
        .collect();

    let teams = enrolled
        .into_iter()
        .filter(|t| !assigned.contains(&t.team_id))
        .map(|t| UnassignedTeamVm {
            team_initials: initials(&t.team_name),
            team_id: t.team_id,
            team_name: t.team_name,
        })
        .collect();

    UnassignedPoolTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        teams,
    }
    .into_response()
}

// ── Group cards ──────────────────────────────────────────────────────────────

pub struct GroupTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub team_initials: String,
    pub coach_name: String,
    pub roster_name: String,
}

pub struct GroupVm {
    pub group_id: String,
    pub name: String,
    pub teams: Vec<GroupTeamVm>,
}

#[derive(Template)]
#[template(path = "admin/widgets/group-cards.html")]
pub struct GroupCardsTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub competition_id: String,
    pub season_id: String,
    pub groups: Vec<GroupVm>,
}

impl IntoResponse for GroupCardsTemplate {
    fn into_response(self) -> Response {
        match self.render() {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("group cards render: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

pub async fn group_cards_widget(
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sid = match SeasonId::try_new(&season_id) {
        Ok(id) => id,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let structure = state
        .competitions
        .season_repository
        .find_structure(&sid)
        .await
        .ok()
        .flatten();

    if let Some(ref s) = structure {
        let struct_groups: Vec<(String, String)> = s
            .ranking_group
            .ranking_groups
            .iter()
            .map(|g| (g.id.clone(), g.name.clone()))
            .collect();
        if !struct_groups.is_empty() {
            if let Err(e) = state
                .competitions
                .group_repository
                .ensure_groups_from_structure(&season_id, &struct_groups)
                .await
            {
                tracing::error!("ensure_groups: {e}");
            }
        }
    }

    let enrolled = match state.competitions.team_info_port.find_enrolled_teams(&season_id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("group_cards enrolled: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let groups_data = match state.competitions.group_repository.find_groups(&season_id).await {
        Ok(g) => g,
        Err(e) => {
            tracing::error!("group_cards groups: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let team_map: std::collections::HashMap<&str, &crate::app::competitions::ports::TeamInfoDto> =
        enrolled.iter().map(|t| (t.team_id.as_str(), t)).collect();

    let groups = groups_data
        .into_iter()
        .map(|g| {
            let teams = g
                .team_ids
                .iter()
                .filter_map(|tid| {
                    team_map.get(tid.as_str()).map(|t| GroupTeamVm {
                        team_id: t.team_id.clone(),
                        team_name: t.team_name.clone(),
                        team_initials: initials(&t.team_name),
                        coach_name: t.coach_name.clone(),
                        roster_name: t.roster_name.clone(),
                    })
                })
                .collect();
            GroupVm {
                group_id: g.group_id,
                name: g.group_name,
                teams,
            }
        })
        .collect();

    GroupCardsTemplate {
        app_routes: AppRoutes::default(),
        space_id,
        competition_id,
        season_id,
        groups,
    }
    .into_response()
}
